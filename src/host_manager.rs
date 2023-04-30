
use std::collections::{HashMap, VecDeque};
use std::str::FromStr;
use std::sync::mpsc;
use std::thread;
use std::sync::{Arc, Mutex};

use crate::module::platform_info;
use crate::module::{
    ModuleSpecification,
    monitoring::MonitoringData,
    monitoring::DataPoint,
    command::CommandResult,
};

use crate::utils::VersionNumber;
use crate::{
    enums::HostStatus,
    enums::Criticality,
    host::Host,
    frontend,
};

const DATA_POINT_BUFFER_SIZE: usize = 4;


// TODO: Split to StateManager and HostCollection?
pub struct HostManager {
    hosts: Arc<Mutex<HostCollection>>,
    /// Provides sender handles for sending StateUpdateMessages to this instance.
    data_sender_prototype: mpsc::Sender<StateUpdateMessage>,
    receiver_handle: Option<thread::JoinHandle<()>>,
    observers: Arc<Mutex<Vec<mpsc::Sender<frontend::HostDisplayData>>>>,
}

impl HostManager {
    pub fn new() -> HostManager {
        let (sender, receiver) = mpsc::channel::<StateUpdateMessage>();
        let shared_hosts = Arc::new(Mutex::new(HostCollection::new()));
        let observers = Arc::new(Mutex::new(Vec::new()));

        let handle = Self::start_receiving_updates(shared_hosts.clone(), receiver, observers.clone());

        HostManager {
            hosts: shared_hosts,
            data_sender_prototype: sender,
            receiver_handle: Some(handle),
            observers: observers,
        }
    }

    pub fn join(&mut self) {
        self.receiver_handle.take().expect("Thread has already stopped.")
                            .join().unwrap();
    }

    pub fn add_host(&mut self, host: Host) -> Result<(), String> {
        let mut hosts = self.hosts.lock().unwrap();
        hosts.add(host, HostStatus::Pending)
    }

    /// Get Host details by name. Panics if the host is not found.
    pub fn get_host(&self, host_name: &String) -> Host {
        let hosts = self.hosts.lock().unwrap();
        hosts.hosts.get(host_name)
                   .expect(format!("Host '{}' not found", host_name).as_str())
                   .host.clone()
    }

    pub fn new_state_update_sender(&self) -> mpsc::Sender<StateUpdateMessage> {
        self.data_sender_prototype.clone()
    }

    pub fn add_observer(&mut self, sender: mpsc::Sender<frontend::HostDisplayData>) {
        self.observers.lock().unwrap().push(sender);
    }

