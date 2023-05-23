use std::collections::HashMap;
use crate::frontend;
use crate::host::*;
use crate::module::connection::ResponseMessage;
use crate::module::*;
use crate::module::command::*;
use crate::utils::ShellCommand;
use lightkeeper_module::command_module;

#[command_module("linux-packages-update", "0.0.1")]
pub struct Update;

impl Module for Update {
    fn new(_settings: &HashMap<String, String>) -> Self {
        Self { }
    }
}

impl CommandModule for Update {
    fn get_connector_spec(&self) -> Option<ModuleSpecification> {
        Some(ModuleSpecification::new("ssh", "0.0.1"))
    }

    fn get_display_options(&self) -> frontend::DisplayOptions {
        frontend::DisplayOptions {
            category: String::from("packages"),
            parent_id: String::from("package"),
            display_style: frontend::DisplayStyle::Icon,
            display_icon: String::from("update"),
            display_text: String::from("Upgrade package"),
            ..Default::default()
        }
    }

    fn get_connector_message(&self, host: Host, parameters: Vec<String>) -> String {
        let package = parameters.first().unwrap();

        let mut command = ShellCommand::new();
        if host.platform.os == platform_info::OperatingSystem::Linux {
            if host.platform.version_is_newer_than(platform_info::Flavor::Debian, "7") &&
               host.platform.version_is_older_than(platform_info::Flavor::Debian, "11") {
                command.arguments(vec!["apt", "--only-upgrade", "-y", "install", package]); 
            }

            command.use_sudo = host.settings.contains(&HostSetting::UseSudo);
        }

        command.to_string()
    }

    fn process_response(&self, _host: Host, response: &ResponseMessage) -> Result<CommandResult, String> {
        // TODO: view output messages of installation (can be pretty long)?
        if response.return_code == 0 {
            Ok(CommandResult::new(response.message.clone()))
        }
        else {
            Ok(CommandResult::new_error(response.message.clone()))
        }
    }
}