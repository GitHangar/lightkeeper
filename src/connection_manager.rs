use std::{
    collections::HashMap,
    sync::mpsc,
    sync::Arc,
    sync::Mutex,
    thread,
};
use crate::Host;
use crate::file_handler;
use crate::module::ModuleSpecification;
use crate::module::connection::*;

pub type ResponseHandlerCallback = Box<dyn FnOnce(Vec<Result<ResponseMessage, String>>) + Send + 'static>;
type ConnectorCollection = HashMap<ModuleSpecification, Box<dyn ConnectionModule + Send>>;

pub struct ConnectionManager {
    /// Collection of ConnectionModules that can be shared between threads.
    /// Host as the first hashmap key, connector id as the second.
    connectors: Arc<Mutex<HashMap<String, ConnectorCollection>>>,
    request_sender_prototype: mpsc::Sender<ConnectorRequest>,
    receiver_handle: Option<thread::JoinHandle<()>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel::<ConnectorRequest>();
        let connectors = Arc::new(Mutex::new(HashMap::new()));

        let handle = Self::start_receiving_messages(connectors.clone(), receiver);

        ConnectionManager {
            connectors: connectors,
            request_sender_prototype: sender,
            receiver_handle: Some(handle),
        }
    }

    // Adds a connector but only if a connector with the same ID doesn't exist.
    // This call will block if process_messages() is currently handling a message.
    pub fn add_connector(&mut self, host: &Host, connector: Connector) {
        let mut connectors = self.connectors.lock().unwrap();

        connectors.entry(host.name.clone()).or_insert(HashMap::new());

        if let Some(host_connectors) = connectors.get_mut(&host.name) {
            let module_spec = connector.get_module_spec();

            if !host_connectors.contains_key(&module_spec) {
                log::debug!("[{}] Adding connector {}", host.name, module_spec.id);
                host_connectors.insert(module_spec, connector);
            }
        }
    }

    pub fn new_request_sender(&mut self) -> mpsc::Sender<ConnectorRequest> {
        return self.request_sender_prototype.clone()
    }

    pub fn join(&mut self) {
        self.receiver_handle.take().expect("Thread has already stopped.")
                            .join().unwrap();
    }

    fn start_receiving_messages(
        connectors: Arc<Mutex<HashMap<String, ConnectorCollection>>>,
        receiver: mpsc::Receiver<ConnectorRequest>
    ) -> thread::JoinHandle<()> {

        // TODO: threadpool
        thread::spawn(move || {
            loop {
                let request = match receiver.recv() {
                    Ok(data) => data,
                    Err(error) => {
                        log::error!("Stopped receiver thread: {}", error);
                        return;
                    }
                };


                if request.request_type == RequestType::Exit {
                    log::debug!("Gracefully exiting connection manager thread");
                    return;
                }

                // Requests with no connector dependency are directly executed here.
                if request.connector_id.is_none() {
                    (request.response_handler)(Vec::new());
                    continue;
                }

                let mut connectors = connectors.lock().unwrap();
                let connector = connectors.get_mut(&request.host.name)
                                          .and_then(|connections| connections.get_mut(&request.connector_id.unwrap())).unwrap();

                if !connector.is_connected() {
                    if let Err(error) = connector.connect(&request.host.ip_address) {
                        log::error!("[{}] Error while connecting {}: {}", request.host.name, request.host.ip_address, error);
                        continue;
                    }
                }

                let mut responses = Vec::<Result<ResponseMessage, String>>::new();
                for request_message in request.messages.iter() {

                    let response_result;
                    match &request.request_type {
                        RequestType::Command => {
                            log::debug!("[{}] Processing command: {}", request.host.name, request_message);
                            response_result = connector.send_message(&request_message);
                            if response_result.is_ok() {
                                // Don't continue if any of the commands fail unexpectedly.
                                if response_result.as_ref().unwrap().return_code != 0 {
                                    break;
                                }
                            }
                        },
                        RequestType::Download => {
                            log::debug!("[{}] Downloading file: {}", request.host.name, request_message);
                            response_result = match connector.download_file(&request_message) {
                                Ok(contents) => {
                                    match file_handler::create_file(&request.host, &request_message, contents) {
                                        Ok(file_path) => Ok(ResponseMessage::new(file_path)),
                                        Err(error) => Err(error.to_string()),
                                    }
                                },
                                Err(error) => Err(error.to_string()),
                            }
                        },
                        RequestType::Upload => {
                            log::debug!("[{}] Uploading file: {}", request.host.name, request_message);
                            response_result = match file_handler::read_file(&request_message) {
                                Ok((metadata, contents)) => {
                                    let mut result = connector.upload_file(&metadata.remote_path, contents);
                                    if result.is_ok() {
                                        if metadata.temporary {
                                            log::debug!("removing temporary local file");
                                            result = file_handler::remove_file(&request_message);
                                        }
                                    }

                                    if result.is_ok() {
                                        Ok(ResponseMessage::empty())
                                    }
                                    else {
                                        Err(result.unwrap_err().to_string())
                                    }
                                },
                                Err(error) => Err(error.to_string()),
                            };
                        },
                        RequestType::Exit => panic!(),
                    }

                    responses.push(response_result);

                    for response in responses.iter() {
                        if let Err(error) = response {
                            log::error!("[{}] error while processing request: {}", request.host.name, error);
                            break;
                        }
                    }
                }

                (request.response_handler)(responses);
            }
        })
    }
}

pub struct ConnectorRequest {
    pub connector_id: Option<ModuleSpecification>,
    pub source_id: String,
    pub host: Host,
    pub messages: Vec<String>,
    pub request_type: RequestType,
    pub response_handler: ResponseHandlerCallback,
}

impl ConnectorRequest {
    pub fn exit_token() -> Self {
        ConnectorRequest {
            connector_id: None,
            source_id: String::new(),
            host: Host::new(&String::new(), &String::from("127.0.0.1"), &String::new(), &Vec::new()).unwrap(),
            messages: Vec::new(),
            request_type: RequestType::Exit,
            response_handler: Box::new(|_| ()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RequestType {
    Command,
    Download,
    Upload,
    Exit,
}