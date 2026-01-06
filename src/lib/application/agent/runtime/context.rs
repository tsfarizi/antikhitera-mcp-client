use std::collections::HashSet;

use super::{ServerGuidance, ToolContext, ToolDescriptor, ToolRuntime};
use crate::domain::sanitize::sanitize_for_toml;

impl ToolRuntime {
    pub async fn build_context(&self) -> ToolContext {
        if self.configs.is_empty() {
            return ToolContext::default();
        }

        let mut context = ToolContext::default();
        let mut seen_servers = HashSet::new();

        for tool in &self.configs {
            if let Some(server_name) = tool.server.as_deref() {
                if seen_servers.insert(server_name.to_string()) {
                    if let Some(instruction) = self.bridge.server_instructions(server_name).await {
                        context.servers.push(ServerGuidance {
                            name: server_name.to_string(),
                            instruction,
                        });
                    }
                }
            }

            let mut descriptor = ToolDescriptor {
                name: tool.name.clone(),
                description: tool.description.clone(),
                server: tool.server.clone(),
                input_schema: None,
            };

            if let Some(server_name) = tool.server.as_deref() {
                if let Some(metadata) = self.bridge.tool_metadata(server_name, &tool.name).await {
                    if !metadata.name.is_empty() {
                        descriptor.name = metadata.name;
                    }
                    if let Some(remote_desc) = metadata.description {
                        // Sanitize remote description to ensure TOML compatibility
                        let sanitized_desc = sanitize_for_toml(&remote_desc);

                        descriptor.description = match descriptor.description {
                            Some(existing)
                                if existing.trim().is_empty()
                                    || existing.trim() == sanitized_desc.trim() =>
                            {
                                Some(sanitized_desc)
                            }
                            Some(existing) => {
                                // Merge remote and local descriptions
                                Some(format!("{} {}", sanitized_desc.trim(), existing.trim()))
                            }
                            None => Some(sanitized_desc),
                        };
                    }
                    descriptor.input_schema = metadata.input_schema;
                }
            }

            context.tools.push(descriptor);
        }

        context
    }
}
