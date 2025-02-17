use serde_derive::{ Serialize, Deserialize };
use serde_yaml;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{ fs, io, collections::HashMap };
use crate::host::HostSetting;
use crate::file_handler;

const MAIN_CONFIG_FILE: &str = "config.yml";
const HOSTS_FILE: &str = "hosts.yml";
const GROUPS_FILE: &str = "groups.yml";
pub const INTERNAL: &str = "internal";


#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    pub preferences: Preferences,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_options: Option<DisplayOptions>,
    pub cache_settings: CacheSettings,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct Groups {
    pub groups: HashMap<String, ConfigGroup>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct Hosts {
    pub hosts: HashMap<String, HostSettings>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct Preferences {
    #[serde(default)]
    pub use_sandbox_mode: bool,
    pub refresh_hosts_on_start: bool,
    pub use_remote_editor: bool,
    pub sudo_remote_editor: bool,
    // TODO: check for valid command.
    pub remote_text_editor: String,
    // TODO: check for valid path.
    /// Command to run when launching a text editor. "internal" is a special value that uses the internal editor.
    pub text_editor: String,
    /// Command to run when launching a terminal. "internal" is a special value that uses the internal terminal.
    pub terminal: String,
    pub terminal_args: Vec<String>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct DisplayOptions {
    pub qtquick_style: String,
    pub hide_info_notifications: bool,
    pub categories: HashMap<String, Category>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct CacheSettings {
    /// Enable cache. Set false to disable completely and make sure cache file is empty.
    /// Otherwise, cache file is maintained even if it's not used at that moment. This setting will make sure it's not used at all.
    pub enable_cache: bool,
    /// Cache provides an initial value before receiving the up-to-date value.
    pub provide_initial_value: bool,
    /// How long entries in cache are considered valid.
    pub initial_value_time_to_live: u64,
    /// If enabled, value is returned only from cache if it is available.
    pub prefer_cache: bool,
    /// How long entries in cache are considered valid.
    pub time_to_live: u64,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct Category {
    pub priority: u16,
    #[serde(default, skip_serializing_if = "Configuration::is_default")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Configuration::is_default")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Configuration::is_default")]
    pub command_order: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Configuration::is_default")]
    pub monitor_order: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Configuration::is_default")]
    pub collapsible_commands: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct HostSettings {
    #[serde(default)]
    pub groups: Vec<String>,
    #[serde(default = "HostSettings::default_address", skip_serializing_if = "HostSettings::is_default_address")]
    pub address: String,
    #[serde(default, skip_serializing_if = "Configuration::is_default")]
    pub fqdn: String,
    #[serde(default, skip_serializing_if = "Configuration::is_default")]
    pub settings: Vec<HostSetting>,
    // Currently, you have to use config groups instead of setting these directly on host.
    // So these are never written but will be populated from groups on config read.
    #[serde(default, skip_serializing_if = "Configuration::always")]
    pub monitors: HashMap<String, MonitorConfig>,
    #[serde(default, skip_serializing_if = "Configuration::always")]
    pub commands: HashMap<String, CommandConfig>,
    #[serde(default, skip_serializing_if = "Configuration::always")]
    pub connectors: HashMap<String, ConnectorConfig>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct ConfigGroup {
    #[serde(default, skip_serializing_if = "Configuration::is_default")]
    pub host_settings: Vec<HostSetting>,
    #[serde(default, skip_serializing_if = "Configuration::is_default")]
    pub monitors: HashMap<String, MonitorConfig>,
    #[serde(default, skip_serializing_if = "Configuration::is_default")]
    pub commands: HashMap<String, CommandConfig>,
    #[serde(default, skip_serializing_if = "Configuration::is_default")]
    pub connectors: HashMap<String, ConnectorConfig>,

}

impl HostSettings {
    pub fn default_address() -> String {
        String::from("0.0.0.0")
    }

    pub fn is_default_address(address: &String) -> bool {
        address == "0.0.0.0"
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MonitorConfig {
    #[serde(default = "MonitorConfig::default_version", skip_serializing_if = "Configuration::version_is_latest")]
    pub version: String,
    #[serde(default = "MonitorConfig::default_enabled", skip_serializing_if = "MonitorConfig::is_enabled")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Configuration::is_default")]
    pub is_critical: Option<bool>,
    #[serde(default, skip_serializing_if = "Configuration::is_default")]
    pub settings: HashMap<String, String>,
}

impl MonitorConfig {
    pub fn default_version() -> String {
        String::from("latest")
    }

    pub fn default_enabled() -> Option<bool> {
        Some(true)
    }

    pub fn is_enabled(enabled: &Option<bool>) -> bool {
        enabled.clone().unwrap_or(true)
    }
}

impl Default for MonitorConfig {
    fn default() -> Self {
        MonitorConfig {
            version: MonitorConfig::default_version(),
            enabled: MonitorConfig::default_enabled(),
            is_critical: None,
            settings: HashMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CommandConfig {
    #[serde(default = "CommandConfig::default_version", skip_serializing_if = "Configuration::version_is_latest")]
    pub version: String,
    #[serde(default, skip_serializing_if = "Configuration::is_default")]
    pub settings: HashMap<String, String>,
}

impl CommandConfig {
    pub fn default_version() -> String {
        String::from("latest")
    }
}

impl Default for CommandConfig {
    fn default() -> Self {
        CommandConfig {
            version: CommandConfig::default_version(),
            settings: HashMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConnectorConfig {
    #[serde(default)]
    pub settings: HashMap<String, String>,
}

impl Configuration {
    pub fn read(config_dir: &String) -> io::Result<(Configuration, Hosts, Groups)> {
        let config_dir = if config_dir.is_empty() {
            file_handler::get_config_dir().unwrap()
        }
        else {
            Path::new(config_dir).to_path_buf()
        };

        let main_config_file_path = config_dir.join(MAIN_CONFIG_FILE);
        let hosts_file_path = config_dir.join(HOSTS_FILE);
        let groups_file_path = config_dir.join(GROUPS_FILE);
        let old_templates_file_path = config_dir.join("templates.yml");

        // If main configuration is missing, this is probably the first run, so create initial configurations.
        if let Err(_) = fs::metadata(&main_config_file_path) {
            Self::write_initial_config(config_dir)?;
        }
        else if let Ok(_) = fs::metadata(config_dir.join("templates.yml")) {
            log::warn!("Old templates.yml configuration file found. Renaming old configuration files and reinitializing.");

            // This is the old groups.yml file. Rename old files with .old suffix and do a new init.
            fs::rename(&main_config_file_path, config_dir.join(format!("{}.old", MAIN_CONFIG_FILE)))?;
            fs::rename(&hosts_file_path, config_dir.join(format!("{}.old", HOSTS_FILE)))?;
            fs::rename(old_templates_file_path, config_dir.join("templates.yml.old"))?;

            Self::write_initial_config(config_dir)?;
        }

        log::info!("Reading main configuration from {}", main_config_file_path.display());
        let config_contents = fs::read_to_string(main_config_file_path)?;

        let mut main_config = serde_yaml::from_str::<Configuration>(config_contents.as_str())
                                     .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;

        // Display options are currently defined in the app's defaults and not really user-configurable.
        let default_main_config = include_str!("../config.example.yml");
        let default_parsed = serde_yaml::from_str::<Configuration>(default_main_config)
                                        .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;
        main_config.display_options = Some(default_parsed.display_options.unwrap());

        log::info!("Reading host configuration from {}", hosts_file_path.display());
        let hosts_contents = fs::read_to_string(hosts_file_path)?;
        let mut hosts = serde_yaml::from_str::<Hosts>(hosts_contents.as_str())
                                   .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;

        log::info!("Reading group configuration from {}", groups_file_path.display());
        let groups_contents = fs::read_to_string(groups_file_path)?;
        let all_groups = serde_yaml::from_str::<Groups>(groups_contents.as_str())
                                    .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;

        // Check there are no invalid group references.
        let invalid_groups = hosts.hosts.values()
            .flat_map(|host_config| host_config.groups.clone())
            .filter(|group_id| !all_groups.groups.contains_key(group_id))
            .collect::<Vec<String>>();

        if !invalid_groups.is_empty() {
            let error_message = format!("Invalid group references: {}", invalid_groups.join(", "));
            return Err(io::Error::new(io::ErrorKind::Other, error_message));
        }

        for (_, host_config) in hosts.hosts.iter_mut() {
            for group_id in host_config.groups.clone().iter() {
                let group_config = all_groups.groups.get(group_id).unwrap();

                // NOTE: Host settings are not merged.
                if !group_config.host_settings.is_empty() {
                    host_config.settings = group_config.host_settings.clone();
                }

                // Merge groups.
                group_config.monitors.iter().for_each(|(monitor_id, new_config)| {
                    let mut merged_config = host_config.monitors.get(monitor_id).cloned().unwrap_or(MonitorConfig::default());
                    merged_config.settings.extend(new_config.settings.clone());
                    merged_config.is_critical = new_config.is_critical;
                    host_config.monitors.insert(monitor_id.clone(), merged_config);
                });

                group_config.commands.iter().for_each(|(command_id, new_config)| {
                    let mut merged_config = host_config.commands.get(command_id).cloned().unwrap_or(CommandConfig::default());
                    merged_config.settings.extend(new_config.settings.clone());
                    merged_config.version = new_config.version.clone();
                    host_config.commands.insert(command_id.clone(), merged_config);
                });

                group_config.connectors.iter().for_each(|(connector_id, new_config)| {
                    let mut merged_config = host_config.connectors.get(connector_id).cloned().unwrap_or(ConnectorConfig::default());
                    merged_config.settings.extend(new_config.settings.clone());
                    host_config.connectors.insert(connector_id.clone(), merged_config);
                });
            }
        }

        Ok((main_config, hosts, all_groups))
    }

    pub fn write_initial_config(config_dir: PathBuf) -> io::Result<()> {
        let default_main_config = include_str!("../config.example.yml");
        let default_hosts_config = include_str!("../hosts.example.yml");
        let default_groups_config = include_str!("../groups.example.yml");

        let main_config_file_path = config_dir.join(MAIN_CONFIG_FILE);
        let hosts_file_path = config_dir.join(HOSTS_FILE);
        let groups_file_path = config_dir.join(GROUPS_FILE);

        fs::create_dir_all(&config_dir)?;

        let main_config_file = fs::OpenOptions::new().write(true).create_new(true).open(main_config_file_path.clone());
        match main_config_file {
            Ok(mut file) => {
                if let Err(error) = file.write_all(default_main_config.as_bytes()) {
                    let message = format!("Failed to write main configuration file {}: {}", main_config_file_path.to_string_lossy(), error);
                    return Err(io::Error::new(io::ErrorKind::Other, message));
                }
                else {
                    log::info!("Created new main configuration file {}", main_config_file_path.to_string_lossy());
                }
            },
            Err(error) => {
                let message = format!("Failed to create main configuration file {}: {}", main_config_file_path.to_string_lossy(), error);
                return Err(io::Error::new(io::ErrorKind::Other, message));
            }
        }

        let hosts_config_file = fs::OpenOptions::new().write(true).create_new(true).open(hosts_file_path.clone());
        match hosts_config_file {
            Ok(mut file) => {
                if let Err(error) = file.write_all(default_hosts_config.as_bytes()) {
                    let message = format!("Failed to write host configuration file {}: {}", hosts_file_path.to_string_lossy(), error);
                    return Err(io::Error::new(io::ErrorKind::Other, message));
                }
                else {
                    log::info!("Created new host configuration file {}", hosts_file_path.to_string_lossy());
                }
            },
            Err(error) => {
                let message = format!("Failed to create host configuration file {}: {}", hosts_file_path.to_string_lossy(), error);
                return Err(io::Error::new(io::ErrorKind::Other, message));
            }
        }

        let groups_config_file = fs::OpenOptions::new().write(true).create_new(true).open(groups_file_path.clone());
        match groups_config_file {
            Ok(mut file) => {
                if let Err(error) = file.write_all(default_groups_config.as_bytes()) {
                    let message = format!("Failed to write group configuration file {}: {}", groups_file_path.to_string_lossy(), error);
                    return Err(io::Error::new(io::ErrorKind::Other, message));
                }
                else {
                    log::info!("Created new group configuration file {}", groups_file_path.to_string_lossy());
                }
            },
            Err(error) => {
                let message = format!("Failed to create group configuration file {}: {}", groups_file_path.to_string_lossy(), error);
                return Err(io::Error::new(io::ErrorKind::Other, message));
            }
        }

        Ok(())
    }

    /// Writes the hosts.yml configuration file.
    pub fn write_hosts_config(config_dir: &String, hosts: &Hosts) -> io::Result<()> {
        let config_dir = if config_dir.is_empty() {
            file_handler::get_config_dir().unwrap()
        }
        else {
            Path::new(config_dir).to_path_buf()
        };

        let hosts_file_path = config_dir.join(HOSTS_FILE);
        let hosts_config_file = fs::OpenOptions::new().write(true).truncate(true).open(hosts_file_path.clone());
        match hosts_config_file {
            Ok(mut file) => {
                let hosts_config = serde_yaml::to_string(hosts).unwrap();
                if let Err(error) = file.write_all(hosts_config.as_bytes()) {
                    let message = format!("Failed to write host configuration file {}: {}", hosts_file_path.to_string_lossy(), error);
                    return Err(io::Error::new(io::ErrorKind::Other, message));
                }
                else {
                    log::info!("Updated host configuration file {}", hosts_file_path.to_string_lossy());
                }
            },
            Err(error) => {
                let message = format!("Failed to open host configuration file {}: {}", hosts_file_path.to_string_lossy(), error);
                return Err(io::Error::new(io::ErrorKind::Other, message));
            }
        }

        Ok(())
    }

    /// Writes the groups.yml configuration file.
    pub fn write_groups_config(config_dir: &String, groups: &Groups) -> io::Result<()> {
        let config_dir = if config_dir.is_empty() {
            file_handler::get_config_dir().unwrap()
        }
        else {
            Path::new(config_dir).to_path_buf()
        };

        let groups_file_path = config_dir.join(GROUPS_FILE);
        let groups_config_file = fs::OpenOptions::new().write(true).truncate(true).open(groups_file_path.clone());
        match groups_config_file {
            Ok(mut file) => {
                let groups_config = serde_yaml::to_string(groups).unwrap();
                if let Err(error) = file.write_all(groups_config.as_bytes()) {
                    let message = format!("Failed to write group configuration file {}: {}", groups_file_path.to_string_lossy(), error);
                    return Err(io::Error::new(io::ErrorKind::Other, message));
                }
                else {
                    log::info!("Updated group configuration file {}", groups_file_path.to_string_lossy());
                }
            },
            Err(error) => {
                let message = format!("Failed to open group configuration file {}: {}", groups_file_path.to_string_lossy(), error);
                return Err(io::Error::new(io::ErrorKind::Other, message));
            }
        }

        Ok(())
    }

    /// Writes the config.yml configuration file.
    pub fn write_main_config(config_dir: &String, config: &Configuration) -> io::Result<()> {
        let config_dir = if config_dir.is_empty() {
            file_handler::get_config_dir().unwrap()
        }
        else {
            Path::new(config_dir).to_path_buf()
        };

        let main_config_file_path = config_dir.join(MAIN_CONFIG_FILE);
        let main_config_file = fs::OpenOptions::new().write(true).truncate(true).open(main_config_file_path.clone());
        match main_config_file {
            Ok(mut file) => {
                // Display options are currently not really user-configurable.
                let config_without_display_options = Configuration {
                    preferences: config.preferences.clone(),
                    cache_settings: config.cache_settings.clone(),
                    display_options: None,
                };

                let main_config = serde_yaml::to_string(&config_without_display_options).unwrap();
                if let Err(error) = file.write_all(main_config.as_bytes()) {
                    let message = format!("Failed to write main configuration file {}: {}", main_config_file_path.to_string_lossy(), error);
                    return Err(io::Error::new(io::ErrorKind::Other, message));
                }
                else {
                    log::info!("Updated main configuration file {}", main_config_file_path.to_string_lossy());
                }
            },
            Err(error) => {
                let message = format!("Failed to open main configuration file {}: {}", main_config_file_path.to_string_lossy(), error);
                return Err(io::Error::new(io::ErrorKind::Other, message));
            }
        }

        Ok(())
    }

    fn is_default<T: Default + PartialEq>(t: &T) -> bool {
        t == &T::default()
    }

    fn always<T>(_t: &T) -> bool {
        true
    }

    pub fn version_is_latest(version: &str) -> bool {
        version == "latest"
    }
}
