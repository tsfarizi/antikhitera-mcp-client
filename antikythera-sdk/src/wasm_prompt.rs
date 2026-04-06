//! WASM bindings for prompt template management
//!
//! This module provides JavaScript/TypeScript compatible bindings
//! for managing all prompt templates in the configuration.

use wasm_bindgen::prelude::*;
use antikythera_core::config::app::PromptsConfig;
use antikythera_core::config::wizard::generators::model;
use antikythera_core::constants::{CONFIG_PATH, MODEL_CONFIG_PATH};
use std::path::Path;
use std::fs;

/// Prompt template manager for WASM
#[wasm_bindgen]
pub struct PromptManager;

#[wasm_bindgen]
impl PromptManager {
    /// Get the current prompt template
    #[wasm_bindgen(js_name = getTemplate)]
    pub fn get_template() -> Result<String, JsValue> {
        let config = load_prompts_config()?;
        Ok(config.template().to_string())
    }

    /// Update the prompt template
    #[wasm_bindgen(js_name = updateTemplate)]
    pub fn update_template(template: &str) -> Result<(), JsValue> {
        model::update_prompt_template(template)
            .map_err(|e| JsValue::from_str(&format!("Failed to update template: {}", e)))
    }

    /// Reset the prompt template to default
    #[wasm_bindgen(js_name = resetTemplate)]
    pub fn reset_template() -> Result<(), JsValue> {
        let default = PromptsConfig::default_template();
        model::update_prompt_template(default)
            .map_err(|e| JsValue::from_str(&format!("Failed to reset template: {}", e)))
    }

    /// Get tool guidance prompt
    #[wasm_bindgen(js_name = getToolGuidance)]
    pub fn get_tool_guidance() -> Result<String, JsValue> {
        let config = load_prompts_config()?;
        Ok(config.tool_guidance().to_string())
    }

    /// Update tool guidance prompt
    #[wasm_bindgen(js_name = updateToolGuidance)]
    pub fn update_tool_guidance(guidance: &str) -> Result<(), JsValue> {
        model::update_tool_guidance(guidance)
            .map_err(|e| JsValue::from_str(&format!("Failed to update tool guidance: {}", e)))
    }

    /// Reset tool guidance to default
    #[wasm_bindgen(js_name = resetToolGuidance)]
    pub fn reset_tool_guidance() -> Result<(), JsValue> {
        let default = PromptsConfig::default_tool_guidance();
        model::update_tool_guidance(default)
            .map_err(|e| JsValue::from_str(&format!("Failed to reset tool guidance: {}", e)))
    }

    /// Get fallback guidance prompt
    #[wasm_bindgen(js_name = getFallbackGuidance)]
    pub fn get_fallback_guidance() -> Result<String, JsValue> {
        let config = load_prompts_config()?;
        Ok(config.fallback_guidance().to_string())
    }

    /// Update fallback guidance prompt
    #[wasm_bindgen(js_name = updateFallbackGuidance)]
    pub fn update_fallback_guidance(guidance: &str) -> Result<(), JsValue> {
        model::update_fallback_guidance(guidance)
            .map_err(|e| JsValue::from_str(&format!("Failed to update fallback guidance: {}", e)))
    }

    /// Reset fallback guidance to default
    #[wasm_bindgen(js_name = resetFallbackGuidance)]
    pub fn reset_fallback_guidance() -> Result<(), JsValue> {
        let default = PromptsConfig::default_fallback_guidance();
        model::update_fallback_guidance(default)
            .map_err(|e| JsValue::from_str(&format!("Failed to reset fallback guidance: {}", e)))
    }

    /// Get JSON retry message
    #[wasm_bindgen(js_name = getJsonRetryMessage)]
    pub fn get_json_retry_message() -> Result<String, JsValue> {
        let config = load_prompts_config()?;
        Ok(config.json_retry_message().to_string())
    }

    /// Update JSON retry message
    #[wasm_bindgen(js_name = updateJsonRetryMessage)]
    pub fn update_json_retry_message(message: &str) -> Result<(), JsValue> {
        model::update_json_retry_message(message)
            .map_err(|e| JsValue::from_str(&format!("Failed to update JSON retry message: {}", e)))
    }

    /// Reset JSON retry message to default
    #[wasm_bindgen(js_name = resetJsonRetryMessage)]
    pub fn reset_json_retry_message() -> Result<(), JsValue> {
        let default = PromptsConfig::default_json_retry_message();
        model::update_json_retry_message(default)
            .map_err(|e| JsValue::from_str(&format!("Failed to reset JSON retry message: {}", e)))
    }

    /// Get tool result instruction
    #[wasm_bindgen(js_name = getToolResultInstruction)]
    pub fn get_tool_result_instruction() -> Result<String, JsValue> {
        let config = load_prompts_config()?;
        Ok(config.tool_result_instruction().to_string())
    }

