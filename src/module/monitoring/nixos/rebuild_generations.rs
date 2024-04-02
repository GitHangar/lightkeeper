use serde_derive::Deserialize;
use serde_json;
use std::collections::HashMap;
use crate::module::connection::ResponseMessage;
use crate::{
    Host,
    frontend,
};

use lightkeeper_module::monitoring_module;
use crate::module::*;
use crate::module::monitoring::*;
use crate::utils::ShellCommand;
use crate::host::HostSetting;

#[monitoring_module(
    name="nixos-rebuild-generations",
    version="0.0.1",
    description="Provides information about configuration generations.",
    settings={
    }
)]
pub struct RebuildGenerations {
}

impl Module for RebuildGenerations {
    fn new(_settings: &HashMap<String, String>) -> Self {
        RebuildGenerations {
        }
    }
}

impl MonitoringModule for RebuildGenerations {
    fn get_display_options(&self) -> frontend::DisplayOptions {
        frontend::DisplayOptions {
            display_style: frontend::DisplayStyle::Text,
            display_text: String::from("Generations"),
            category: String::from("nixos"),
            use_multivalue: true,
            ignore_from_summary: true,
            ..Default::default()
        }
    }

    fn get_connector_spec(&self) -> Option<ModuleSpecification> {
        Some(ModuleSpecification::new("ssh", "0.0.1"))
    }

    fn get_connector_message(&self, host: Host, _result: DataPoint) -> Result<String, String> {
        let mut command = ShellCommand::new();
        command.use_sudo = host.settings.contains(&HostSetting::UseSudo);
        command.ignore_stderr = true;

        if host.platform.is_same_or_greater(platform_info::Flavor::NixOS, "20") {
            command.arguments(vec!["nixos-rebuild", "list-generations", "--json"]);
            Ok(command.to_string())
        }
        else {
            Err(String::from("Unsupported platform"))
        }
    }

    fn process_response(&self, _host: Host, response: ResponseMessage, _result: DataPoint) -> Result<DataPoint, String> {
        let mut result = DataPoint::empty();

        let mut generations: Vec<GenerationData> = serde_json::from_str(response.message.as_str()).map_err(|e| {
            log::error!("Failed to parse JSON response: {}", e.to_string());
            e.to_string()
        })?;
        generations.sort_by_key(|generation| -(generation.generation as i32));

        for generation in generations.iter() {
            // let parsed_date = NaiveDateTime::parse_from_str(&generation.date, "%Y-%m-%dT%H:%M:%SZ").unwrap().and_utc();
            let date_string = generation.date.replace("T", " ").replace("Z", "");

            let mut data_point = DataPoint::empty();
            data_point.label = format!("#{} @ {}", generation.generation, date_string);
            data_point.description = format!("NixOS {} | Kernel {}", generation.nixosVersion, generation.kernelVersion);

            if generation.current {
                data_point.tags.push(String::from("Current"));
            }

            result.multivalue.push(data_point);
        }

        Ok(result)
    }
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct GenerationData {
    generation: u16,
    date: String,
    nixosVersion: String,
    kernelVersion: String,
    // configurationRevision: String,
    // specialisations: Vec<String>,
    current: bool,
}