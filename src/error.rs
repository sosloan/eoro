use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErrorCategory {
    Configuration,
    InvalidRequest,
    Unavailable,
    Timeout,
    Cancelled,
    Backend,
    Io,
    Serialization,
}

#[derive(Clone, Debug)]
pub struct MixerError {
    pub category: ErrorCategory,
    pub message: String,
}

impl MixerError {
    pub fn new(category: ErrorCategory, message: impl Into<String>) -> Self {
        Self {
            category,
            message: message.into(),
        }
    }
}

impl Display for MixerError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:?}: {}", self.category, self.message)
    }
}

impl std::error::Error for MixerError {}

impl From<std::io::Error> for MixerError {
    fn from(error: std::io::Error) -> Self {
        Self::new(ErrorCategory::Io, error.to_string())
    }
}

impl From<serde_json::Error> for MixerError {
    fn from(error: serde_json::Error) -> Self {
        Self::new(ErrorCategory::Serialization, error.to_string())
    }
}
