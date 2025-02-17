
use std::sync::Arc;
use std::sync::mpsc;
use std::collections::HashMap;
use serde_derive::{Serialize, Deserialize};
use std::cell::RefCell;
use std::rc::Rc;

use crate::configuration::Hosts;
use crate::enums::Criticality;
use crate::file_handler;
use crate::host_manager::HostManager;
use crate::module::command::UIAction;
use crate::module::module_factory::ModuleFactory;
use crate::utils::{ShellCommand, ErrorMessage};
use crate::{
    configuration::Preferences,
    Host,
    host_manager::StateUpdateMessage,
    frontend::DisplayOptions,
    connection_manager::*, 
};

use crate::module::{
    command::Command,
    command::CommandResult,
};

// Default needs to be implemented because of Qt QObject requirements.
#[derive(Default)]
pub struct CommandHandler {
    /// Host name is the first key, command id is the second key.
    commands: HashMap<String, HashMap<String, Command>>,
    /// For communication to ConnectionManager.
    request_sender: Option<mpsc::Sender<ConnectorRequest>>,
    /// Channel to send state updates to HostManager.
    state_update_sender: Option<mpsc::Sender<StateUpdateMessage>>,
    /// Preferences from config file.
    preferences: Preferences,
    /// Effective host configurations.
    hosts_config: Hosts,
    /// Every execution gets an invocation ID. Valid ID numbers begin from 1.
    invocation_id_counter: u64,

    // Shared resources.
    /// Mainly for getting up-to-date Host-datas.
    host_manager: Rc<RefCell<HostManager>>,
    module_factory: Arc<ModuleFactory>,
}

impl CommandHandler {
    pub fn new(host_manager: Rc<RefCell<HostManager>>, module_factory: Arc<ModuleFactory>) -> Self {
        CommandHandler {
            commands: HashMap::new(),
            request_sender: None,
            state_update_sender: None,
            preferences: Preferences::default(),
            hosts_config: Hosts::default(),
            invocation_id_counter: 0,

            host_manager: host_manager.clone(),
            module_factory: module_factory,
        }
    }

    pub fn configure(&mut self,
                     hosts_config: &Hosts,
                     preferences: &Preferences,
                     request_sender: mpsc::Sender<ConnectorRequest>,
                     state_update_sender: mpsc::Sender<StateUpdateMessage>) {

        self.commands.clear();
        self.request_sender = Some(request_sender);
        self.state_update_sender = Some(state_update_sender);

        self.preferences = preferences.clone();
        self.hosts_config = hosts_config.clone();

        for (host_id, host_config) in hosts_config.hosts.iter() {
            for (command_id, command_config) in host_config.commands.iter() {
                let command_spec = crate::module::ModuleSpecification::new(command_id, &command_config.version);
                let command = self.module_factory.new_command(&command_spec, &command_config.settings);
                self.add_command(host_id, command);
            }
        }
    }

    fn add_command(&mut self, host_id: &String, command: Command) {
        self.commands.entry(host_id.clone()).or_insert(HashMap::new());

        let command_collection = self.commands.get_mut(host_id).unwrap();
        let module_spec = command.get_module_spec();

        // Only add if missing.
        command_collection.entry(module_spec.id).or_insert(command);
    }

