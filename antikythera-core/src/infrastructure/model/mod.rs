//! Model infrastructure module
//!
//! # Architecture — WASM boundary
//!
//! This module defines the LLM provider *contract* (traits + types) that is
//! safe to compile into any target including `wasm32-wasip1` (WASM component).
//!
//! The *implementations* (HTTP clients for Gemini, OpenAI-compatible, and
//! Ollama) are gated behind the `http-providers` feature flag and physically
//! live in `antikythera-cli`.  They are re-exposed here for convenience when
//! the feature is enabled (native / browser-WASM builds).
//!
//! # Structure
//! - `types`   — Request, Response, Error types (always compiled)
//! - `traits`  — `ModelProvider`, `ModelClient` traits (always compiled)
//! - `provider` — `DynamicModelProvider` routing layer (always compiled;
//!                `from_configs` only when `http-providers` is enabled)
//!
//! The following sub-modules are only present when `http-providers` is active:
//! - `adapter`  — Message-format adapters (OpenAI / Gemini / Ollama wire formats)
//! - `clients`  — Concrete HTTP client implementations
//! - `factory`  — `ProviderFactory` + `resolve_api_key` helper

// ── Always-available modules ────────────────────────────────────────────────
pub mod provider;
pub mod traits;
pub mod types;

// ── HTTP provider implementations (native / browser-WASM only) ─────────────
#[cfg(feature = "http-providers")]
pub mod adapter;
#[cfg(feature = "http-providers")]
pub mod clients;
#[cfg(feature = "http-providers")]
pub mod factory;

// ── Re-exports ───────────────────────────────────────────────────────────────
pub use provider::DynamicModelProvider;
pub use traits::ModelProvider;
pub use types::{ModelError, ModelRequest, ModelResponse};
