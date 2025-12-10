//! # Application Module
//!
//! This module contains the core application logic for the MCP client.
//!
//! ## Submodules
//!
//! - [`client`] - The main MCP client for communicating with AI models
//! - [`agent`] - Autonomous agent that can use tools and execute multi-step tasks
//! - [`stdio`] - Standard input/output interface for command-line interaction
//! - [`tooling`] - Tool server management and MCP server integration

pub mod agent;
pub mod client;
pub mod stdio;
pub mod tooling;
