use super::AppConfig;
use super::provider::ModelProviderConfig;
use super::tool::ToolConfig;

/// Convert AppConfig to TOML string representation
pub fn to_raw_toml_string(config: &AppConfig) -> String {
    render_config_raw(
        &config.default_provider,
        &config.model,
        config.system_prompt.as_deref(),
        &config.prompt_template(),
        &config.tools,
        &config.providers,
    )
}

fn render_config_raw(
    default_provider: &str,
    model: &str,
    system_prompt: Option<&str>,
    prompt_template: &str,
    tools: &[ToolConfig],
    providers: &[ModelProviderConfig],
) -> String {
    let escape = |value: &str| value.replace('"', "\\\"");
    let mut raw = format!(
        "default_provider = \"{}\"\nmodel = \"{}\"\n\n",
        escape(default_provider),
        escape(model)
    );

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

    if !providers.is_empty() {
        raw.push('\n');
        for provider in providers {
            raw.push_str("[[providers]]\n");
            raw.push_str(&format!("id = \"{}\"\n", escape(&provider.id)));
            raw.push_str(&format!("type = \"{}\"\n", escape(&provider.provider_type)));
            raw.push_str(&format!("endpoint = \"{}\"\n", escape(&provider.endpoint)));
            if let Some(api_key) = &provider.api_key {
                raw.push_str(&format!("api_key = \"{}\"\n", escape(api_key)));
            }
            raw.push_str("models = [\n");
            for model_info in &provider.models {
                match &model_info.display_name {
                    Some(label) => raw.push_str(&format!(
                        "    {{ name = \"{}\", display_name = \"{}\" }},\n",
                        escape(&model_info.name),
                        escape(label),
                    )),
                    None => raw.push_str(&format!(
                        "    {{ name = \"{}\" }},\n",
                        escape(&model_info.name),
                    )),
                }
            }
            raw.push_str("]\n\n");
        }
    }

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
