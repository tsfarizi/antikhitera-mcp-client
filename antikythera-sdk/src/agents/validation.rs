//! Validation logic for agents and options

use super::orchestrator::GuardrailOptions;
use super::types::{AgentConfig, AgentValidationResult, StreamingOptions};

impl AgentConfig {
    /// Validate agent configuration
    pub fn validate(&self) -> AgentValidationResult {
        let mut errors = Vec::new();

        // ID validation
        if self.id.is_empty() {
            errors.push("Agent ID cannot be empty".to_string());
        }
        if !self
            .id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            errors.push(
                "Agent ID can only contain alphanumeric characters, hyphens, and underscores"
                    .to_string(),
            );
        }

        // Name validation
        if self.name.is_empty() {
            errors.push("Agent name cannot be empty".to_string());
        }

        // Model validation
        if self.model_provider.is_empty() {
            errors.push("Model provider cannot be empty".to_string());
        }
        if self.model.is_empty() {
            errors.push("Model name cannot be empty".to_string());
        }

        // Max steps validation
        if self.max_steps == 0 {
            errors.push("Max steps must be greater than 0".to_string());
        }

        // Temperature validation
        if let Some(temp) = self.temperature
            && !(0.0..=2.0).contains(&temp)
        {
            errors.push("Temperature must be between 0.0 and 2.0".to_string());
        }

        AgentValidationResult {
            valid: errors.is_empty(),
            errors,
            agent_id: self.id.clone(),
        }
    }
}

pub fn validate_guardrail_options_collect(guardrails: &GuardrailOptions) -> Vec<String> {
    let mut errors = Vec::new();

    if let Some(timeout) = &guardrails.timeout
        && timeout.max_timeout_ms == Some(0)
    {
        errors.push("guardrails.timeout.max_timeout_ms must be > 0 if set".to_string());
    }

    if let Some(budget) = &guardrails.budget
        && budget.max_task_steps == Some(0)
    {
        errors.push("guardrails.budget.max_task_steps must be > 0 if set".to_string());
    }

    if let Some(rate_limit) = &guardrails.rate_limit {
        if rate_limit.max_tasks == Some(0) {
            errors.push("guardrails.rate_limit.max_tasks must be > 0 if set".to_string());
        }
        if rate_limit.window_ms == Some(0) {
            errors.push("guardrails.rate_limit.window_ms must be > 0 if set".to_string());
        }
        if rate_limit.max_tasks.is_some() ^ rate_limit.window_ms.is_some() {
            errors.push("guardrails.rate_limit requires both max_tasks and window_ms".to_string());
        }
    }

    errors
}

pub fn validate_streaming_options_collect(options: &StreamingOptions) -> Vec<String> {
    let mut errors = Vec::new();

    if options.max_buffered_events == Some(0) {
        errors.push("max_buffered_events must be > 0 if set".to_string());
    }

    errors
}
