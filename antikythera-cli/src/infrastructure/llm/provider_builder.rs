//! Provider builder — CLI's primary entry-point for constructing a `DynamicModelProvider`
//!
//! This module now exists only as a compatibility surface.
//!
//! Direct model API calls are no longer created inside this repository. The
//! embedding host is responsible for invoking the model and passing the result
//! back into the framework through the host/WASM boundary.

use antikythera_core::config::ModelProviderConfig;
use antikythera_core::infrastructure::model::{DynamicModelProvider, ModelError};

/// Build a [`DynamicModelProvider`] from a slice of provider configurations.
pub fn build_provider_from_configs(
    _configs: &[ModelProviderConfig],
) -> Result<DynamicModelProvider, ModelError> {
    Err(ModelError::unsupported(
        "Repo ini tidak lagi membangun client HTTP untuk model. Host FFI harus melakukan pemanggilan model dan mengirim balik pesan/riwayat yang dibutuhkan ke framework.",
    ))
}
