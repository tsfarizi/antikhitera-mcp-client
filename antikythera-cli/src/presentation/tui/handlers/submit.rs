//! Chat input submission handler with agent/chat dispatch.

use std::sync::Arc;

use chrono::Utc;

use crate::infrastructure::history::{ChatHistorySession, ChatHistoryStore, ChatTurn, TurnRole};
use crate::infrastructure::llm::{StreamEvent, set_stream_event_sink};
use crate::presentation::tui::app::ChatApp;
use crate::presentation::tui::event_loop::scroll_to_bottom;
use crate::presentation::tui::types::{PendingResponse, UiMessage, UiTone};
use antikythera_core::ProviderLogger;
use antikythera_core::application::agent::{Agent, AgentOptions};
use antikythera_core::application::client::{ChatRequest, McpClient};
use antikythera_core::application::resilience::{ContextWindowPolicy, RetryPolicy, with_retry_if};
use antikythera_core::infrastructure::model::DynamicModelProvider;
use tokio::sync::{mpsc, oneshot};

use super::commands::process_command;

pub(crate) fn submit_input(client: &mut Arc<McpClient<DynamicModelProvider>>, app: &mut ChatApp) {
    let input = app.input.trim().to_string();
    app.input.clear();

    if input.is_empty() {
        app.status = "Ketik pesan atau slash command untuk melanjutkan.".to_string();
        return;
    }

    // Prevent double-submission while a request is already in-flight.
    if app.pending_rx.is_some() {
        app.status = "Menunggu respons...".to_string();
        return;
    }

    if input.starts_with('/') {
        process_command(app, client, &input);
        return;
    }

    app.push_message(UiMessage::new("You", &input, UiTone::User));
    app.status = format!("Mengirim ke {}/{}...", app.provider, app.model);
    app.loading = true;
    // Scroll to show latest messages (count lines from message body lengths).
    app.conversation_scroll = scroll_to_bottom(&app.messages, app.conversation_scroll);

    // Capture user turn into the in-flight debug history session.
    if app.current_history_session.is_none() {
        app.current_history_session = Some(ChatHistorySession::new(
            ChatHistoryStore::new_id(),
            app.provider.clone(),
            app.model.clone(),
            app.agent_mode,
        ));
    }
    if let Some(session) = &mut app.current_history_session {
        if let Some(ref id) = app.session_id {
            session.core_session_id = Some(id.clone());
        }
        session.turns.push(ChatTurn {
            timestamp: Utc::now().to_rfc3339(),
            role: TurnRole::User,
            content: input.clone(),
            tool_steps: 0,
        });
    }

    let (tx, rx) = oneshot::channel();
    app.pending_rx = Some(rx);

    // Install a streaming sink that forwards token chunks to the TUI render loop
    // so tokens appear live in the Conversation panel while the task runs.
    let (stream_tx, stream_rx) = mpsc::unbounded_channel::<String>();
    app.stream_rx = Some(stream_rx);
    app.streaming_content.clear();
    set_stream_event_sink(Arc::new(move |event: &StreamEvent| {
        if let StreamEvent::Chunk { content, .. } = event {
            let _ = stream_tx.send(content.clone());
        }
    }));

    let health_ref = Arc::clone(&app.health);
    let provider_id = app.provider.clone();
    let model_id = app.model.clone();
    ProviderLogger::new(&antikythera_core::get_active_session()).info(format!(
        "CLI → CORE: dispatching request | provider={} model={} mode={} session={}",
        provider_id,
        model_id,
        if app.agent_mode { "agent" } else { "chat" },
        app.session_id.as_deref().unwrap_or("<baru>"),
    ));

    // TODO: Add integration tests for submit_input with mock ChatApp state.
    // The function requires a full running client, channel setup, and tokio runtime.

    if app.agent_mode {
        let options = AgentOptions {
            session_id: app.session_id.clone(),
            ..AgentOptions::default()
        };
        let client_arc = Arc::clone(client);
        tokio::spawn(async move {
            let start = std::time::Instant::now();
            let result = Agent::new(client_arc)
                .run(input, options)
                .await
                .map_err(|e| e.user_message());
            let elapsed_ms = start.elapsed().as_millis() as u64;
            if let Ok(mut h) = health_ref.lock() {
                match &result {
                    Ok(_) => h.record_success(&provider_id, elapsed_ms),
                    Err(e) => h.record_failure(&provider_id, e.as_str()),
                }
            }
            let _ = tx.send(PendingResponse::Agent(result));
        });
    } else {
        let client_arc = Arc::clone(client);
        let session_id = app.session_id.clone();
        let cw_policy = ContextWindowPolicy::default();
        let retry_policy = RetryPolicy::default();
        tokio::spawn(async move {
            // Auto-prune context window before sending if the session is long.
            if let Some(ref sid) = session_id {
                let removed = client_arc.prune_session(sid, &cw_policy).await;
                if removed > 0 {
                    antikythera_core::SessionLogger::new(sid).info(format!(
                        "Context window pruned before request | removed={}",
                        removed
                    ));
                }
            }

            let start = std::time::Instant::now();
            // Retry on transient failures with exponential back-off.
            let result: Result<
                antikythera_core::application::client::ChatResult,
                antikythera_core::application::client::McpError,
            > = with_retry_if(
                &retry_policy,
                || {
                    let c = Arc::clone(&client_arc);
                    let prompt = input.clone();
                    let sid = session_id.clone();
                    async move {
                        c.chat(ChatRequest {
                            prompt,
                            attachments: Vec::new(),
                            system_prompt: None,
                            session_id: sid,
                            raw_mode: false,
                            bypass_template: false,
                            force_json: false,
                        })
                        .await
                    }
                },
                |_: &antikythera_core::application::client::McpError| true,
            )
            .await;

            let elapsed_ms = start.elapsed().as_millis() as u64;
            if let Ok(mut h) = health_ref.lock() {
                match &result {
                    Ok(r) => h.record_success(&r.provider, elapsed_ms),
                    Err(e) => h.record_failure(&provider_id, e.user_message()),
                }
            }
            let _ = tx.send(PendingResponse::Chat(result.map_err(|e| e.user_message())));
        });
    }
}
