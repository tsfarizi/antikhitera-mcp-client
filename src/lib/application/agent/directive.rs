use serde_json::Value;

#[derive(Debug)]
pub enum AgentDirective {
    Final { response: Value },
    CallTool { tool: String, input: Value },
}
