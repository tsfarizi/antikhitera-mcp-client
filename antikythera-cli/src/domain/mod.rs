//! CLI Domain Layer
//!
//! Core domain entities and use cases.
//! Dependencies point inward - domain has NO external dependencies.

pub mod entities;
pub mod use_cases;

pub use entities::*;
pub use use_cases::*;