    fn start_receiving_updates(hosts: Arc<Mutex<HostCollection>>, receiver: mpsc::Receiver<StateUpdateMessage>,
        observers: Arc<Mutex<Vec<mpsc::Sender<frontend::HostDisplayData>>>>) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            loop {
                let message = match receiver.recv() {
                    Ok(data) => data,
                    Err(error) => {
                        log::error!("Stopped receiver thread: {}", error);
                        return;
                    }
                };

                if message.exit_thread {
                    log::debug!("Gracefully exiting state manager thread.");
                    for observer in observers.lock().unwrap().iter() {
                        observer.send(frontend::HostDisplayData::exit_token()).unwrap();
                    }
                    return;
                }

                let mut hosts = hosts.lock().unwrap();
                let host_state = hosts.hosts.get_mut(&message.host_name).unwrap();
                let mut new_monitoring_data: Option<MonitoringData> = None;
                let mut new_command_results: Option<CommandResult> = None;

                if let Some(message_data_point) = message.data_point {

                    // Specially structured data point for passing platform info here.
                    // TODO: remove hardcoding?
                    if message_data_point.value == "_platform_info" {
                        if let Ok(platform) = Self::read_platform_info(message_data_point) {
                            host_state.host.platform = platform;
                            log::debug!("[{}] Platform info updated", host_state.host.name);
                        }
                        else {
                            log::error!("[{}] Invalid platform info received", host_state.host.name);
                        }
                    }
                    else {
                        // Check first if there already exists a key for monitor id.
                        if let Some(monitoring_data) = host_state.monitor_data.get_mut(&message.module_spec.id) {

                            monitoring_data.values.push_back(message_data_point.clone());

                            if monitoring_data.values.len() > DATA_POINT_BUFFER_SIZE {
                                monitoring_data.values.pop_front();
                            }
                        }
                        else {
                            let mut new_data = MonitoringData::new(message.module_spec.id.clone(), message.display_options);
                            new_data.values.push_back(message_data_point.clone());
                            host_state.monitor_data.insert(message.module_spec.id.clone(), new_data);
                        }

                        // Also add to a list of new data points.
                        let mut new = host_state.monitor_data.get(&message.module_spec.id).unwrap().clone();
                        new.values = VecDeque::from(vec![message_data_point.clone()]);
                        new_monitoring_data = Some(new.clone());
                    }
                }
                else if let Some(command_result) = message.command_result {
                    host_state.command_results.insert(message.module_spec.id, command_result.clone());
                    // Also add to a list of new command results.
                    new_command_results = Some(command_result);
                }

                host_state.update_status();

                // Send the state update to the front end.
                let observers = observers.lock().unwrap();
                for observer in observers.iter() {
                    observer.send(frontend::HostDisplayData {
                        name: host_state.host.name.clone(),
                        domain_name: host_state.host.fqdn.clone(),
                        platform: host_state.host.platform.clone(),
                        ip_address: host_state.host.ip_address.clone(),
                        monitoring_data: host_state.monitor_data.clone(),
                        new_monitoring_data: new_monitoring_data.clone(),
                        command_results: host_state.command_results.clone(),
                        new_command_results: new_command_results.clone(),
                        status: host_state.status,
                        exit_thread: false,
                    }).unwrap();
                }
            }
        })
    }

    pub fn get_display_data(&self) -> frontend::DisplayData {
        let mut display_data = frontend::DisplayData::new();

        let hosts = self.hosts.lock().unwrap();
        for (_, host_state) in hosts.hosts.iter() {
            for (monitor_id, monitor_data) in host_state.monitor_data.iter() {
                if !display_data.all_monitor_names.contains(monitor_id) {
                    display_data.all_monitor_names.push(monitor_id.clone());

                    let header = match monitor_data.display_options.unit.is_empty() {
                        true => format!("{}", monitor_data.display_options.display_text),
                        false => format!("{} ({})", monitor_data.display_options.display_text, monitor_data.display_options.unit),
                    };
                    display_data.table_headers.push(header);
                }
            }
        }

        for (host_name, state) in hosts.hosts.iter() {
            display_data.hosts.insert(host_name.clone(), frontend::HostDisplayData {
                name: state.host.name.clone(),
                domain_name: state.host.fqdn.clone(),
                platform: state.host.platform.clone(),
                ip_address: state.host.ip_address.clone(),
                monitoring_data: state.monitor_data.clone(),
                new_monitoring_data: None,
                command_results: state.command_results.clone(),
                new_command_results: None,
                status: state.status,
                exit_thread: false,
            });
        }

        display_data
    }

    fn read_platform_info(data_point: DataPoint) -> Result<platform_info::PlatformInfo, String> {
        let mut platform = platform_info::PlatformInfo::default();
        for data in data_point.multivalue.iter() {
            match data.label.as_str() {
                "os" => {
                    platform.os = platform_info::OperatingSystem::from_str(data.value.as_str()).map_err(|error| error.to_string())?
                },
                "os_version" => {
                    platform.os_version = VersionNumber::from_string(&data.value)
                },
                "os_flavor" => {
                    platform.os_flavor = platform_info::Flavor::from_str(data.value.as_str()).map_err(|error| error.to_string())?
                },
                "architecture" => {
                    platform.architecture = platform_info::Architecture::from(&data.value)
                },
                _ => return Err(String::from("Invalid platform info data"))
            }
        }
        Ok(platform)
    }
}

impl Default for HostManager {
    fn default() -> Self {
        HostManager::new()
    }
}

#[derive(Default)]
pub struct StateUpdateMessage {
    pub host_name: String,
    pub display_options: frontend::DisplayOptions,
    pub module_spec: ModuleSpecification,
    // Only used with MonitoringModule.
    pub data_point: Option<DataPoint>,
    pub command_result: Option<CommandResult>,
    pub exit_thread: bool,
}

impl StateUpdateMessage {
    pub fn exit_token() -> Self {
        StateUpdateMessage {
            exit_thread: true,
            ..Default::default()
        }
    }
}


struct HostCollection {
    hosts: HashMap<String, HostState>,
}

impl HostCollection {
    fn new() -> Self {
        HostCollection {
            hosts: HashMap::new(),
        }
    }

    fn add(&mut self, host: Host, default_status: HostStatus) -> Result<(), String> {
        if self.hosts.contains_key(&host.name) {
            return Err(String::from("Host already exists"));
        }

        self.hosts.insert(host.name.clone(), HostState::from_host(host, default_status));
        Ok(())
    }
}


struct HostState {
    host: Host,
    status: HostStatus,
    monitor_data: HashMap<String, MonitoringData>,
    command_results: HashMap<String, CommandResult>,
}

impl HostState {
    fn from_host(host: Host, status: HostStatus) -> Self {
        HostState {
            host: host,
            monitor_data: HashMap::new(),
            command_results: HashMap::new(),
            status: status,
        }
    }

    fn update_status(&mut self) {
        let critical_monitor = &self.monitor_data.iter().find(|(_, data)| {
            // There should always be some monitoring data available at this point.
            data.is_critical && data.values.back().unwrap().criticality == Criticality::Critical
        });

        if let Some((name, _)) = critical_monitor {
            log::debug!("Host is now down since monitor \"{}\" is at critical level", name);
        }

        self.status = match critical_monitor {
            Some(_) => HostStatus::Down,
            None => HostStatus::Up,
        };
    }
}