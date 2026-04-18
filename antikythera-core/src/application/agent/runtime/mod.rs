mod context;
mod execution;
mod instructions;
mod parser;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::config::ToolConfig;
use crate::infrastructure::model::ModelToolDefinition;

pub(super) use super::context::{ServerGuidance, ToolContext, ToolDescriptor};
pub(super) use super::directive::AgentDirective;
pub(super) use super::errors::{AgentError, ToolError};
pub(super) use crate::application::tooling::{ToolInvokeError, ToolServerInterface};
pub(super) use serde_json::{Value, json};
#[derive(Clone)]
pub struct ToolRuntime {
    configs: Vec<ToolConfig>,
    index: HashMap<String, ToolConfig>,
    bridge: Arc<dyn ToolServerInterface>,
    execution_semaphore: Arc<Semaphore>,
}

impl ToolRuntime {
    pub fn new(configs: Vec<ToolConfig>, bridge: Arc<dyn ToolServerInterface>) -> Self {
        let index = configs
            .iter()
            .cloned()
            .map(|cfg| (cfg.name.to_lowercase(), cfg))
            .collect();

        Self {
            configs,
            index,
            bridge,
            execution_semaphore: Arc::new(Semaphore::new(10)), // Default limit to 10 concurrent tools
        }
    }

    pub fn native_tool_definitions(&self, context: &ToolContext) -> Vec<ModelToolDefinition> {
        context
            .tools
            .iter()
            .map(|tool| ModelToolDefinition {
                name: tool.name.clone(),
                description: tool.description.clone(),
                input_schema: tool
                    .input_schema
                    .clone()
                    .unwrap_or_else(|| json!({ "type": "object", "properties": {} })),
            })
            .collect()
    }
}