    /// Update tool result instruction
    #[wasm_bindgen(js_name = updateToolResultInstruction)]
    pub fn update_tool_result_instruction(instruction: &str) -> Result<(), JsValue> {
        model::update_tool_result_instruction(instruction)
            .map_err(|e| JsValue::from_str(&format!("Failed to update tool result instruction: {}", e)))
    }

    /// Reset tool result instruction to default
    #[wasm_bindgen(js_name = resetToolResultInstruction)]
    pub fn reset_tool_result_instruction() -> Result<(), JsValue> {
        let default = PromptsConfig::default_tool_result_instruction();
        model::update_tool_result_instruction(default)
            .map_err(|e| JsValue::from_str(&format!("Failed to reset tool result instruction: {}", e)))
    }

    /// Get agent instructions
    #[wasm_bindgen(js_name = getAgentInstructions)]
    pub fn get_agent_instructions() -> Result<String, JsValue> {
        let config = load_prompts_config()?;
        Ok(config.agent_instructions().to_string())
    }

    /// Update agent instructions
    #[wasm_bindgen(js_name = updateAgentInstructions)]
    pub fn update_agent_instructions(instructions: &str) -> Result<(), JsValue> {
        model::update_prompts_field("agent_instructions", instructions, true)
            .map_err(|e| JsValue::from_str(&format!("Failed to update agent instructions: {}", e)))
    }

    /// Reset agent instructions to default
    #[wasm_bindgen(js_name = resetAgentInstructions)]
    pub fn reset_agent_instructions() -> Result<(), JsValue> {
        let default = PromptsConfig::default_agent_instructions();
        model::update_prompts_field("agent_instructions", default, true)
            .map_err(|e| JsValue::from_str(&format!("Failed to reset agent instructions: {}", e)))
    }

    /// Get UI instructions
    #[wasm_bindgen(js_name = getUiInstructions)]
    pub fn get_ui_instructions() -> Result<String, JsValue> {
        let config = load_prompts_config()?;
        Ok(config.ui_instructions().to_string())
    }

    /// Update UI instructions
    #[wasm_bindgen(js_name = updateUiInstructions)]
    pub fn update_ui_instructions(instructions: &str) -> Result<(), JsValue> {
        model::update_prompts_field("ui_instructions", instructions, true)
            .map_err(|e| JsValue::from_str(&format!("Failed to update UI instructions: {}", e)))
    }

    /// Reset UI instructions to default
    #[wasm_bindgen(js_name = resetUiInstructions)]
    pub fn reset_ui_instructions() -> Result<(), JsValue> {
        let default = PromptsConfig::default_ui_instructions();
        model::update_prompts_field("ui_instructions", default, true)
            .map_err(|e| JsValue::from_str(&format!("Failed to reset UI instructions: {}", e)))
    }

    /// Get language instructions
    #[wasm_bindgen(js_name = getLanguageInstructions)]
    pub fn get_language_instructions() -> Result<String, JsValue> {
        let config = load_prompts_config()?;
        Ok(config.language_instructions().to_string())
    }

    /// Update language instructions
    #[wasm_bindgen(js_name = updateLanguageInstructions)]
    pub fn update_language_instructions(instructions: &str) -> Result<(), JsValue> {
        model::update_prompts_field("language_instructions", instructions, true)
            .map_err(|e| JsValue::from_str(&format!("Failed to update language instructions: {}", e)))
    }

    /// Reset language instructions to default
    #[wasm_bindgen(js_name = resetLanguageInstructions)]
    pub fn reset_language_instructions() -> Result<(), JsValue> {
        let default = PromptsConfig::default_language_instructions();
        model::update_prompts_field("language_instructions", default, true)
            .map_err(|e| JsValue::from_str(&format!("Failed to reset language instructions: {}", e)))
    }

    /// Get agent max steps error message
    #[wasm_bindgen(js_name = getAgentMaxStepsError)]
    pub fn get_agent_max_steps_error() -> Result<String, JsValue> {
        let config = load_prompts_config()?;
        Ok(config.agent_max_steps_error().to_string())
    }

    /// Update agent max steps error message
    #[wasm_bindgen(js_name = updateAgentMaxStepsError)]
    pub fn update_agent_max_steps_error(message: &str) -> Result<(), JsValue> {
        model::update_prompts_field("agent_max_steps_error", message, false)
            .map_err(|e| JsValue::from_str(&format!("Failed to update agent max steps error: {}", e)))
    }

    /// Reset agent max steps error to default
    #[wasm_bindgen(js_name = resetAgentMaxStepsError)]
    pub fn reset_agent_max_steps_error() -> Result<(), JsValue> {
        let default = PromptsConfig::default_agent_max_steps_error();
        model::update_prompts_field("agent_max_steps_error", default, false)
            .map_err(|e| JsValue::from_str(&format!("Failed to reset agent max steps error: {}", e)))
    }

