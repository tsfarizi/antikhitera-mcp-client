//! Chat runner - main event loop coordinator

use super::input::{CommandResult, InputAction, handle_input, parse_command};
use super::state::{ChatMessage, ChatState};
use super::ui::ChatUI;
use crate::agent::{Agent, AgentOptions};
use crate::client::{ChatRequest, McpClient};
use crate::model::ModelProvider;
use crate::tui::terminal::{Tui, init_terminal, restore_terminal};
use crossterm::event::{self};
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

/// Result of chat session
pub enum ChatResult {
    Exit,
    Error(String),
}

/// Run the TUI chat interface
pub async fn run_chat<P>(
    client: Arc<McpClient<P>>,
    provider: String,
    model: String,
) -> Result<ChatResult, Box<dyn Error>>
where
    P: ModelProvider + 'static,
{
    let mut terminal = init_terminal()?;
    let mut state = ChatState::new();
    state.add_message(ChatMessage::system(
        "Welcome to MCP Chat! Type /help for commands, or start chatting.",
    ));

    let result = run_chat_loop(&mut terminal, &mut state, client, &provider, &model).await;

    restore_terminal()?;
    result
}

/// Internal chat loop
async fn run_chat_loop<P>(
    terminal: &mut Tui,
    state: &mut ChatState,
    client: Arc<McpClient<P>>,
    provider: &str,
    model: &str,
) -> Result<ChatResult, Box<dyn Error>>
where
    P: ModelProvider + 'static,
{
    let (response_tx, mut response_rx) = mpsc::channel::<ResponseEvent>(10);

    loop {
        terminal.draw(|frame| {
            ChatUI::render(frame, state, provider, model);
        })?;
        while let Ok(event) = response_rx.try_recv() {
            match event {
                ResponseEvent::Message(content) => {
                    state.loading = false;
                    state.add_message(ChatMessage::assistant(content));
                }
                ResponseEvent::Error(err) => {
                    state.loading = false;
                    state.add_message(ChatMessage::system(format!("Error: {}", err)));
                }
                ResponseEvent::SessionUpdate(id) => {
                    let is_new = state.session_id.as_ref() != Some(&id);
                    state.session_id = Some(id.clone());
                    if is_new {
                        state.status_message = Some(format!("Session: {}", &id[..8.min(id.len())]));
                    }
                }
                ResponseEvent::Logs(logs) => {
                    state.last_logs = logs;
                }
                ResponseEvent::Steps(steps) => {
                    state.last_steps = steps;
                }
            }
        }
        let timeout = if state.loading {
            Duration::from_millis(100)
        } else {
            Duration::from_millis(50)
        };

        if event::poll(timeout)? {
            let event = event::read()?;
            let action = handle_input(state, event);

            match action {
                InputAction::Exit => {
                    return Ok(ChatResult::Exit);
                }

                InputAction::Submit => {
                    let input = state.take_input();
                    if !input.is_empty() {
                        state.add_message(ChatMessage::user(&input));
                        state.loading = true;
                        state.status_message = None;
                        let client_clone = client.clone();
                        let session_id = state.session_id.clone();
                        let agent_mode = state.agent_mode;
                        let tx = response_tx.clone();

                        tokio::spawn(async move {
                            send_message(client_clone, input, session_id, agent_mode, tx).await;
                        });
                    }
                }

                InputAction::Command(cmd) => {
                    handle_command(state, &cmd);
                }

                InputAction::ScrollUp => {
                    state.scroll_up();
                }

                InputAction::ScrollDown => {
                    state.scroll_down(1000); // Max scroll will be limited by content
                }

                InputAction::ScrollTop => {
                    state.scroll_offset = 0;
                }

                InputAction::ScrollBottom => {
                    state.scroll_to_bottom();
                }

                InputAction::None => {}
            }
        } else if state.loading {
            state.tick_loading();
        }
        if state.status_message.is_some() && !state.loading {}
    }
}

/// Events from async response handling
enum ResponseEvent {
    Message(String),
    Error(String),
    SessionUpdate(String),
    Logs(Vec<String>),
    Steps(Vec<crate::agent::AgentStep>),
}

/// Send message asynchronously
async fn send_message<P>(
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
                let _ = tx.send(ResponseEvent::Message(outcome.response)).await;
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
fn handle_command(state: &mut ChatState, input: &str) {
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

        CommandResult::Exit => {}

        CommandResult::Unknown(cmd) => {
            state.add_message(ChatMessage::system(format!(
                "Unknown command: {}. Type /help for available commands.",
                cmd
            )));
        }
    }
}
