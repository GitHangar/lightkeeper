
use std::collections::HashMap;
use serde_derive::Serialize;
use chrono::{DateTime, Utc};

use crate::{
    module::Module,
    module::ModuleSpecification,
    utils::enums::Criticality,
    frontend,
};

pub type Command = Box<dyn CommandModule + Send + Sync>;

pub trait CommandModule : Module {
    fn new_command_module(settings: &HashMap<String, String>) -> Command where Self: Sized + 'static + Send + Sync {
        Box::new(Self::new(settings))
    }

    fn get_connector_spec(&self) -> Option<ModuleSpecification> {
        None
    }

    // TODO: less boilerplate for module implementation?
    fn clone_module(&self) -> Command;

    fn get_display_options(&self) -> frontend::DisplayOptions {
        frontend::DisplayOptions {
            display_style: frontend::DisplayStyle::Text,
            display_text: self.get_module_spec().id,
            ..Default::default()
        }
    }

    fn get_action(&self) -> CommandAction {
        CommandAction::None
    }

    // target_id is just an optional argument for the command.
    // TODO: rename?
    fn get_connector_request(&self, _target_id: String) -> String;
    fn process_response(&self, response: &String) -> Result<CommandResult, String>;
}


#[derive(Clone, Serialize)]
pub struct CommandResult {
    pub message: String,
    pub criticality: Criticality,
    pub time: DateTime<Utc>,
}

impl CommandResult {
    pub fn new(message: String) -> Self {
        CommandResult {
            message: message,
            criticality: Criticality::Normal,
            time: Utc::now(),
        }
    }

    pub fn new_with_level(message: String, criticality: Criticality) -> Self {
        CommandResult {
            message: message,
            criticality: criticality,
            time: Utc::now(),
        }
    }


    pub fn empty() -> Self {
        Default::default()
    }

    pub fn empty_and_critical() -> Self {
        let mut empty = Self::empty();
        empty.criticality = Criticality::Critical;
        empty
    }
}

impl Default for CommandResult {
    fn default() -> Self {
        CommandResult {
            message: String::from(""),
            criticality: Criticality::Normal,
            time: Utc::now(),
        }
    }
}

#[derive(Clone, Serialize)]
pub enum CommandAction {
    None,
    Dialog,
}