    /// Returns invocation ID or 0 on error.
    pub fn execute(&mut self, host_id: &String, command_id: &String, parameters: &Vec<String>) -> u64 {

        let host = self.host_manager.borrow().get_host(host_id);

        if !host.platform.is_set() {
            log::warn!("[{}] Executing command \"{}\" despite missing platform info", host_id, command_id);
        }

        let command = self.commands.get(host_id).unwrap()
                                   .get(command_id).unwrap();
        let state_update_sender = self.state_update_sender.as_ref().unwrap().clone();

        let messages = match get_command_connector_messages(&host, command, parameters) {
            Ok(messages) => messages,
            Err(error) => {
                log::error!("Command \"{}\" failed: {}", command_id, error);
                state_update_sender.send(StateUpdateMessage {
                    host_name: host.name,
                    display_options: command.get_display_options(),
                    module_spec: command.get_module_spec(),
                    command_result: Some(CommandResult::new_error(error)),
                    ..Default::default()
                }).unwrap_or_else(|error| {
                    log::error!("Couldn't send message to state manager: {}", error);
                });
                return 0;
            }
        };

        self.invocation_id_counter += 1;

        self.request_sender.as_ref().unwrap().send(ConnectorRequest {
            connector_spec: command.get_connector_spec(),
            source_id: command.get_module_spec().id,
            host: host.clone(),
            request_type: RequestType::Command,
            messages: messages,
            response_handler: Self::get_response_handler(
                host,
                command.box_clone(),
                self.invocation_id_counter,
                state_update_sender
            ),
            cache_policy: CachePolicy::BypassCache, 
        }).unwrap_or_else(|error| {
            log::error!("Couldn't send message to connector: {}", error);
        });

        self.invocation_id_counter
    }

    // Return value contains host's commands. `parameters` is not set since provided by data point later on.
    pub fn get_commands_for_host(&self, host_id: String) -> HashMap<String, CommandData> {
        if let Some(command_collection) = self.commands.get(&host_id) {
            command_collection.iter().map(|(command_id, command)| {
                (command_id.clone(), CommandData::new(command_id.clone(), command.get_display_options()))
            }).collect()
        }
        else {
            HashMap::new()
        }
    }

    pub fn get_command_for_host(&self, host_id: &String, command_id: &String) -> CommandData {
        let command_collection = self.commands.get(host_id).unwrap_or_else(|| panic!("Host {} not found", host_id));
        let command = command_collection.get(command_id).unwrap_or_else(|| panic!("Command {} not found", command_id));
        CommandData::new(command_id.clone(), command.get_display_options())
    }

    fn get_response_handler(host: Host, command: Command, invocation_id: u64, state_update_sender: mpsc::Sender<StateUpdateMessage>) -> ResponseHandlerCallback {
        Box::new(move |results| {
            let (responses, errors): (Vec<_>, Vec<_>) =  results.into_iter().partition(Result::is_ok);
            let responses = responses.into_iter().map(Result::unwrap).collect::<Vec<_>>();
            let mut errors = errors.into_iter().map(|error| ErrorMessage::new(Criticality::Error, error.unwrap_err())).collect::<Vec<_>>();
            let command_id = command.get_module_spec().id.clone();

            let mut result;
            if !responses.is_empty() {
                result = command.process_responses(host.clone(), responses.clone());
                if let Err(error) = result {
                    if error.is_empty() {
                        // Wasn't implemented, try the other method.
                        result = command.process_response(host.clone(), responses.first().unwrap());
                    }
                    else {
                        result = Err(error);
                    }
                }
            }
            else {
                result = Err(format!("No responses received for command {}", command_id));
            }

            let new_command_result = match result {
                Ok(mut command_result) => {
                    let log_message = if command_result.message.len() > 5000 {
                        format!("{}...(long message cut)...", &command_result.message[..5000])
                    }
                    else {
                        command_result.message.clone()
                    };

                    log::debug!("[{}] Command result received: {}", host.name, log_message);
                    command_result.invocation_id = invocation_id;
                    command_result.command_id = command.get_module_spec().id;
                    Some(command_result)
                },
                Err(error) => {
                    errors.push(ErrorMessage::new(Criticality::Error, error));
                    None
                }
            };

            for error in errors.iter() {
                log::error!("[{}] Error from command {}: {}", host.name, command_id, error.message);
            }

            state_update_sender.send(StateUpdateMessage {
                host_name: host.name,
                display_options: command.get_display_options(),
                module_spec: command.get_module_spec(),
                command_result: new_command_result,
                errors: errors,
                ..Default::default()
            }).unwrap_or_else(|error| {
                log::error!("Couldn't send message to state manager: {}", error);
            });
        })

    }

