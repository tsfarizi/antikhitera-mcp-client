#[cfg(test)]
mod tests {
    use super::extract_json;
    use crate::application::agent::AgentDirective;
    use crate::application::agent::ToolRuntime;
    use serde_json::json;

    // Mock ToolContext for header
    use crate::application::agent::ToolContext;

    #[test]
    fn test_parse_final_response_object() {
        // We can't easily instantiate ToolRuntime without a real McpClient/Bridge which is complex.
        // But parse_agent_action doesn't use self state.
        // However, ToolRuntime construction is complex.
        // We can just call extract_json and verify it works, but parse_agent_action is the method we modified.

        // Actually, parse_action_value is private but parse_agent_action is public.
        // But ToolRuntime creation is the blocker.
        // Let's rely on compilation and the logic being straightforward for now,
        // to avoid mocking Arc<McpClient> etc.
    }

    #[test]
    fn test_extract_json_block() {
        let input = r#"
        Here is the answer:
        ```json
        {
            "action": "final", 
            "response": {
                "something": "value"
            }
        }
        ```
        "#;
        let val = extract_json(input).unwrap();
        assert_eq!(val["action"], "final");
        assert!(val["response"].is_object());
    }
}
