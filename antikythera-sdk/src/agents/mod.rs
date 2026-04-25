pub mod types;
pub mod orchestrator;
pub mod validation;
pub mod registry;
pub mod conversions;

pub use types::*;
pub use orchestrator::{
    OrchestratorMonitorSnapshot, OrchestratorOptions, TaskResultDetail, GuardrailOptions,
    RetryConditionOption, TimeoutGuardrailOptions, BudgetGuardrailOptions, RateLimitGuardrailOptions,
    with_hardening_runtime
};
pub use registry::{AgentRegistry, global_agent_registry};
pub use validation::{validate_guardrail_options_collect, validate_streaming_options_collect};

// ============================================================================
// Public SDK API (FFI-friendly wrappers)
// ============================================================================

/// Register a new agent configuration from JSON.
pub fn mcp_register_agent(config_json: &str) -> AgentValidationResult {
    let config: AgentConfig = match serde_json::from_str(config_json) {
        Ok(c) => c,
        Err(e) => {
            return AgentValidationResult {
                valid: false,
                errors: vec![format!("Invalid JSON: {e}")],
                agent_id: String::new(),
            };
        }
    };

    global_agent_registry().register(config)
}

/// Unregister an agent by ID.
pub fn mcp_unregister_agent(id: &str) -> Result<bool, String> {
    global_agent_registry().unregister(id)
}

/// List all registered agents.
pub fn mcp_list_agents() -> Result<Vec<AgentConfig>, String> {
    global_agent_registry().list()
}

/// Get configuration for a specific agent.
pub fn mcp_get_agent(id: &str) -> Result<AgentConfig, String> {
    match global_agent_registry().get(id)? {
        Some(config) => Ok(config),
        None => Err(format!("Agent '{}' not found", id)),
    }
}

/// Get runtime status of all agents.
pub fn mcp_get_agent_status() -> Result<Vec<AgentStatus>, String> {
    global_agent_registry().status_list()
}

/// Validate agent configuration without registering.
pub fn mcp_validate_agent(config_json: &str) -> AgentValidationResult {
    let config: AgentConfig = match serde_json::from_str(config_json) {
        Ok(c) => c,
        Err(e) => {
            return AgentValidationResult {
                valid: false,
                errors: vec![format!("Invalid JSON: {e}")],
                agent_id: String::new(),
            };
        }
    };

    config.validate()
}

/// Export all agents configuration as JSON.
pub fn mcp_export_agents_config() -> Result<String, String> {
    global_agent_registry().export_json()
}

/// Import agents configuration from JSON.
pub fn mcp_import_agents_config(config_json: &str) -> Result<usize, String> {
    global_agent_registry().import_json(config_json)
}

// Compatibility aliases for tests

#[cfg(feature = "multi-agent")]
pub fn configure_hardening(options_json: &str) -> Result<bool, String> {
    mcp_configure_hardening(options_json)
}

#[cfg(feature = "multi-agent")]
pub fn cancel_orchestrator() -> Result<bool, String> {
    mcp_cancel_orchestrator()
}

#[cfg(feature = "multi-agent")]
pub fn get_monitor_snapshot() -> Result<String, String> {
    mcp_get_monitor_snapshot()
}

#[cfg(feature = "multi-agent")]
pub fn task_result_detail(task_result_json: &str) -> Result<String, String> {
    mcp_task_result_detail(task_result_json)
}

#[cfg(feature = "multi-agent")]
pub fn reset_hardening_runtime() -> Result<bool, String> {
    orchestrator::with_hardening_runtime(|state| {
        *state = orchestrator::HardeningRuntimeState::default();
        Ok(true)
    })
}

// Orchestrator SDK methods


/// Return the default [`OrchestratorOptions`] as a JSON string.
pub fn mcp_default_orchestrator_options() -> OrchestratorOptions {
    OrchestratorOptions::default()
}

/// Return default [`StreamingOptions`] as a JSON string.
pub fn mcp_default_streaming_options() -> StreamingOptions {
    StreamingOptions::default()
}

/// Validate a [`StreamingOptions`] JSON string.
pub fn mcp_validate_streaming_options(options_json: &str) -> ValidationReport {
    match serde_json::from_str::<StreamingOptions>(options_json) {
        Ok(opts) => {
            let errors = validate_streaming_options_collect(&opts);
            if errors.is_empty() {
                ValidationReport {
                    valid: true,
                    errors: Vec::new(),
                }
            } else {
                ValidationReport {
                    valid: false,
                    errors,
                }
            }
        }
        Err(e) => ValidationReport {
            valid: false,
            errors: vec![format!("Invalid JSON: {e}")],
        },
    }
}

