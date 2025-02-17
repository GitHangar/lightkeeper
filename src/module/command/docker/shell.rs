use std::collections::HashMap;
use crate::frontend;
use crate::host::*;
use crate::module::connection::ResponseMessage;
use crate::module::*;
use crate::module::command::*;
use crate::utils::ShellCommand;
use crate::utils::string_validation;
use lightkeeper_module::command_module;

#[command_module(
    name="docker-shell",
    version="0.0.1",
    description="Opens a shell inside a Docker container.",
)]
pub struct Shell;

impl Module for Shell {
    fn new(_settings: &HashMap<String, String>) -> Self {
        Shell { }
    }
}

impl CommandModule for Shell {
    fn get_connector_spec(&self) -> Option<ModuleSpecification> {
        Some(ModuleSpecification::new("ssh", "0.0.1"))
    }

    fn get_display_options(&self) -> frontend::DisplayOptions {
        frontend::DisplayOptions {
            category: String::from("docker-containers"),
            parent_id: String::from("docker-containers"),
            display_style: frontend::DisplayStyle::Icon,
            display_icon: String::from("terminal"),
            display_text: String::from("Open shell inside"),
            action: UIAction::Terminal,
            ..Default::default()
        }
    }

    fn get_connector_message(&self, host: Host, parameters: Vec<String>) -> Result<String, String> {
        let target_id = parameters.first().unwrap();

        if !string_validation::is_alphanumeric(target_id) {
            panic!("Invalid container ID: {}", target_id)
        }

        let mut command = ShellCommand::new();
        command.use_sudo = host.settings.contains(&crate::host::HostSetting::UseSudo);

        if host.platform.version_is_same_or_greater_than(platform_info::Flavor::Debian, "8") ||
           host.platform.version_is_same_or_greater_than(platform_info::Flavor::Ubuntu, "20") {

            command.arguments(vec!["docker", "exec", "-it", target_id, "/bin/sh"]);
        }

        else if host.platform.version_is_same_or_greater_than(platform_info::Flavor::RedHat, "8") ||
                host.platform.version_is_same_or_greater_than(platform_info::Flavor::CentOS, "8") {

            command.arguments(vec!["docker", "exec", "-it", target_id, "/bin/sh"]);
        }
        else {
            return Err(String::from("Unsupported platform"));
        }

        Ok(command.to_string())
    }

    fn process_response(&self, _host: Host, response: &ResponseMessage) -> Result<CommandResult, String> {
        Ok(CommandResult::new_info(response.message.clone()))
    }
}