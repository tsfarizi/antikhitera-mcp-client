use serde_json::Value;

pub(super) fn looks_like_tool_call(content: &str) -> bool {
    fn parse_candidate(text: &str) -> Option<Value> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }
        if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
            return Some(value);
        }
        if trimmed.starts_with("```") {
            let stripped = trimmed
                .trim_start_matches("```json")
                .trim_start_matches("```JSON")
                .trim_start_matches("```");
            if let Some(end) = stripped.rfind("```") {
                let slice = &stripped[..end];
                if let Ok(value) = serde_json::from_str::<Value>(slice.trim()) {
                    return Some(value);
                }
            }
        }
        if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}'))
            && start < end
        {
            let slice = &trimmed[start..=end];
            if let Ok(value) = serde_json::from_str::<Value>(slice) {
                return Some(value);
            }
        }
        None
    }

    fn matches_tool_signature(value: &Value) -> bool {
        match value {
            Value::Object(map) => {
                if let Some(action) = map.get("action").and_then(Value::as_str)
                    && action.eq_ignore_ascii_case("call_tool")
                {
                    return true;
                }
                if map.contains_key("tool_code") {
                    return true;
                }
                if let Some(tool) = map.get("tool")
                    && tool.is_string()
                    && !map.contains_key("response")
                {
                    return true;
                }
                if let Some(tool_calls) = map.get("tool_calls") {
                    return matches_tool_signature(tool_calls);
                }
                false
            }
            Value::Array(items) => items.iter().any(matches_tool_signature),
            _ => false,
        }
    }

    parse_candidate(content)
        .map(|value| matches_tool_signature(&value))
        .unwrap_or(false)
}
