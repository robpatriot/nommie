//! Error handling for the Nommie backend.

pub mod domain;
pub mod error_code;

pub use domain::DomainError;
pub use error_code::ErrorCode;
