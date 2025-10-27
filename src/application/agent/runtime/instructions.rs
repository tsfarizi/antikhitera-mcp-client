use super::{ToolContext, ToolRuntime, json};

impl ToolRuntime {
    pub fn compose_system_instructions(&self, context: &ToolContext) -> String {
        let mut lines = vec![
            "You are an autonomous assistant that can call tools to solve user requests."
                .to_string(),
            "All responses must be valid JSON without commentary or code fences.".to_string(),
            "When you need to invoke a tool, respond with: {\"action\":\"call_tool\",\"tool\":\"tool_name\",\"input\":{...}}."
                .to_string(),
            "To obtain the list of available tools, call the special tool: {\"action\":\"call_tool\",\"tool\":\"list_tools\"}."
                .to_string(),
            "When you are ready to give the final answer to the user, respond with: {\"action\":\"final\",\"response\":\"...\"}."
                .to_string(),
            "Detect the user's language automatically and answer using that same language unless they explicitly request another language."
                .to_string(),
            "Do not call any translation-related tools; handle language understanding internally."
                .to_string(),
        ];

        if context.is_empty() {
            lines.push("No additional tools are currently configured.".to_string());
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
