use std::collections::{HashMap, HashSet};
use std::time::Instant;
use tracing::{debug, info};

use super::{ServerGuidance, ToolContext, ToolDescriptor, ToolRuntime};
use crate::domain::sanitize::sanitize_for_toml;

impl ToolRuntime {
    pub async fn build_context(&self, input: Option<&str>) -> ToolContext {
        let start_time = Instant::now();
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

        // --- JIT Context Injection & Dynamic Pruning ---
        if let Some(user_input) = input {
            let input_lower = user_input.to_lowercase();
            // Simple keyword matching: count occurrences of words from tool name/description in the input
            let mut scores: HashMap<usize, usize> = HashMap::new();
            for (idx, tool) in context.tools.iter().enumerate() {
                let mut score = 0;
                let keywords: Vec<&str> = tool.name.split(|c: char| !c.is_alphanumeric()).collect();
                for kw in keywords {
                    if !kw.is_empty() && input_lower.contains(&kw.to_lowercase()) {
                        score += 2;
                    }
                }
                if let Some(desc) = &tool.description {
                    let desc_keywords: Vec<&str> =
                        desc.split(|c: char| !c.is_alphanumeric()).collect();
                    for kw in desc_keywords {
                        if kw.len() > 3 && input_lower.contains(&kw.to_lowercase()) {
                            score += 1;
                        }
                    }
                }
                // Base score to ensure some generic tools might stay if tie
                scores.insert(idx, score);
            }

            // Sort by score descending
            let mut indices: Vec<usize> = (0..context.tools.len()).collect();
            indices.sort_by_key(|&idx| std::cmp::Reverse(scores.get(&idx).copied().unwrap_or(0)));

            // Dynamic pruning: Keep top 5 or tools with score > 0
            let max_tools = 5;
            let mut pruned_tools = Vec::new();
            for (i, &idx) in indices.iter().enumerate() {
                let score = scores.get(&idx).copied().unwrap_or(0);
                if i < max_tools || score > 0 {
                    pruned_tools.push(context.tools[idx].clone());
                }
            }
            debug!(
                original_count = context.tools.len(),
                pruned_count = pruned_tools.len(),
                "JIT Context Injection applied"
            );
            context.tools = pruned_tools;
        }

        let elapsed = start_time.elapsed();
        info!(latency_ms = ?elapsed.as_millis(), "Context selection and injection completed");

        context
    }
}
