//! SDK-level conversions

#[cfg(feature = "multi-agent")]
use super::orchestrator::{
    GuardrailOptions, OrchestratorMonitorSnapshot, OrchestratorOptions, TaskResultDetail,
};
use super::types::{StreamingModeOption, StreamingOptions};

#[cfg(feature = "multi-agent")]
use antikythera_core::application::agent::multi_agent::{
    BudgetGuardrail, BudgetSnapshot, CancellationGuardrail, GuardrailChain, OrchestratorBudget,
    RateLimitGuardrail, RetryCondition, TaskResult, TimeoutGuardrail,
};

#[cfg(feature = "sdk-core")]
impl From<StreamingModeOption> for antikythera_core::StreamingMode {
    fn from(value: StreamingModeOption) -> Self {
        match value {
            StreamingModeOption::Token => antikythera_core::StreamingMode::Token,
            StreamingModeOption::Event => antikythera_core::StreamingMode::Event,
            StreamingModeOption::Mixed => antikythera_core::StreamingMode::Mixed,
        }
    }
}

#[cfg(feature = "sdk-core")]
impl StreamingOptions {
    /// Convert SDK streaming options into core streaming request.
    pub fn to_streaming_request(&self) -> antikythera_core::StreamingRequest {
        antikythera_core::StreamingRequest {
            mode: self.mode.into(),
            include_final_response: self.include_final_response,
            max_buffered_events: self.max_buffered_events,
            phase2: None,
        }
    }
}

#[cfg(feature = "multi-agent")]
impl From<&OrchestratorOptions> for OrchestratorBudget {
    fn from(opts: &OrchestratorOptions) -> Self {
        let mut budget = OrchestratorBudget::new();
        if let Some(steps) = opts.max_total_steps {
            budget = budget.with_max_total_steps(steps);
        }
        if let Some(tasks) = opts.max_total_tasks {
            budget = budget.with_max_total_tasks(tasks);
        }
        if let Some(concurrency) = opts.max_concurrent_tasks {
            budget = budget.with_max_concurrent_tasks(concurrency);
        }
        budget
    }
}

#[cfg(feature = "multi-agent")]
impl From<super::orchestrator::RetryConditionOption> for RetryCondition {
    fn from(opt: super::orchestrator::RetryConditionOption) -> Self {
        match opt {
            super::orchestrator::RetryConditionOption::Always => RetryCondition::Always,
            super::orchestrator::RetryConditionOption::OnTransient => RetryCondition::OnTransient,
            super::orchestrator::RetryConditionOption::Never => RetryCondition::Never,
        }
    }
}

#[cfg(feature = "multi-agent")]
impl GuardrailOptions {
    /// Convert SDK guardrail options into a core guardrail chain.
    pub fn to_guardrail_chain(&self) -> GuardrailChain {
        let mut chain = GuardrailChain::new();

        if let Some(timeout) = &self.timeout
            && let Some(max_timeout_ms) = timeout.max_timeout_ms.filter(|value| *value > 0)
        {
            let mut guardrail = TimeoutGuardrail::new(max_timeout_ms);
            if timeout.require_explicit_timeout {
                guardrail = guardrail.require_timeout();
            }
            chain.push(std::sync::Arc::new(guardrail));
        }

        if let Some(budget) = &self.budget {
            let mut guardrail = BudgetGuardrail::new();
            if let Some(max_task_steps) = budget.max_task_steps.filter(|value| *value > 0) {
                guardrail = guardrail.with_max_task_steps(max_task_steps);
            }
            if budget.require_explicit_budget {
                guardrail = guardrail.require_explicit_budget();
            }
            if budget.allow_exhausted_orchestrator {
                guardrail = guardrail.allow_exhausted_orchestrator();
            }
            if budget.max_task_steps.is_some()
                || budget.require_explicit_budget
                || budget.allow_exhausted_orchestrator
            {
                chain.push(std::sync::Arc::new(guardrail));
            }
        }

        if let Some(rate_limit) = &self.rate_limit
            && let (Some(max_tasks), Some(window_ms)) = (
                rate_limit.max_tasks.filter(|value| *value > 0),
                rate_limit.window_ms.filter(|value| *value > 0),
            )
        {
            chain.push(std::sync::Arc::new(RateLimitGuardrail::new(
                max_tasks, window_ms,
            )));
        }

        if self.cancellation {
            chain.push(std::sync::Arc::new(CancellationGuardrail::new()));
        }

        chain
    }
}

#[cfg(feature = "multi-agent")]
impl From<&BudgetSnapshot> for OrchestratorMonitorSnapshot {
    fn from(snap: &BudgetSnapshot) -> Self {
        Self {
            consumed_steps: snap.consumed_steps,
            dispatched_tasks: snap.dispatched_tasks,
            max_total_steps: snap.max_total_steps,
            max_total_tasks: snap.max_total_tasks,
            max_concurrent_tasks: snap.max_concurrent_tasks,
            step_budget_exhausted: snap
                .max_total_steps
                .is_some_and(|max| snap.consumed_steps >= max),
            task_budget_exhausted: snap
                .max_total_tasks
                .is_some_and(|max| snap.dispatched_tasks >= max),
            cancelled: false,
        }
    }
}

#[cfg(feature = "multi-agent")]
impl From<&TaskResult> for TaskResultDetail {
    fn from(result: &TaskResult) -> Self {
        let routing = result.metadata.routing_decision.as_ref();
        Self {
            error_kind: result.error_kind.as_ref().map(|k| {
                serde_json::to_value(k)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_else(|| format!("{:?}", k))
            }),
            is_transient: result.is_transient(),
            router_name: routing.map(|r| r.router_name.clone()),
            selected_agent_id: routing.map(|r| r.selected_agent_id.clone()),
            candidates_considered: routing.map(|r| r.candidates_considered),
            routing_reason: routing.and_then(|r| r.reason.clone()),
            concurrency_wait_ms: result.metadata.concurrency_wait_ms,
            budget_exhausted: result.metadata.budget_exhausted,
            guardrail_name: result.metadata.guardrail_name.clone(),
            guardrail_stage: result.metadata.guardrail_stage.clone(),
        }
    }
}
