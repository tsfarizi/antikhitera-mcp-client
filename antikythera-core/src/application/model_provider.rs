//! Application-facing model provider port.
//!
//! This keeps application modules decoupled from concrete infrastructure
//! module paths while preserving the existing trait contract.

pub use crate::infrastructure::model::ModelProvider;
