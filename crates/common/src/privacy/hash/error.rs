use std::fmt;

#[derive(Debug, Clone)]
pub enum HashError {
    SaltGeneration(String),
    HashComputation(String),
    InvalidInput(String),
    ConfigurationError(String),
}

impl fmt::Display for HashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HashError::SaltGeneration(msg) => write!(f, "Salt generation failed: {}", msg),
            HashError::HashComputation(msg) => write!(f, "Hash computation failed: {}", msg),
            HashError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            HashError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
        }
    }
}

impl std::error::Error for HashError {}

pub type HashResult<T> = Result<T, HashError>;
