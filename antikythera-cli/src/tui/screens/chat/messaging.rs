//! Messaging handlers for chat
//!
//! Provides async message sending and command handling.

use super::input::{CommandResult, parse_command};
use super::state::{ChatMessage, ChatState};
use antikythera_core::application::agent::{Agent, AgentOptions};
use antikythera_core::application::client::{ChatRequest, McpClient};
use antikythera_core::infrastructure::model::ModelProvider;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Events from async response handling
pub(super) enum ResponseEvent {
    Message(String),
    Error(String),
    SessionUpdate(String),
    Logs(Vec<String>),
    Steps(Vec<antikythera_core::application::agent::AgentStep>),
}

/// Send message asynchronously
pub(super) async fn send_message<P>(
    client: Arc<McpClient<P>>,
    prompt: String,
    session_id: Option<String>,
    agent_mode: bool,
    tx: mpsc::Sender<ResponseEvent>,
) where
    P: ModelProvider + 'static,
{
    if agent_mode {
        let agent = Agent::new(client.clone());
        let mut options = AgentOptions::default();
        options.session_id = session_id;

        match agent.run(prompt, options).await {
            Ok(outcome) => {
                let _ = tx
                    .send(ResponseEvent::SessionUpdate(outcome.session_id))
                    .await;
                let response_str = match outcome.response {
                    serde_json::Value::String(s) => s,
                    v => serde_json::to_string(&v).unwrap_or_default(),
                };
                let _ = tx.send(ResponseEvent::Message(response_str)).await;
                if !outcome.logs.is_empty() {
                    let _ = tx.send(ResponseEvent::Logs(outcome.logs)).await;
                }
                if !outcome.steps.is_empty() {
                    let _ = tx.send(ResponseEvent::Steps(outcome.steps)).await;
                }
            }
            Err(err) => {
                let _ = tx.send(ResponseEvent::Error(err.user_message())).await;
            }
        }
    } else {
        let request = ChatRequest {
            prompt,
            session_id,
            ..Default::default()
        };

        match client.chat(request).await {
            Ok(result) => {
                let _ = tx
                    .send(ResponseEvent::SessionUpdate(result.session_id))
                    .await;
                let _ = tx.send(ResponseEvent::Message(result.content)).await;
                if !result.logs.is_empty() {
                    let _ = tx.send(ResponseEvent::Logs(result.logs)).await;
                }
            }
            Err(err) => {
                let _ = tx.send(ResponseEvent::Error(err.user_message())).await;
            }
        }
    }
}

/// Handle command execution
/// Returns true if setup menu should be shown
pub(super) fn handle_command(state: &mut ChatState, input: &str) -> bool {
    let result = parse_command(input);

    match result {
        CommandResult::None => {}

        CommandResult::ShowHelp => {
            state.add_message(ChatMessage::system(
                r#"Available commands:
  /help          - Show this help
  /agent [on|off] - Toggle or set agent mode
  /reset         - Reset session and start new
  /logs          - Show last interaction logs
  /steps         - Show last tool steps
  /exit          - Exit chat"#,
            ));
        }

        CommandResult::ToggleAgent => {
            state.toggle_agent_mode();
        }

        CommandResult::SetAgent(enabled) => {
            state.agent_mode = enabled;
            state.status_message = Some(format!(
                "Agent mode: {}",
                if enabled { "ON" } else { "OFF" }
            ));
        }

        CommandResult::Reset => {
            state.reset();
            state.add_message(ChatMessage::system("Session reset. Starting fresh."));
        }

        CommandResult::ShowLogs => {
            if state.last_logs.is_empty() {
                state.add_message(ChatMessage::system("No logs from last interaction."));
            } else {
                let logs = state.last_logs.join("\n");
                state.add_message(ChatMessage::system(format!("Last logs:\n{}", logs)));
            }
        }

        CommandResult::ShowSteps => {
            if state.last_steps.is_empty() {
                state.add_message(ChatMessage::system("No tool steps from last interaction."));
            } else {
                let steps: Vec<String> = state
                    .last_steps
                    .iter()
                    .map(|s| format!("• {}: {}", s.tool, if s.success { "✓" } else { "✗" }))
                    .collect();
                state.add_message(ChatMessage::system(format!(
                    "Last tool steps:\n{}",
                    steps.join("\n")
                )));
            }
        }

        CommandResult::ShowSetup => {
            // Signal that setup menu should be shown
            return true;
        }

        CommandResult::Exit => {}

        CommandResult::Unknown(cmd) => {
            state.add_message(ChatMessage::system(format!(
                "Unknown command: {}. Type /help for available commands.",
                cmd
            )));
        }
    }

    false
}
