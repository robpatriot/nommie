//! Test support utilities for backend integration tests
//!
//! This module provides utilities to help set up and configure test environments
//! for the Nommie backend. It includes app builders, factory functions, and
//! other testing helpers.
//!
//! # Quick Start
//!
//! ```rust
//! use backend::infra::state::build_state;
//! use backend::test_support::create_test_app;
//! use backend::config::db::DbProfile;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Build test state with test database
//! let state = build_state().with_db(DbProfile::Test).build().await?;
//!
//! // Create test app with production routes
//! let app = create_test_app(state).with_prod_routes().build().await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Modules
//!
//! - [`app_builder`] - Utilities for building test Actix Web applications
//! - [`factories`] - Factory functions for creating test data
//!
//! # Re-exports
//!
//! - [`create_test_app`] - Main function for creating test app builders

pub mod app_builder;
pub mod factories;

// Re-export the main builders and utilities
pub use app_builder::create_test_app;
