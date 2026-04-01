//! UI domain module for dynamic component system.
//!
//! Components are defined by TOML schema, not hard-coded structs.

mod components;
mod intent;
mod schema;

pub use components::*;
pub use intent::*;
pub use schema::*;
