use super::{ToolContext, ToolRuntime, json};
use crate::config::PromptsConfig;

impl ToolRuntime {
    pub fn compose_system_instructions(
        &self,
        context: &ToolContext,
        prompts: &PromptsConfig,
    ) -> String {
        let mut lines = vec![
            prompts.template().to_string(),
            prompts.agent_instructions().to_string(),
            prompts.ui_instructions().to_string(),
            prompts.language_instructions().to_string(),
        ];

        if context.is_empty() {
            lines.push(prompts.no_tools_guidance().to_string());
            return lines.join(" ");
        }

        for guidance in &context.servers {
            lines.push(format!(
                "Server '{}' guidance: {}",
                guidance.name, guidance.instruction
            ));
        }

        if !context.tools.is_empty() {
            lines.push("Configured tools:".to_string());
            for descriptor in &context.tools {
                let mut line = format!("- {}", descriptor.name);
                if let Some(server) = &descriptor.server {
                    line.push_str(&format!(" (server: {})", server));
                }
                if let Some(description) = &descriptor.description {
                    line.push_str(&format!(": {}", description));
                }
                if let Some(schema) = &descriptor.input_schema {
                    let compact = serde_json::to_string(schema).unwrap_or_default();
                    line.push_str(&format!(". Input schema: {}", compact));
                }
                lines.push(line);
            }
        }

        lines.join(" ")
    }

    pub fn initial_user_prompt(&self, prompt: String, context: &ToolContext) -> String {
        let mut payload = json!({
            "action": "user_request",
            "prompt": prompt,
        });

        if !context.is_empty() {
            if let Some(map) = payload.as_object_mut() {
                if let Ok(value) = serde_json::to_value(context) {
                    map.insert("tool_context".to_string(), value);
                }
            }
        }

        payload.to_string()
    }
}
