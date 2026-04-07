//! JSON Schema Validation Module
//!
//! Provides schema definition, validation, and auto-retry for JSON output.
//!
//! ## Architecture
//!
//! ```text
//! json_schema/
//! ├── types.rs      # Schema type definitions (nested structures supported)
//! ├── validator.rs  # Validation engine with retry logic
//! └── ffi.rs        # FFI bindings for CLI and external languages
//! ```
//!
//! ## Usage
//!
//! 1. Register a schema
//! 2. Get schema prompt to append to LLM prompt
//! 3. Validate LLM response
//! 4. Auto-retry with error feedback on failure

pub mod types;
pub mod validator;
pub mod ffi;

// Re-export main types for convenience
pub use types::{JsonSchema, SchemaType, SchemaField, ValidationError};
pub use validator::{JsonValidator, ValidationResult, RetryManager};
