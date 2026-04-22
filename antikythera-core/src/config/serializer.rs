use super::AppConfig;
use super::tool::ToolConfig;

/// Convert AppConfig to TOML string representation
pub fn to_raw_toml_string(config: &AppConfig) -> String {
    render_config_raw(
        config.system_prompt.as_deref(),
        config.prompt_template(),
        &config.tools,
    )
}

fn render_config_raw(
    system_prompt: Option<&str>,
    prompt_template: &str,
    tools: &[ToolConfig],
) -> String {
    let escape = |value: &str| value.replace('"', "\\\"");
    let mut raw = String::new();

    if let Some(system_prompt) = system_prompt {
        raw.push_str(&format!(
            "system_prompt = \"{}\"\n\n",
            escape(system_prompt),
        ));
    }

    raw.push_str("prompt_template = \"\"\"\n");
    raw.push_str(prompt_template);
    if !prompt_template.ends_with('\n') {
        raw.push('\n');
    }
    raw.push_str("\"\"\"\n");

    if !tools.is_empty() {
        raw.push_str("tools = [\n");
        for tool in tools {
            match &tool.description {
                Some(desc) => raw.push_str(&format!(
                    "    {{ name = \"{}\", description = \"{}\" }},\n",
                    escape(&tool.name),
                    escape(desc),
                )),
                None => raw.push_str(&format!("    \"{}\",\n", escape(&tool.name))),
            }
        }
        raw.push_str("]\n");
    }

    raw
}
