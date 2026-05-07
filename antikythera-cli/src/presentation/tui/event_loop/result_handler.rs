use chrono::Utc;

use antikythera_core::ProviderLogger;
use antikythera_core::application::agent::{AgentOutcome, AgentStep};
use antikythera_core::application::client::ChatResult;

use crate::infrastructure::history::{ChatTurn, TurnRole};

use super::super::app::ChatApp;
use super::super::types::{UiMessage, UiTone};

pub(super) fn apply_chat_result(app: &mut ChatApp, result: ChatResult) {
    ProviderLogger::new(&antikythera_core::get_active_session()).info(format!(
        "CORE → CLI: chat response received | provider={} model={} session={} chars={}",
        result.provider,
        result.model,
        result.session_id,
        result.content.len(),
    ));
    app.session_id = Some(result.session_id.clone());
    antikythera_core::set_active_session(&result.session_id);
    app.status = format!(
        "Respons diterima dari {}/{}.",
        result.provider, result.model
    );
    app.push_message(UiMessage::new(
        format!("Assistant [{}]", result.provider),
        result.content.clone(),
        UiTone::Assistant,
    ));

    // Append assistant turn and persist the debug history session.
    if let Some(session) = &mut app.current_history_session {
        session.core_session_id = Some(result.session_id.clone());
        session.updated_at = Utc::now().to_rfc3339();
        if session.title.is_empty()
            && let Some(first) = session.turns.iter().find(|t| t.role == TurnRole::User)
        {
            session.title = first.content.chars().take(60).collect();
        }
        session.turns.push(ChatTurn {
            timestamp: Utc::now().to_rfc3339(),
            role: TurnRole::Assistant,
            content: result.content.clone(),
            tool_steps: 0,
        });
        let _ = app.history_store.save_session(session);
    }
}

pub(super) fn apply_agent_outcome(app: &mut ChatApp, outcome: AgentOutcome) {
    ProviderLogger::new(&antikythera_core::get_active_session()).info(format!(
        "CORE → CLI: agent outcome received | session={} steps={} chars={}",
        outcome.session_id,
        outcome.steps.len(),
        outcome.response.to_string().len(),
    ));
    app.session_id = Some(outcome.session_id.clone());
    antikythera_core::set_active_session(&outcome.session_id);
    app.status = format!("Agent selesai dengan {} langkah tool.", outcome.steps.len());
    let response_text = format_agent_response(&outcome.response);
    app.push_message(UiMessage::new(
        "Agent",
        response_text.clone(),
        UiTone::Assistant,
    ));
    // Scroll conversation to show the response.
    app.conversation_scroll = scroll_to_bottom(&app.messages, app.conversation_scroll);

    if !outcome.steps.is_empty() {
        app.push_message(UiMessage::new(
            "Tool Trace",
            render_steps_summary(&outcome.steps),
            UiTone::System,
        ));
    }

    // Append assistant turn and persist the debug history session.
    let tool_step_count = outcome.steps.len();
    if let Some(session) = &mut app.current_history_session {
        session.core_session_id = Some(outcome.session_id.clone());
        session.updated_at = Utc::now().to_rfc3339();
        if session.title.is_empty()
            && let Some(first) = session.turns.iter().find(|t| t.role == TurnRole::User)
        {
            session.title = first.content.chars().take(60).collect();
        }
        session.turns.push(ChatTurn {
            timestamp: Utc::now().to_rfc3339(),
            role: TurnRole::Assistant,
            content: response_text,
            tool_steps: tool_step_count,
        });
        let _ = app.history_store.save_session(session);
    }
}

/// Estimate the scroll offset needed to show the bottom of the conversation.
/// Each message body line counts as 1 visible line; message headers add 1 line each;
/// blank separators add 1 line each.
pub(crate) fn scroll_to_bottom(messages: &[UiMessage], _current_scroll: u16) -> u16 {
    let total_lines: usize = messages
        .iter()
        .map(|m| 2 + m.body.lines().count()) // title line + 1 body line minimum + separator
        .sum();
    total_lines.saturating_sub(8) as u16 // ~8 lines of viewport
}

fn format_agent_response(value: &serde_json::Value) -> String {
    value.as_str().map(ToOwned::to_owned).unwrap_or_else(|| {
        serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
    })
}

fn render_steps_summary(steps: &[AgentStep]) -> String {
    steps
        .iter()
        .enumerate()
        .map(|(index, step)| {
            format!(
                "{}. {} [{}]{}",
                index + 1,
                step.tool,
                if step.success { "ok" } else { "failed" },
                step.message
                    .as_deref()
                    .map(|message| format!(" - {}", message))
                    .unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