    /// Get no tools guidance
    #[wasm_bindgen(js_name = getNoToolsGuidance)]
    pub fn get_no_tools_guidance() -> Result<String, JsValue> {
        let config = load_prompts_config()?;
        Ok(config.no_tools_guidance().to_string())
    }

    /// Update no tools guidance
    #[wasm_bindgen(js_name = updateNoToolsGuidance)]
    pub fn update_no_tools_guidance(guidance: &str) -> Result<(), JsValue> {
        model::update_prompts_field("no_tools_guidance", guidance, false)
            .map_err(|e| JsValue::from_str(&format!("Failed to update no tools guidance: {}", e)))
    }

    /// Reset no tools guidance to default
    #[wasm_bindgen(js_name = resetNoToolsGuidance)]
    pub fn reset_no_tools_guidance() -> Result<(), JsValue> {
        let default = PromptsConfig::default_no_tools_guidance();
        model::update_prompts_field("no_tools_guidance", default, false)
            .map_err(|e| JsValue::from_str(&format!("Failed to reset no tools guidance: {}", e)))
    }

    /// Get all prompts as JSON object
    #[wasm_bindgen(js_name = getAllPrompts)]
    pub fn get_all_prompts() -> Result<String, JsValue> {
        let config = load_prompts_config()?;
        let json = serde_json::json!({
            "template": config.template(),
            "tool_guidance": config.tool_guidance(),
            "fallback_guidance": config.fallback_guidance(),
            "json_retry_message": config.json_retry_message(),
            "tool_result_instruction": config.tool_result_instruction(),
            "agent_instructions": config.agent_instructions(),
            "ui_instructions": config.ui_instructions(),
            "language_instructions": config.language_instructions(),
            "agent_max_steps_error": config.agent_max_steps_error(),
            "no_tools_guidance": config.no_tools_guidance()
        });
        Ok(json.to_string())
    }

    /// Reset all prompts to defaults
    #[wasm_bindgen(js_name = resetAllPrompts)]
    pub fn reset_all_prompts() -> Result<(), JsValue> {
        let config = PromptsConfig::default();
        
        model::update_prompt_template(config.template())
            .map_err(|e| JsValue::from_str(&format!("Failed to reset template: {}", e)))?;
        
        model::update_tool_guidance(config.tool_guidance())
            .map_err(|e| JsValue::from_str(&format!("Failed to reset tool guidance: {}", e)))?;
        
        model::update_fallback_guidance(config.fallback_guidance())
            .map_err(|e| JsValue::from_str(&format!("Failed to reset fallback guidance: {}", e)))?;
        
        model::update_json_retry_message(config.json_retry_message())
            .map_err(|e| JsValue::from_str(&format!("Failed to reset JSON retry message: {}", e)))?;
        
        model::update_tool_result_instruction(config.tool_result_instruction())
            .map_err(|e| JsValue::from_str(&format!("Failed to reset tool result instruction: {}", e)))?;
        
        model::update_prompts_field("agent_instructions", config.agent_instructions(), true)
            .map_err(|e| JsValue::from_str(&format!("Failed to reset agent instructions: {}", e)))?;
        
        model::update_prompts_field("ui_instructions", config.ui_instructions(), true)
            .map_err(|e| JsValue::from_str(&format!("Failed to reset UI instructions: {}", e)))?;
        
        model::update_prompts_field("language_instructions", config.language_instructions(), true)
            .map_err(|e| JsValue::from_str(&format!("Failed to reset language instructions: {}", e)))?;
        
        model::update_prompts_field("agent_max_steps_error", config.agent_max_steps_error(), false)
            .map_err(|e| JsValue::from_str(&format!("Failed to reset agent max steps error: {}", e)))?;
        
        model::update_prompts_field("no_tools_guidance", config.no_tools_guidance(), false)
            .map_err(|e| JsValue::from_str(&format!("Failed to reset no tools guidance: {}", e)))?;
        
        Ok(())
    }

    /// Get the raw model.toml file content
    #[wasm_bindgen(js_name = getRawConfig)]
    pub fn get_raw_config() -> Result<String, JsValue> {
        let config_path = Path::new(MODEL_CONFIG_PATH);
        fs::read_to_string(config_path)
            .map_err(|e| JsValue::from_str(&format!("Failed to read config file: {}", e)))
    }
}

/// Helper function to load prompts config from file
fn load_prompts_config() -> Result<PromptsConfig, JsValue> {
    use antikythera_core::config::app::AppConfig;
    
    let config_path = Path::new(CONFIG_PATH);
    let config = AppConfig::load(Some(config_path))
        .map_err(|e| JsValue::from_str(&format!("Failed to load config: {}", e)))?;
    
    Ok(config.prompts)
}
