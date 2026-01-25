use super::{ToolContext, ToolRuntime, json};
use crate::config::PromptsConfig;

impl ToolRuntime {
    pub fn compose_system_instructions(
        &self,
        context: &ToolContext,
        prompts: &PromptsConfig,
    ) -> String {
        let mut lines = vec![
            prompts.agent_instructions().to_string(),
            prompts.ui_instructions().to_string(),
            format!(
                "Available UI Components (UI Catalog):\n{}",
                self.build_dynamic_component_catalog()
            ),
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

    /// Build detailed dynamic component catalog for system prompt.
    fn build_dynamic_component_catalog(&self) -> String {
        let mut components: Vec<_> = self.ui_schema.components.keys().cloned().collect();
        components.sort();

        let mut entries = Vec::new();
        for name in &components {
            if let Some(schema) = self.ui_schema.components.get(name) {
                let desc = schema.description.as_deref().unwrap_or("No description");
                let mut info = format!("- '{}': {}", name, desc);

                if !schema.required_fields.is_empty() {
                    info.push_str(&format!(
                        " (Required props: {})",
                        schema.required_fields.join(", ")
                    ));
                }

                if schema.is_container {
                    info.push_str(" [Container - supports children]");
                }

                if let Some(mapping) = &schema.mapping {
                    let mapped_fields: Vec<_> = mapping.keys().collect();
                    info.push_str(&format!(
                        " [Supports Late-Binding for: {}]",
                        mapped_fields
                            .iter()
                            .map(|s| s.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }

                entries.push(info);
            }
        }

        entries.join("\n")
    }
}
