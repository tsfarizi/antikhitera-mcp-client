use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use antikythera_core::application::agent::multi_agent::{
    AgentProfile, AgentTask, CancellationGuardrail, ErrorKind, ExecutionMode, GuardrailChain,
    MultiAgentOrchestrator, RateLimitGuardrail, TaskGuardrail, TimeoutGuardrail,
};
use antikythera_core::application::client::{ClientConfig, McpClient};
use antikythera_core::infrastructure::model::{
    ModelError, ModelProvider, ModelRequest, ModelResponse,
};
use async_trait::async_trait;

#[derive(Debug)]
struct CountingProvider {
    call_count: Arc<AtomicUsize>,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl ModelProvider for CountingProvider {
    async fn chat(&self, _request: ModelRequest) -> Result<ModelResponse, ModelError> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        Ok(ModelResponse::new(
            r#"{"final_answer":"ok"}"#.to_string(),
            None,
        ))
    }
}

#[derive(Debug, Default)]
struct AlwaysRejectGuardrail;

impl TaskGuardrail for AlwaysRejectGuardrail {
    fn name(&self) -> &'static str {
        "always_reject"
    }

    fn pre_check(
        &self,
        _task: &AgentTask,
        _profile: &AgentProfile,
        _context: &antikythera_core::application::agent::multi_agent::GuardrailContext,
    ) -> Result<(), antikythera_core::application::agent::multi_agent::GuardrailRejection> {
        Err(
            antikythera_core::application::agent::multi_agent::GuardrailRejection::new(
                self.name(),
                antikythera_core::application::agent::multi_agent::GuardrailStage::PreCheck,
                ErrorKind::Permanent,
                "forced rejection",
            ),
        )
    }
}

fn build_orchestrator(call_count: Arc<AtomicUsize>) -> MultiAgentOrchestrator<CountingProvider> {
    let client = Arc::new(McpClient::new(
        CountingProvider { call_count },
        ClientConfig::new("mock", "mock-model"),
    ));

    MultiAgentOrchestrator::new(client, ExecutionMode::Sequential).register_agent(AgentProfile {
        id: "guarded-agent".to_string(),
        name: "Guarded Agent".to_string(),
        role: "general".to_string(),
        system_prompt: Some("You are a guarded test agent".to_string()),
        max_steps: Some(4),
    })
}

// Split into 5 parts for consistent test organization.
include!("guardrails_tests/part_01.rs");
include!("guardrails_tests/part_02.rs");
include!("guardrails_tests/part_03.rs");
include!("guardrails_tests/part_04.rs");
include!("guardrails_tests/part_05.rs");
