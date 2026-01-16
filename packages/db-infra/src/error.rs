use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbInfraError {
    #[error("Configuration error: {message}")]
    Config { message: String },
}
