use std::collections::HashMap;
use crate::frontend;
use crate::host::*;
use crate::module::*;
use crate::module::command::*;
use crate::utils::ShellCommand;
use lightkeeper_module::command_module;

#[command_module(
    name="docker-compose-logs",
    version="0.0.1",
    description="Show docker-compose logs for services.",
)]
pub struct Logs {
}

impl Module for Logs {
    fn new(_settings: &HashMap<String, String>) -> Logs {
        Logs {
        }
    }
}

impl CommandModule for Logs {
    fn get_connector_spec(&self) -> Option<ModuleSpecification> {
        Some(ModuleSpecification::new("ssh", "0.0.1"))
    }

    fn get_display_options(&self) -> frontend::DisplayOptions {
        frontend::DisplayOptions {
            category: String::from("docker-compose"),
            parent_id: String::from("docker-compose"),
            display_style: frontend::DisplayStyle::Icon,
            display_icon: String::from("view-document"),
            display_text: String::from("Logs"),
            action: UIAction::LogView,
            multivalue_level: 2,
            ..Default::default()
        }
    }

    fn get_connector_message(&self, host: Host, parameters: Vec<String>) -> Result<String, String> {
        let compose_file = parameters.first().unwrap();
        let project = parameters.get(1).unwrap();

        let mut command = ShellCommand::new();
        command.use_sudo = host.settings.contains(&crate::host::HostSetting::UseSudo);

        if host.platform.version_is_same_or_greater_than(platform_info::Flavor::Debian, "8") ||
           host.platform.version_is_same_or_greater_than(platform_info::Flavor::Ubuntu, "20") {

            // TODO: Don't hardcode page size
            command.arguments(vec!["docker-compose", "-f", compose_file, "logs", "--tail", "400", "--no-color", "-t", project]);
        }
        else if host.platform.version_is_same_or_greater_than(platform_info::Flavor::RedHat, "8") ||
                host.platform.version_is_same_or_greater_than(platform_info::Flavor::CentOS, "8") {

            command.arguments(vec!["docker", "compose", "-f", compose_file, "logs", "--tail", "400", "--no-color", "-t", project]);
        }
        else {
            return Err(String::from("Unsupported platform"));
        }
        Ok(command.to_string())
    }

    fn process_response(&self, _host: Host, response: &connection::ResponseMessage) -> Result<CommandResult, String> {
        // Removes the prefix "PROJECT_NAME_1             |"
        let prefix_removed = response.message.lines().map(|line| {
            line.split_once("|").map(|(_, rest)| rest.trim_start()).unwrap_or(line)
        }).collect::<Vec<&str>>().join("\n");

        if response.is_error() {
            return Err(response.message.clone());
        }
        Ok(CommandResult::new_hidden(prefix_removed))
    }
}