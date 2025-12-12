//! Chat runner - main event loop coordinator
//!
//! Provides the main chat loop and TUI interface.

use super::input::{InputAction, handle_input};
use super::messaging::{ResponseEvent, handle_command, send_message};
use super::state::{ChatMessage, ChatState};
use super::ui::ChatUI;
use crate::client::McpClient;
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
