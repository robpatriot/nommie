//! Authorization module for admin capabilities.
//!
//! Provides capability-based authorization centered on Principal and AdminCapability.

mod capability;
mod policy;
mod principal;

pub use capability::AdminCapability;
pub use policy::has_capability;
pub use principal::Principal;
