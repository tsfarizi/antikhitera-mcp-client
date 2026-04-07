//! Agent Management Integration Tests

use antikythera_sdk::agents::*;
use serial_test::serial;
use std::ffi::CString;

fn c_string_to_rust(ptr: *mut std::os::raw::c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe {
        let s = std::ffi::CStr::from_ptr(ptr).to_str().unwrap().to_string();
        drop(CString::from_raw(ptr));
        s
    }
}

#[test]
#[serial]
fn test_register_valid_agent() {
    agents_lock().clear();
    agent_status_lock().clear();

    let config = AgentConfig {
        id: "test-agent".to_string(),
        agent_type: AgentType::GeneralAssistant,
        name: "Test Agent".to_string(),
        description: Some("A test agent".to_string()),
        model_provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        max_steps: 10,
        can_call_tools: true,
        capabilities: vec![
            AgentCapability {
                name: "coding".to_string(),
                level: SkillLevel::Expert,
                description: "Expert coding assistant".to_string(),
            }
        ],
        custom_prompt: None,
        temperature: Some(0.7),
        enabled: true,
    };

    let json = serde_json::to_string(&config).unwrap();
    let c_json = CString::new(json).unwrap();

    let result_ptr = mcp_register_agent(c_json.as_ptr());
    let result = c_string_to_rust(result_ptr);
    let validation: AgentValidationResult = serde_json::from_str(&result).unwrap();

    assert!(validation.valid);
    assert_eq!(validation.agent_id, "test-agent");
}

#[test]
#[serial]
fn test_register_duplicate_agent() {
    agents_lock().clear();
    agent_status_lock().clear();

    let config = AgentConfig {
        id: "duplicate-agent".to_string(),
        agent_type: AgentType::CodeReviewer,
        name: "Duplicate Agent".to_string(),
        description: None,
        model_provider: "anthropic".to_string(),
        model: "claude-3".to_string(),
        max_steps: 5,
        can_call_tools: false,
        capabilities: vec![],
        custom_prompt: None,
        temperature: None,
        enabled: true,
    };

    let json = serde_json::to_string(&config).unwrap();
    mcp_register_agent(CString::new(json.clone()).unwrap().as_ptr());

    let c_json2 = CString::new(json).unwrap();
    let result_ptr = mcp_register_agent(c_json2.as_ptr());
    let result = c_string_to_rust(result_ptr);
    let validation: AgentValidationResult = serde_json::from_str(&result).unwrap();

    assert!(!validation.valid);
    assert!(validation.errors.iter().any(|e| e.contains("already exists")));
}

#[test]
#[serial]
fn test_list_agents() {
    agents_lock().clear();
    agent_status_lock().clear();

    let config1 = AgentConfig {
        id: "agent-1".to_string(),
        agent_type: AgentType::GeneralAssistant,
        name: "Agent 1".to_string(),
        description: None,
        model_provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        max_steps: 10,
        can_call_tools: true,
        capabilities: vec![],
        custom_prompt: None,
        temperature: None,
        enabled: true,
    };

    let config2 = AgentConfig {
        id: "agent-2".to_string(),
        agent_type: AgentType::DataAnalyst,
        name: "Agent 2".to_string(),
        description: None,
        model_provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        max_steps: 15,
        can_call_tools: true,
        capabilities: vec![],
        custom_prompt: None,
        temperature: None,
        enabled: true,
    };

    mcp_register_agent(CString::new(serde_json::to_string(&config1).unwrap()).unwrap().as_ptr());
    mcp_register_agent(CString::new(serde_json::to_string(&config2).unwrap()).unwrap().as_ptr());

    let result_ptr = mcp_list_agents();
    let result = c_string_to_rust(result_ptr);
    let agents: Vec<AgentConfig> = serde_json::from_str(&result).unwrap();

    assert_eq!(agents.len(), 2);
}

#[test]
#[serial]
fn test_agent_validation_invalid_temperature() {
    let config = AgentConfig {
        id: "valid-id".to_string(),
        agent_type: AgentType::Custom,
        name: "Valid Name".to_string(),
        description: None,
        model_provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        max_steps: 10,
        can_call_tools: true,
        capabilities: vec![],
        custom_prompt: None,
        temperature: Some(3.0),
        enabled: true,
    };

    let validation = config.validate();
    assert!(!validation.valid);
    assert!(validation.errors.iter().any(|e| e.contains("Temperature")));
}

#[test]
#[serial]
fn test_export_import_agents() {
    agents_lock().clear();
    agent_status_lock().clear();

    let config = AgentConfig {
        id: "export-test-agent".to_string(),
        agent_type: AgentType::CodeReviewer,
        name: "Export Test Agent".to_string(),
        description: None,
        model_provider: "anthropic".to_string(),
        model: "claude-3".to_string(),
        max_steps: 5,
        can_call_tools: false,
        capabilities: vec![],
        custom_prompt: None,
        temperature: None,
        enabled: true,
    };

    mcp_register_agent(CString::new(serde_json::to_string(&config).unwrap()).unwrap().as_ptr());

    let export_ptr = mcp_export_agents_config();
    let export_json = c_string_to_rust(export_ptr);

    agents_lock().clear();
    agent_status_lock().clear();

    let import_ptr = mcp_import_agents_config(CString::new(export_json).unwrap().as_ptr());
    let import_result = c_string_to_rust(import_ptr);
    let import_result_json: serde_json::Value = serde_json::from_str(&import_result).unwrap();

    assert_eq!(import_result_json["success"], true);
    assert_eq!(import_result_json["imported"], 1);
}
