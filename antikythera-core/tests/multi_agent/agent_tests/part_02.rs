use antikythera_core::application::agent::{AgentState, Event, TerminationReason};

// ── is_terminal ───────────────────────────────────────────────────────────

#[test]
fn terminated_is_terminal() {
    let state = AgentState::Terminated {
        reason: TerminationReason::Success,
    };
    assert!(state.is_terminal());
}

#[test]
fn final_message_is_terminal() {
    let state = AgentState::FinalMessage {
        content: "done".into(),
        data: None,
        metadata: None,
    };
    assert!(state.is_terminal());
}

#[test]
fn non_terminal_states_are_not_terminal() {
    for state in [
        AgentState::Idle,
        AgentState::ParsingDirective,
        AgentState::WaitingForContext,
        AgentState::FinalizingResponse,
    ] {
        assert!(!state.is_terminal(), "{state} should not be terminal");
    }
}

// ── transition: Idle ─────────────────────────────────────────────────────

#[test]
fn idle_prompt_received_transitions_to_parsing_directive() {
    let next = AgentState::Idle.transition(Event::PromptReceived {
        prompt: "hello".into(),
    });
    assert_eq!(next, AgentState::ParsingDirective);
}

#[test]
fn idle_cancelled_transitions_to_terminated() {
    let next = AgentState::Idle.transition(Event::Cancelled);
    assert!(matches!(
        next,
        AgentState::Terminated {
            reason: TerminationReason::Cancelled
        }
    ));
}

#[test]
fn idle_error_transitions_to_recovering() {
    let next = AgentState::Idle.transition(Event::Error {
        message: "boom".into(),
    });
    assert!(matches!(
        next,
        AgentState::RecoveringError { retry_count: 0, .. }
    ));
}

// ── transition: ParsingDirective ─────────────────────────────────────────

#[test]
fn parsing_directive_parsed_transitions_to_executing_tool() {
    let next = AgentState::ParsingDirective.transition(Event::DirectiveParsed {
        tool: "my_tool".into(),
        input: serde_json::json!({}),
    });
    assert!(matches!(next, AgentState::ExecutingTool { .. }));
}

#[test]
fn parsing_final_response_transitions_to_finalizing() {
    let next = AgentState::ParsingDirective.transition(Event::FinalResponse);
    assert_eq!(next, AgentState::FinalizingResponse);
}

#[test]
fn parsing_error_transitions_to_recovering() {
    let next = AgentState::ParsingDirective.transition(Event::Error {
        message: "parse error".into(),
    });
    assert!(matches!(
        next,
        AgentState::RecoveringError { retry_count: 0, .. }
    ));
}

#[test]
fn parsing_max_steps_transitions_to_terminated() {
    let next = AgentState::ParsingDirective.transition(Event::MaxStepsExceeded);
    assert!(matches!(
        next,
        AgentState::Terminated {
            reason: TerminationReason::MaxStepsExceeded
        }
    ));
}

// ── transition: ExecutingTool ────────────────────────────────────────────

#[test]
fn executing_tool_completed_returns_to_parsing() {
    let state = AgentState::ExecutingTool {
        tool_id: "t".into(),
        input: serde_json::json!({}),
    };
    let next = state.transition(Event::ToolCompleted {
        tool: "t".into(),
        output: serde_json::json!({}),
    });
    assert_eq!(next, AgentState::ParsingDirective);
}

#[test]
fn executing_tool_failed_transitions_to_recovering() {
    let state = AgentState::ExecutingTool {
        tool_id: "t".into(),
        input: serde_json::json!({}),
    };
    let next = state.transition(Event::ToolFailed {
        tool: "t".into(),
        error: "network error".into(),
    });
    assert!(matches!(
        next,
        AgentState::RecoveringError { retry_count: 0, .. }
    ));
}

// ── transition: WaitingForContext ────────────────────────────────────────

#[test]
fn waiting_context_received_transitions_to_parsing() {
    let next = AgentState::WaitingForContext.transition(Event::ContextReceived {
        context: "data".into(),
    });
    assert_eq!(next, AgentState::ParsingDirective);
}

// ── transition: RecoveringError ──────────────────────────────────────────

#[test]
fn recovering_error_increments_retry_count() {
    let state = AgentState::RecoveringError {
        error: "e".into(),
        retry_count: 1,
    };
    let next = state.transition(Event::Error {
        message: "e2".into(),
    });
    assert!(matches!(
        next,
        AgentState::RecoveringError { retry_count: 2, .. }
    ));
}

#[test]
fn recovering_saturates_at_max_u8() {
    let state = AgentState::RecoveringError {
        error: "e".into(),
        retry_count: u8::MAX,
    };
    let next = state.transition(Event::Error {
        message: "e".into(),
    });
    assert!(matches!(
        next,
        AgentState::RecoveringError {
            retry_count: 255,
            ..
        }
    ));
}

// ── transition: FinalizingResponse ───────────────────────────────────────

#[test]
fn finalizing_response_sent_transitions_to_final_message() {
    let next = AgentState::FinalizingResponse.transition(Event::ResponseSent);
    assert!(matches!(next, AgentState::FinalMessage { .. }));
}

// ── transition: terminal states are sticky ───────────────────────────────

#[test]
fn final_message_ignores_all_events() {
    let state = AgentState::FinalMessage {
        content: "x".into(),
        data: None,
        metadata: None,
    };
    let next = state.transition(Event::Cancelled);
    assert!(matches!(next, AgentState::FinalMessage { .. }));
}

#[test]
fn terminated_ignores_all_events() {
    let state = AgentState::Terminated {
        reason: TerminationReason::Success,
    };
    let next = state.transition(Event::PromptReceived {
        prompt: "hi".into(),
    });
    assert!(matches!(
        next,
        AgentState::Terminated {
            reason: TerminationReason::Success
        }
    ));
}
