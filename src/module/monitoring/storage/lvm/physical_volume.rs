
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
    name="storage-lvm-physical-volume",
    version="0.0.1",
    description="Provides information about LVM physical volumes.",
)]
pub struct PhysicalVolume {
}

impl Module for PhysicalVolume {
    fn new(_settings: &HashMap<String, String>) -> Self {
        PhysicalVolume {
        }
    }
}

impl MonitoringModule for PhysicalVolume {
    fn get_display_options(&self) -> frontend::DisplayOptions {
        frontend::DisplayOptions {
            display_style: frontend::DisplayStyle::CriticalityLevel,
            display_text: String::from("Physical Volumes"),
            category: String::from("storage"),
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

        if host.platform.version_is_same_or_greater_than(platform_info::Flavor::Debian, "9") ||
           host.platform.version_is_same_or_greater_than(platform_info::Flavor::Ubuntu, "20") ||
           host.platform.version_is_same_or_greater_than(platform_info::Flavor::CentOS, "8") {
            command.arguments(vec!["pvs", "--separator", "|", "--options", "pv_name,pv_attr,pv_size", "--units", "H"]);
            Ok(command.to_string())
        }
        else {
            Err(String::from("Unsupported platform"))
        }
    }

    fn process_response(&self, _host: Host, response: ResponseMessage, _result: DataPoint) -> Result<DataPoint, String> {
        if response.message.is_empty() && response.return_code == 0 {
            return Ok(DataPoint::empty());
        }

        let mut result = DataPoint::empty();

        let lines = response.message.lines().skip(1);
        for line in lines {
            let mut parts = line.split("|");
            let pv_name = parts.next().unwrap().trim_start().to_string();
            let pv_attr = parts.next().unwrap().to_string();
            let pv_size = parts.next().unwrap().to_string();

            let mut data_point = DataPoint::labeled_value(pv_name.clone(), String::from("OK"));
            data_point.description = format!("size: {}", pv_size);

            if pv_attr.chars().nth(2).unwrap() == 'm' {
                data_point.criticality = crate::enums::Criticality::Critical;
                data_point.value = String::from("Missing");
            }

            data_point.command_params = vec![pv_name];
            result.multivalue.push(data_point);
        }

        Ok(result)
    }
}