use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;
use std::sync::mpsc::{self, Sender};

use crate::Host;
use crate::module::connection::ResponseMessage;
use crate::module::monitoring::*;
use crate::host_manager::{StateUpdateMessage, HostManager};
use crate::connection_manager::{ ConnectorRequest, ResponseHandlerCallback, RequestType };


pub struct MonitorManager {
    // Host name is the first key, monitor id is the second key.
    monitors: HashMap<String, HashMap<String, Monitor>>,
    request_sender: Sender<ConnectorRequest>,
    // Channel to send state updates to HostManager.
    state_update_sender: Sender<StateUpdateMessage>,
    host_manager: Rc<RefCell<HostManager>>,
    /// Every refresh operation gets an invocation ID. Valid ID numbers begin from 1.
    invocation_id_counter: u64,
}

impl MonitorManager {
    pub fn new(request_sender: mpsc::Sender<ConnectorRequest>, host_manager: Rc<RefCell<HostManager>>) -> Self {
        MonitorManager {
            monitors: HashMap::new(),
            request_sender: request_sender,
            host_manager: host_manager.clone(),
            state_update_sender: host_manager.borrow().new_state_update_sender(),
            invocation_id_counter: 0,
        }
    }

    // Adds a monitor but only if a monitor with the same ID doesn't exist.
    pub fn add_monitor(&mut self, host: &Host, monitor: Monitor) {
        self.monitors.entry(host.name.clone()).or_insert(HashMap::new());

        let monitor_collection = self.monitors.get_mut(&host.name).unwrap();
        let module_spec = monitor.get_module_spec();

        // Only add if missing.
        if !monitor_collection.contains_key(&module_spec.id) {
            log::debug!("[{}] Adding monitor {}", host.name, module_spec.id);

            // Independent monitors are always executed first.
            // They don't depend on platform info or connectors.
            if monitor.get_connector_spec().is_none() {
                self.invocation_id_counter += 1;

                self.request_sender.send(ConnectorRequest {
                    connector_spec: None,
                    source_id: monitor.get_module_spec().id,
                    host: host.clone(),
                    messages: Vec::new(),
                    request_type: RequestType::Command,
                    response_handler: Self::get_response_handler(
                        host.clone(), vec![monitor.box_clone()], self.invocation_id_counter,
                        self.request_sender.clone(), self.state_update_sender.clone(), DataPoint::empty_and_critical()
                    )
                }).unwrap_or_else(|error| {
                    log::error!("Couldn't send message to connector: {}", error);
                });
            }

            // Add initial state value indicating no data as been received yet.
            Self::send_state_update(&host, &monitor, self.state_update_sender.clone(), DataPoint::no_data());

            monitor_collection.insert(module_spec.id, monitor);
        }
    }

    pub fn refresh_platform_info(&mut self, host_id: Option<&String>) {
        for (host_name, monitor_collection) in self.monitors.iter() {
            if let Some(host_filter) = host_id {
                if host_name != host_filter {
                    continue;
                }
            }

            let host = self.host_manager.borrow().get_host(host_name);
            log::debug!("[{}] Refreshing platform info", host_name);

            // Executed only if required connector is available.
            if monitor_collection.iter().any(|(_, monitor)| monitor.get_connector_spec().unwrap_or_default().id == "ssh") {
                self.invocation_id_counter += 1;

                // TODO: remove hardcoding and execute once per connector type.
                let info_provider = internal::PlatformInfoSsh::new_monitoring_module(&HashMap::new());
                self.request_sender.send(ConnectorRequest {
                    connector_spec: info_provider.get_connector_spec(),
                    source_id: info_provider.get_module_spec().id,
                    host: host.clone(),
                    messages: vec![info_provider.get_connector_message(host.clone(), DataPoint::empty())],
                    request_type: RequestType::Command,
                    response_handler: Self::get_response_handler(
                        host.clone(), vec![info_provider], self.invocation_id_counter,
                        self.request_sender.clone(), self.state_update_sender.clone(), DataPoint::empty_and_critical()
                    )
                }).unwrap_or_else(|error| {
                    log::error!("Couldn't send message to connector: {}", error);
                });
            }
        }
    }

    /// Use `None` to refresh all monitors on every host or limit by host.
    /// Returns the invocation IDs of the refresh operations.
    pub fn refresh_monitors_of_category(&mut self, host_id: &String, category: &String) -> Vec<u64> {
        let host = self.host_manager.borrow().get_host(host_id);
        let monitors_by_category = self.monitors.get(host_id).unwrap().iter()
                                                .filter(|(_, monitor)| &monitor.get_display_options().category == category)
                                                .collect();

        let invocation_ids = self.refresh_monitors(host, monitors_by_category);
        self.invocation_id_counter = invocation_ids.last().unwrap().clone();
        invocation_ids
    }

    /// Refresh by monitor ID.
    /// Returns the invocation IDs of the refresh operations.
    pub fn refresh_monitors_by_id(&mut self, host_id: &String, monitor_id: &String) -> Vec<u64> {
        let host = self.host_manager.borrow().get_host(host_id);
        let monitor = self.monitors.get(host_id).unwrap().iter()
                                   .filter(|(_, monitor)| &monitor.get_module_spec().id == monitor_id)
                                   .collect();

        let invocation_ids = self.refresh_monitors(host, monitor);
        self.invocation_id_counter = invocation_ids.last().unwrap().clone();
        invocation_ids
    }

