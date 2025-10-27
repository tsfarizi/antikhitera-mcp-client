mod context;
mod execution;
mod instructions;
mod parser;

use std::collections::HashMap;
use std::sync::Arc;

use crate::config::ToolConfig;

pub(super) use super::context::{ServerGuidance, ToolContext, ToolDescriptor};
pub(super) use super::directive::AgentDirective;
pub(super) use super::errors::{AgentError, ToolError};
pub(super) use crate::application::tooling::{ToolInvokeError, ToolServerInterface};
pub(super) use serde_json::{Value, json};

pub struct ToolRuntime {
    configs: Vec<ToolConfig>,
    index: HashMap<String, ToolConfig>,
    bridge: Arc<dyn ToolServerInterface>,
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
        }
    }
}