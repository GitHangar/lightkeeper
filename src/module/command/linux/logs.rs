use std::collections::HashMap;
use crate::frontend;
use crate::module::command::UIAction;
use crate::module::connection::ResponseMessage;
use crate::module::*;
use crate::module::command::*;
use lightkeeper_module::command_module;

#[command_module("logs", "0.0.1")]
pub struct Logs;

impl Module for Logs {
    fn new(_settings: &HashMap<String, String>) -> Self {
        Logs { }
    }
}

impl CommandModule for Logs {
    fn get_connector_spec(&self) -> Option<ModuleSpecification> {
        Some(ModuleSpecification::new("ssh", "0.0.1"))
    }

    fn get_display_options(&self) -> frontend::DisplayOptions {
        frontend::DisplayOptions {
            category: String::from("host"),
            display_style: frontend::DisplayStyle::Icon,
            display_icon: String::from("view-document"),
            display_text: String::from("Show logs"),
            action: UIAction::LogView,
            ..Default::default()
        }
    }

    // Parameter 1 is for unit selection and special values "all" and "dmesg".
    // Parameter 2 is for grepping. Filters rows based on regexp.
    fn get_connector_message(&self, _platform: PlatformInfo, parameters: Vec<String>) -> String {
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

    fn process_response(&self, _platform: PlatformInfo, response: &ResponseMessage) -> Result<CommandResult, String> {
        Ok(CommandResult::new(response.message.clone()))
    }
}