    //
    // INTEGRATED COMMANDS
    //

    pub fn download_file(&mut self, host_id: &String, command_id: &String, remote_file_path: &String) -> (u64, String) {
        let host = self.host_manager.borrow().get_host(&host_id);
        let command = self.commands.get(host_id).unwrap()
                                   .get(command_id).unwrap();

        let connector_messages = get_command_connector_messages(&host, command, &[remote_file_path.clone()]).map_err(|error| {
            log::error!("Command \"{}\" failed: {}", command_id, error);
            return;
        }).unwrap();

        let (_, local_file_path) = file_handler::convert_to_local_paths(&host, remote_file_path);
        // Whether to load file contents to the command result.
        let load_contents = command.get_display_options().action == UIAction::TextEditor;
        self.invocation_id_counter += 1;

        self.request_sender.as_ref().unwrap().send(ConnectorRequest {
            connector_spec: command.get_connector_spec(),
            source_id: command.get_module_spec().id,
            host: host.clone(),
            request_type: RequestType::Download,
            messages: connector_messages,
            response_handler: Self::get_response_handler_download_file(
                host,
                command.box_clone(),
                self.invocation_id_counter,
                load_contents,
                self.state_update_sender.as_ref().unwrap().clone()
            ),
            cache_policy: CachePolicy::BypassCache,
        }).unwrap();

        (self.invocation_id_counter, local_file_path)
    }

    pub fn save_and_upload_file(&mut self, host_id: &String, command_id: &String, local_file_path: &String, new_contents: Vec<u8>) -> u64 {
        let host = self.host_manager.borrow().get_host(&host_id);
        let command = self.commands.get(host_id).unwrap()
                                   .get(command_id).unwrap();
        let state_update_sender = self.state_update_sender.as_ref().unwrap().clone();

        file_handler::update_file(local_file_path, new_contents).unwrap();
        self.invocation_id_counter += 1;

        self.request_sender.as_ref().unwrap().send(ConnectorRequest {
            connector_spec: command.get_connector_spec(),
            source_id: command.get_module_spec().id,
            host: host.clone(),
            request_type: RequestType::Upload,
            messages: vec![local_file_path.clone()],
            response_handler: Self::get_response_handler_upload_file(
                host,
                command.box_clone(),
                self.invocation_id_counter,
                state_update_sender
            ),
            cache_policy: CachePolicy::BypassCache,
        }).unwrap();

        self.invocation_id_counter
    }

    fn remote_ssh_command(&self, host_id: &String) -> ShellCommand {
        let host = self.host_manager.borrow().get_host(&host_id);

        let ssh_settings = self.hosts_config.hosts[host_id]
                                            .connectors["ssh"]
                                            .settings.clone();

        let remote_address = if !host.fqdn.is_empty() {
            host.fqdn.clone()
        }
        else {
            host.ip_address.to_string()
        };

        let mut command = ShellCommand::new();
        command.arguments(vec![
            String::from("ssh"),
            String::from("-t"),
            String::from("-p"), ssh_settings.get("port").unwrap_or(&String::from("22")).clone(),
            remote_address,
        ]);

        if let Some(username) = ssh_settings.get("username") {
            command.arguments(vec![String::from("-l"), username.clone()]);
        }

        command
    }

    pub fn open_remote_terminal_command(&self, host_id: &String, command_id: &String, parameters: &Vec<String>) -> ShellCommand {
        let host = self.host_manager.borrow().get_host(&host_id);
        let mut command = self.remote_ssh_command(&host_id);

        let command_module = self.commands.get(host_id).unwrap()
                                          .get(command_id).unwrap();

        let connector_messages = get_command_connector_messages(&host, command_module, &parameters).map_err(|error| {
            log::error!("Command \"{}\" failed: {}", command_id, error);
            return;
        }).unwrap();

        command.arguments(connector_messages);
        command
    }