    /// Use `None` to refresh all monitors on every host or limit by host.
    pub fn refresh_host_monitors(&mut self, host_filter: Option<&String>) -> Vec<u64> {
        let monitors: HashMap<&String, &HashMap<String, Monitor>> = match host_filter {
            Some(host_filter) => self.monitors.iter().filter(|(host_id, _)| host_id == &host_filter).collect(),
            None => self.monitors.iter().collect(),
        };

        let mut invocation_ids = Vec::new();
        for (host_id, monitor_collection) in monitors {
            let host = self.host_manager.borrow().get_host(host_id);
            invocation_ids.extend(self.refresh_monitors(host, monitor_collection.iter().collect()));
        }

        self.invocation_id_counter = invocation_ids.last().unwrap().clone();
        invocation_ids
    }

    fn refresh_monitors(&self, host: Host, monitors: HashMap<&String, &Monitor>) -> Vec<u64> {
        if host.platform.is_unset() {
            log::warn!("[{}] Refreshing monitors despite missing platform info", host.name);
        }

        let mut current_invocation_id = self.invocation_id_counter;
        let mut invocation_ids = Vec::new();

        // Split into 2: base modules and extension modules.
        let (extensions, bases): (Vec<&Monitor>, Vec<&Monitor>) = 
            monitors.values().partition(|monitor| monitor.get_metadata_self().parent_module.is_some());

        for monitor in bases {
            current_invocation_id += 1;
            invocation_ids.push(current_invocation_id);

            let mut request_monitors = vec![monitor.box_clone()];
            if let Some(extension) = extensions.iter().find(|ext| ext.get_metadata_self().parent_module.unwrap() == monitor.get_module_spec()) {
                request_monitors.push(extension.box_clone());
            }

            Self::send_connector_request(
                host.clone(), request_monitors, current_invocation_id,
                self.request_sender.clone(), self.state_update_sender.clone(), DataPoint::empty_and_critical() 
            );
        }

        invocation_ids
    }

    /// Send a connector request to ConnectionManager.
    fn send_connector_request(host: Host, monitors: Vec<Monitor>, invocation_id: u64,
                              request_sender: Sender<ConnectorRequest>, state_update_sender: Sender<StateUpdateMessage>,
                              parent_result: DataPoint) {
        let monitor = monitors[0].box_clone();
        let messages = [monitor.get_connector_messages(host.clone(), parent_result.clone()),
                        vec![monitor.get_connector_message(host.clone(), parent_result.clone())]].concat();
        let response_handler = Self::get_response_handler(
            host.clone(), monitors, invocation_id, request_sender.clone(), state_update_sender.clone(), parent_result
        );

        request_sender.send(ConnectorRequest {
            connector_spec: monitor.get_connector_spec(),
            source_id: monitor.get_module_spec().id,
            host: host.clone(),
            messages: messages,
            request_type: RequestType::Command,
            response_handler: response_handler,
        }).unwrap_or_else(|error| {
            log::error!("Couldn't send message to connector: {}", error);
        });
    }

    fn get_response_handler(host: Host, mut monitors: Vec<Monitor>, invocation_id: u64,
                            request_sender: Sender<ConnectorRequest>, state_update_sender: Sender<StateUpdateMessage>,
                            parent_result: DataPoint) -> ResponseHandlerCallback {

        Box::new(move |results| {
            let monitor = monitors.remove(0);
            let monitor_id = monitor.get_module_spec().id;

            let (responses, errors): (Vec<_>, Vec<_>) =  results.into_iter().partition(Result::is_ok);
            let responses = responses.into_iter().map(Result::unwrap).collect::<Vec<_>>();
            let errors = errors.into_iter().map(Result::unwrap_err).collect::<Vec<_>>();

            let mut new_result = parent_result.clone();
            if errors.is_empty() {
                let process_result = if responses.len() > 1 {
                    monitor.process_responses(host.clone(), responses.clone(), parent_result.clone())
                }
                else if responses.len() == 1 {
                    monitor.process_response(host.clone(), responses[0].to_owned(), parent_result)
                }
                else {
                    // Some special modules require no connectors and receive no response messages.
                    monitor.process_response(host.clone(), ResponseMessage::empty(), parent_result)
                };

                match process_result {
                    Ok(data_point) => {
                        log::debug!("[{}] Data point received for monitor {}: {} {}", host.name, monitor_id, data_point.label, data_point);

                        new_result = data_point;
                    },
                    Err(error) => {
                        log::error!("[{}] Error from monitor {}: {}", host.name, monitor_id, error);
                    }
                }
            }
            else {
                for error in errors {
                    log::error!("[{}] Error refreshing monitor {}: {}", host.name, monitor_id, error);
                }
            }


            new_result.invocation_id = invocation_id;

            if !monitors.is_empty() {
                // Process extension modules recursively until the final result is reached.
                Self::send_connector_request(host, monitors, invocation_id, request_sender, state_update_sender, new_result);
            }
            else {
                Self::send_state_update(&host, &monitor, state_update_sender, new_result);
            }
        })
    }

    /// Send a state update to HostManager.
    fn send_state_update(host: &Host, monitor: &Monitor, state_update_sender: Sender<StateUpdateMessage>, data_point: DataPoint) {
        state_update_sender.send(StateUpdateMessage {
            host_name: host.name.clone(),
            display_options: monitor.get_display_options(),
            module_spec: monitor.get_module_spec(),
            data_point: Some(data_point),
            command_result: None,
            exit_thread: false,
        }).unwrap_or_else(|error| {
            log::error!("Couldn't send message to state manager: {}", error);
        });
    }
}


// Default needs to be implemented because of Qt QObject requirements.
impl Default for MonitorManager {
    fn default() -> Self {
        let (request_sender, _) = mpsc::channel();
        let (state_update_sender, _) = mpsc::channel();
        Self {
            request_sender: request_sender,
            state_update_sender: state_update_sender,
            host_manager: Rc::new(RefCell::new(HostManager::default())),
            invocation_id_counter: 0,
            monitors: HashMap::new(),
        }
    }
}