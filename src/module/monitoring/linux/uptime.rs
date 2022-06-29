
use std::collections::HashMap;
use chrono::{ NaiveDateTime, Utc };
use crate::{
    utils::strip_newline,
    Host,
};
use crate::module::{
    Module,
    Metadata,
    monitoring::{MonitoringModule, DisplayStyle, DisplayOptions, DataPoint},
    ModuleSpecification,
};

pub struct Uptime {
}

impl Module for Uptime {
    fn get_metadata() -> Metadata {
        Metadata {
            module_spec: ModuleSpecification::new(String::from("uptime"), "0.0.1"),
            category: String::from("host"),
            description: String::from(""),
            url: String::from(""),
        }
    }

    fn new(_settings: &HashMap<String, String>) -> Self {
        Uptime { }
    }

    fn get_module_spec(&self) -> ModuleSpecification {
        Self::get_metadata().module_spec
    }
}

impl MonitoringModule for Uptime {
    fn get_display_options(&self) -> DisplayOptions {
        DisplayOptions {
            display_style: DisplayStyle::String,
            display_name: String::from("Uptime"),
            use_multivalue: false,
            unit: String::from("d"),
        }
    }

    fn get_connector_spec(&self) -> Option<ModuleSpecification> {
        Some(ModuleSpecification::new(String::from("ssh"), "0.0.1"))
    }

    fn get_connector_message(&self) -> String {
        String::from("uptime -s")
    }

    fn process(&self, _host: &Host, response: &String, _connector_is_connected: bool) -> Result<DataPoint, String> {
        let boot_datetime = NaiveDateTime::parse_from_str(&strip_newline(response), "%Y-%m-%d %H:%M:%S")
                                          .map_err(|e| e.to_string())?;

        let uptime = Utc::now().naive_utc() - boot_datetime;
        Ok(DataPoint::new(uptime.num_days().to_string()))
    }
}