    // TODO: this will block the UI thread? Improve!
    pub fn open_external_terminal(&self, host_id: &String, command_id: &String, parameters: Vec<String>) {
        let command_args = self.open_remote_terminal_command(&host_id, &command_id, &parameters);

        log::debug!("Starting local process: {} {}", self.preferences.terminal, command_args.to_string());
        let result = ShellCommand::new()
            .arguments(vec![self.preferences.terminal.clone()])
            .arguments(self.preferences.terminal_args.clone())
            .arguments(command_args.to_vec())
            .execute();

        if let Err(error) = result {
            log::error!("Couldn't start terminal: {}", error);
        }
    }

    // TODO: this will block the UI thread? Improve!
    pub fn open_remote_text_editor_command(&self, host_id: &String, remote_file_path: &String) -> ShellCommand {
        let mut command = self.remote_ssh_command(&host_id);

        if self.preferences.sudo_remote_editor {
            command.argument("sudo");
        }

        command.argument(self.preferences.remote_text_editor.clone());
        command.argument(remote_file_path.clone());
        command
    }

    pub fn open_external_text_editor(&self, host_id: &String, command_id: &String, remote_file_path: &String) {
        let host = self.host_manager.borrow().get_host(&host_id);
        let command = self.commands.get(host_id).unwrap()
                                   .get(command_id).unwrap();

        let connector_messages = get_command_connector_messages(&host, command, &[remote_file_path.clone()]).map_err(|error| {
            log::error!("Command \"{}\" failed: {}", command_id, error);
            return;
        }).unwrap();

        self.request_sender.as_ref().unwrap().send(ConnectorRequest {
            connector_spec: command.get_connector_spec(),
            source_id: command.get_module_spec().id,
            host: host.clone(),
            request_type: RequestType::Download,
            messages: connector_messages,
            response_handler: Self::get_response_handler_external_text_editor(
                host,
                command.box_clone(),
                self.preferences.clone(),
                self.request_sender.as_ref().unwrap().clone(),
                self.state_update_sender.as_ref().unwrap().clone()
            ),
            cache_policy: CachePolicy::BypassCache,
        }).unwrap_or_else(|error| {
            log::error!("Couldn't send message to connector: {}", error);
        });
    }

    fn get_response_handler_download_file(host: Host, command: Command, invocation_id: u64, load_contents: bool,
                                          state_update_sender: mpsc::Sender<StateUpdateMessage>) -> ResponseHandlerCallback { 
        Box::new(move |responses| {
            // TODO: Commands don't yet support multiple commands per module. Implement later (take a look at monitor_manager.rs).
            let response = responses.first().unwrap();

            match response {
                Ok(response_message) => {
                    let local_file_path = response_message.message.clone();

                    let command_result  = if load_contents {
                        let (_, contents) = file_handler::read_file(&local_file_path).unwrap();
                        CommandResult::new_hidden(String::from_utf8(contents).unwrap())
                                      .with_invocation_id(invocation_id)
                    }
                    else {
                        CommandResult::new_hidden(local_file_path)
                                      .with_invocation_id(invocation_id)
                    };

                    state_update_sender.send(StateUpdateMessage {
                        host_name: host.name,
                        display_options: command.get_display_options(),
                        module_spec: command.get_module_spec(),
                        command_result: Some(command_result),
                        ..Default::default()
                    }).unwrap_or_else(|error| {
                        log::error!("Couldn't send message to state manager: {}", error);
                    });
                },
                Err(error) => {
                    let error_message = format!("Error downloading file: {}", error);
                    log::error!("{}", error_message);

                    state_update_sender.send(StateUpdateMessage {
                        host_name: host.name,
                        display_options: command.get_display_options(),
                        module_spec: command.get_module_spec(),
                        command_result: Some(CommandResult::new_critical_error(error_message)),
                        ..Default::default()
                    }).unwrap_or_else(|error| {
                        log::error!("Couldn't send message to state manager: {}", error);
                    });
                }
            }
        })
    }

