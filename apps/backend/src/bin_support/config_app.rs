//! Centralized application configuration loaded from environment variables.
//!
//! This module provides a unified `Config` struct that consolidates all
//! configuration values from environment variables, replacing scattered
//! configuration loading throughout the codebase.

use std::env;

use crate::config::db::{DbKind, RuntimeEnv};
use crate::config::email_allowlist::EmailAllowlist;
use crate::error::AppError;

/// Centralized application configuration loaded from environment variables
#[derive(Debug, Clone)]
pub struct Config {
    // Server configuration
    pub host: String,
    pub port: u16,

    // Database configuration (hardcoded to Prod/Postgres for binary)
    pub runtime_env: RuntimeEnv,
    pub db_kind: DbKind,

    // Security configuration
    pub jwt_secret: String,

    // Redis configuration
    pub redis_url: String,

    // Email allowlist configuration (optional)
    pub email_allowlist: Option<EmailAllowlist>,

    // HTTP payload limits
    pub max_json_payload_size: usize,
    pub max_payload_size: usize,

    // Shutdown configuration
    pub websocket_timeout_secs: u64,
}

impl Config {
    /// Load and validate all configuration from environment variables
    pub fn from_env() -> Result<Self, AppError> {
        // Validate required environment variables first
        Self::validate_required_env()?;

        // Server configuration
        let host = env::var("BACKEND_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

        let port_str = env::var("BACKEND_PORT").unwrap_or_else(|_| "3001".to_string());
        let port = port_str.parse::<u16>().map_err(|e| AppError::Config {
            detail: format!(
                "BACKEND_PORT must be a valid port number, got '{}'",
                port_str
            ),
            source: Box::new(e),
        })?;

        // Database configuration (hardcoded for binary)
        let runtime_env = RuntimeEnv::Prod;
        let db_kind = DbKind::Postgres;

        // Security configuration
        let jwt_secret = env::var("BACKEND_JWT_SECRET").map_err(|_| AppError::Config {
            detail: "BACKEND_JWT_SECRET must be set".to_string(),
            source: Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "BACKEND_JWT_SECRET environment variable not found",
            )),
        })?;

        // Redis configuration
        let redis_url = env::var("REDIS_URL").map_err(|_| AppError::Config {
            detail: "REDIS_URL must be set".to_string(),
            source: Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "REDIS_URL environment variable not found",
            )),
        })?;

        // Email allowlist configuration (optional)
        let email_allowlist = EmailAllowlist::from_env();

        // HTTP payload limits (default to 1MB if not specified)
        const DEFAULT_MAX_PAYLOAD_SIZE: usize = 1024 * 1024;
        let max_json_payload_size = env::var("MAX_JSON_PAYLOAD_SIZE")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(DEFAULT_MAX_PAYLOAD_SIZE);
        let max_payload_size = env::var("MAX_PAYLOAD_SIZE")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(DEFAULT_MAX_PAYLOAD_SIZE);

        // Shutdown configuration (hardcoded)
        let websocket_timeout_secs = 2;

        Ok(Config {
            host,
            port,
            runtime_env,
            db_kind,
            jwt_secret,
            redis_url,
            email_allowlist,
            max_json_payload_size,
            max_payload_size,
            websocket_timeout_secs,
        })
    }

    /// Validate critical environment variables at startup
    fn validate_required_env() -> Result<(), AppError> {
        // BACKEND_JWT_SECRET: required, non-empty, minimum length
        match env::var("BACKEND_JWT_SECRET") {
            Ok(secret) if secret.len() >= 32 => {}
            Ok(_) => {
                return Err(AppError::Config {
                    detail: "BACKEND_JWT_SECRET is too short. It should be at least 32 characters for security.".to_string(),
                    source: Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "BACKEND_JWT_SECRET length validation failed",
                    )),
                });
            }
            Err(_) => {
                return Err(AppError::Config {
                    detail: "BACKEND_JWT_SECRET must be set.".to_string(),
                    source: Box::new(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "BACKEND_JWT_SECRET environment variable not found",
                    )),
                });
            }
        }

        // Database environment for production: basic presence checks
        // (detailed validation still happens in db config errors).
        if cfg!(not(test)) {
            if let Ok(runtime_env) = env::var("RUNTIME_ENV") {
                if runtime_env.eq_ignore_ascii_case("prod") {
                    for name in &["PROD_DB", "APP_DB_USER", "APP_DB_PASSWORD"] {
                        if env::var(name).is_err() {
                            return Err(AppError::Config {
                                detail: format!("{name} must be set when RUNTIME_ENV=prod."),
                                source: Box::new(std::io::Error::new(
                                    std::io::ErrorKind::NotFound,
                                    format!("{name} environment variable not found"),
                                )),
                            });
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
