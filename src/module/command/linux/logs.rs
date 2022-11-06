use std::collections::HashMap;
use crate::frontend;
use crate::module::command::CommandAction;
use crate::module::connection::ResponseMessage;
use crate::module::{
    Module,
    command::CommandModule,
    command::Command,
    command::CommandResult,
    Metadata,
    ModuleSpecification,
};


#[derive(Clone)]
pub struct Logs;

impl Module for Logs {
    fn get_metadata() -> Metadata {
        // TODO: define dependnecy to systemd-service command
        Metadata {
            module_spec: ModuleSpecification::new("logs", "0.0.1"),
            description: String::from(""),
            url: String::from(""),
        }
    }

    fn new(_settings: &HashMap<String, String>) -> Self {
        Logs { }
    }

    fn get_module_spec(&self) -> ModuleSpecification {
        Self::get_metadata().module_spec
    }
}

impl CommandModule for Logs {
    fn clone_module(&self) -> Command {
        Box::new(self.clone())
    }

    fn get_connector_spec(&self) -> Option<ModuleSpecification> {
        Some(ModuleSpecification::new("ssh", "0.0.1"))
    }

    fn get_display_options(&self) -> frontend::DisplayOptions {
        frontend::DisplayOptions {
            category: String::from("host"),
            display_style: frontend::DisplayStyle::Icon,
            display_icon: String::from("view-document"),
            display_text: String::from("Show logs"),
            action: CommandAction::LogView,
            ..Default::default()
        }
    }

    // Parameter 1 is for unit selection and special values "all" and "dmesg".
    // Parameter 2 is for grepping. Filters rows based on regexp.
    fn get_connector_message(&self, parameters: Vec<String>) -> String {
        // TODO: filter out all but alphanumeric characters
        // TODO: validate?

        let mut result = String::from("sudo journalctl -q -n 400");
        if let Some(parameter1) = parameters.first() {
            if !parameter1.is_empty() {
                let suffix = match parameter1.as_str() {
                    "all" => String::from(""),
                    "dmesg" => String::from("--dmesg"),
                    _ => format!("-u {}", parameter1),
                };

                result = format!("{} {}", result, suffix);
            }
        }

        if let Some(parameter2) = parameters.get(1) {
            if !parameter2.is_empty() {
                result = format!("{} -g {}", result, parameter2);
            }
        }
        result
    }

    fn process_response(&self, response: &ResponseMessage) -> Result<CommandResult, String> {
        Ok(CommandResult::new(response.message.clone()))
    }
}