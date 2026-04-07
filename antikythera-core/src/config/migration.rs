//! Migration utilities (stub - no TOML migration needed)

/// Check if migration is needed (always false for Postcard-only setup)
pub fn needs_migration() -> bool {
    false
}

/// Get migration status
pub fn migration_status() -> String {
    "Postcard config is up to date".to_string()
}

/// Migrate from TOML to Postcard (no-op)
pub fn migrate_toml_to_postcard() -> Result<(), String> {
    Err("TOML migration is no longer supported. Use Postcard config directly.".to_string())
}
