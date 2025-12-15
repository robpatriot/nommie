//! Error handling for the Nommie backend.

pub mod domain;
pub mod error_code;

#[cfg(test)]
mod tests_error_codes_unique;
#[cfg(test)]
mod tests_error_mapping;

pub use error_code::ErrorCode;
