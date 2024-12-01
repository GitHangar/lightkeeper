use strum_macros::Display;

use std::error;
use std::fmt;


#[derive(Clone, Debug)]
pub struct LkError {
    pub source_id: String,
    pub kind: ErrorKind,
    pub message: String,
    pub parameter: Option<String>,
}

impl LkError {
    pub fn new<Stringable: ToString>(kind: ErrorKind, message: Stringable) -> LkError {
        LkError {
            source_id: String::new(),
            kind: kind,
            message: message.to_string(),
            parameter: None,
        }
    }

    pub fn not_implemented() -> LkError {
        LkError {
            source_id: String::new(),
            kind: ErrorKind::NotImplemented,
            message: "Not implemented".to_string(),
            parameter: None,
        }
    }

    pub fn unsupported_platform() -> LkError {
        LkError {
            source_id: String::new(),
            kind: ErrorKind::UnsupportedPlatform,
            message: "Unsupported platform".to_string(),
            parameter: None,
        }
    }

    pub fn host_key_unverified<Stringable: ToString>(source_id: Stringable, message: Stringable, key_id: Stringable) -> LkError {
        LkError {
            source_id: source_id.to_string(),
            kind: ErrorKind::HostKeyNotVerified,
            message: message.to_string(),
            parameter: Some(key_id.to_string())
        }
    }

    pub fn config<Stringable: ToString>(message: Stringable) -> LkError {
        LkError::new(ErrorKind::InvalidConfig, message)
    }

    pub fn other<Stringable: ToString>(message: Stringable) -> LkError {
        LkError::new(ErrorKind::Other, message)
    }

    pub fn other_p<Stringable: ToString>(message: &str, parameter: Stringable) -> LkError {
        LkError {
            kind: ErrorKind::Other,
            source_id: String::new(),
            message: format!("{}: {}", message, parameter.to_string()),
            parameter: Some(parameter.to_string())
        }
    }

    pub fn set_source<Stringable: ToString>(mut self, source: Stringable) -> LkError {
        self.source_id = source.to_string();
        self
    }
}

impl fmt::Display for LkError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.source_id.is_empty() {
            write!(f, "{}", self.message)
        }
        else {
            write!(f, "{}: {}", self.source_id, self.message)
        }
    }
}

impl error::Error for LkError {
}

impl From<std::io::Error> for LkError {
    fn from(error: std::io::Error) -> Self {
        LkError::new(ErrorKind::Other, error)
    }
}

impl From<String> for LkError {
    fn from(error: String) -> Self {
        LkError::new(ErrorKind::Other, error)
    }
}

#[derive(Clone, Default, Debug, Display, PartialEq, Eq)]
pub enum ErrorKind {
    /// The requested operation is not supported on the platform.
    UnsupportedPlatform,
    /// Connection timed out, was refused or disconnected.
    ConnectionFailed,
    /// Encountered an unknown host key.
    HostKeyNotVerified,
    /// Not implemented.
    NotImplemented,
    /// Error in configuration files.
    InvalidConfig,
    #[default]
    /// Other unspecified error.
    Other,
}