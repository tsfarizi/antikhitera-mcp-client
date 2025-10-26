use serde_json::Value;

#[derive(Debug)]
pub enum AgentDirective {
    Final { response: String },
    CallTool { tool: String, input: Value },
}