    fn get_response_handler_upload_file(host: Host, command: Command, invocation_id: u64,
                                        state_update_sender: mpsc::Sender<StateUpdateMessage>) -> ResponseHandlerCallback {

        Box::new(move |responses| {
            // TODO: Commands don't yet support multiple commands per module. Implement later (take a look at monitor_manager.rs).
            // TODO: check that destination file hasn't changed?
            let response = responses.first().unwrap();

            let command_result = match response {
                Ok(message) => {
                    CommandResult::new_info(message.message.to_owned())
                                  .with_invocation_id(invocation_id)
                },
                Err(error) => {
                    let error_message = format!("Error uploading file: {}", error);
                    log::error!("{}", error_message);
                    CommandResult::new_critical_error(error_message)
                                  .with_invocation_id(invocation_id)
                }
            };

            state_update_sender.send(StateUpdateMessage {
                host_name: host.name,
                display_options: command.get_display_options(),
                module_spec: command.get_module_spec(),
                command_result: Some(command_result),
                ..Default::default()
            }).unwrap();
        })
    }

    fn get_response_handler_external_text_editor(host: Host, command: Command,
                                                 preferences: Preferences,
                                                 request_sender: mpsc::Sender<ConnectorRequest>,
                                                 state_update_sender: mpsc::Sender<StateUpdateMessage>) -> ResponseHandlerCallback {

        Box::new(move |responses| {
            // TODO: Commands don't yet support multiple commands per module. Implement later (take a look at monitor_manager.rs).
            let response = responses.first().unwrap();

            match response {
                Ok(response_message) => {
                    let local_file = response_message.message.clone();
                    log::debug!("Starting local process: {} {}", preferences.text_editor, local_file);
                    let result = ShellCommand::new()
                        .arguments(vec![preferences.text_editor, local_file])
                        .execute();

                    if let Err(error) = result {
                        log::error!("Couldn't start text editor: {}", error);
                        return;
                    }

                    request_sender.send(ConnectorRequest {
                        connector_spec: command.get_connector_spec(),
                        source_id: command.get_module_spec().id,
                        host: host.clone(),
                        request_type: RequestType::Upload,
                        messages: vec![response_message.message.clone()],
                        response_handler: Self::get_response_handler_upload_file(host, command, 0, state_update_sender),
                        cache_policy: CachePolicy::BypassCache,
                    }).unwrap_or_else(|error| {
                        log::error!("Couldn't send message to connector: {}", error);
                    });
                },
                Err(error) => {
                    let error_message = format!("Error downloading file: {}", error);
                    log::error!("{}", error_message);

                    state_update_sender.send(StateUpdateMessage {
                        host_name: host.name,
                        display_options: command.get_display_options(),
                        module_spec: command.get_module_spec(),
                        command_result: Some(CommandResult::new_critical_error(error_message)),
                        ..Default::default()
                    }).unwrap_or_else(|error| {
                        log::error!("Couldn't send message to state manager: {}", error);
                    });
                }
            };
        })
    }
}

fn get_command_connector_messages(host: &Host, command: &Command, parameters: &[String]) -> Result<Vec<String>, String> {
    let mut all_messages: Vec<String> = Vec::new();

    match command.get_connector_messages(host.clone(), parameters.to_owned()) {
        Ok(messages) => all_messages.extend(messages),
        Err(error) => {
            if !error.is_empty() {
                return Err(error);
            }
        }
    }

    match command.get_connector_message(host.clone(), parameters.to_owned()) {
        Ok(message) => all_messages.push(message),
        Err(error) => {
            if !error.is_empty() {
                return Err(error);
            }
        }
    }

    all_messages.retain(|message| !message.is_empty());
    Ok(all_messages)
}


#[derive(Default, Clone, Serialize, Deserialize)]
pub struct CommandData {
    pub command_id: String,
    pub command_params: Vec<String>,
    pub display_options: DisplayOptions,
}

impl CommandData {
    pub fn new(command_id: String, display_options: DisplayOptions) -> Self {
        CommandData {
            command_id: command_id,
            command_params: Vec::new(),
            display_options: display_options,
        }
    }
}