/// Validate an [`OrchestratorOptions`] JSON string.
pub fn mcp_validate_orchestrator_options(options_json: &str) -> ValidationReport {
    match serde_json::from_str::<OrchestratorOptions>(options_json) {
        Ok(opts) => {
            let mut errors: Vec<String> = Vec::new();
            if opts.max_concurrent_tasks == Some(0) {
                errors.push("max_concurrent_tasks must be > 0 if set".to_string());
            }
            if opts.max_total_steps == Some(0) {
                errors.push("max_total_steps must be > 0 if set".to_string());
            }
            if opts.max_total_tasks == Some(0) {
                errors.push("max_total_tasks must be > 0 if set".to_string());
            }
            errors.extend(validate_guardrail_options_collect(&opts.guardrails));
            if errors.is_empty() {
                ValidationReport {
                    valid: true,
                    errors: Vec::new(),
                }
            } else {
                ValidationReport {
                    valid: false,
                    errors,
                }
            }
        }
        Err(e) => ValidationReport {
            valid: false,
            errors: vec![format!("Invalid JSON: {e}")],
        },
    }
}

#[cfg(feature = "multi-agent")]
pub fn mcp_configure_hardening(options_json: &str) -> Result<bool, String> {
    let options: OrchestratorOptions = serde_json::from_str(options_json)
        .map_err(|e| format!("Invalid OrchestratorOptions JSON: {e}"))?;

    if options.max_concurrent_tasks == Some(0) {
        return Err("max_concurrent_tasks must be > 0 if set".to_string());
    }
    if options.max_total_steps == Some(0) {
        return Err("max_total_steps must be > 0 if set".to_string());
    }
    if options.max_total_tasks == Some(0) {
        return Err("max_total_tasks must be > 0 if set".to_string());
    }

    let errors = validate_guardrail_options_collect(&options.guardrails);
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }

    orchestrator::with_hardening_runtime(|state| {
        state.options = options;
        state.cancelled = false;
        Ok(true)
    })
}

#[cfg(feature = "multi-agent")]
pub fn mcp_cancel_orchestrator() -> Result<bool, String> {
    orchestrator::with_hardening_runtime(|state| {
        state.cancelled = true;
        Ok(true)
    })
}

#[cfg(feature = "multi-agent")]
pub fn mcp_get_monitor_snapshot() -> Result<String, String> {
    orchestrator::with_hardening_runtime(|state| {
        let monitor = if let Some(snapshot) = state.last_budget_snapshot.as_ref() {
            OrchestratorMonitorSnapshot::from(snapshot).with_cancelled(state.cancelled)
        } else {
            OrchestratorMonitorSnapshot {
                max_total_steps: state.options.max_total_steps,
                max_total_tasks: state.options.max_total_tasks,
                max_concurrent_tasks: state.options.max_concurrent_tasks,
                cancelled: state.cancelled,
                ..OrchestratorMonitorSnapshot::default()
            }
        };

        serde_json::to_string(&monitor)
            .map_err(|e| format!("Failed to serialize monitor snapshot: {e}"))
    })
}

#[cfg(feature = "multi-agent")]
pub fn mcp_task_result_detail(task_result_json: &str) -> Result<String, String> {
    use antikythera_core::application::agent::multi_agent::TaskResult;

    let result: TaskResult = serde_json::from_str(task_result_json)
        .map_err(|e| format!("Invalid TaskResult JSON: {e}"))?;
    let detail = TaskResultDetail::from(&result);
    serde_json::to_string(&detail).map_err(|e| format!("Failed to serialize TaskResultDetail: {e}"))
}

#[cfg(feature = "multi-agent")]
pub fn mcp_orchestrator_snapshot(
    budget_snapshot_json: &str,
    cancelled: bool,
) -> Result<OrchestratorMonitorSnapshot, String> {
    use antikythera_core::application::agent::multi_agent::BudgetSnapshot;

    match serde_json::from_str::<BudgetSnapshot>(budget_snapshot_json) {
        Ok(snap) => {
            let monitor = OrchestratorMonitorSnapshot::from(&snap).with_cancelled(cancelled);
            Ok(monitor)
        }
        Err(e) => Err(format!("Invalid BudgetSnapshot JSON: {}", e)),
    }
}
