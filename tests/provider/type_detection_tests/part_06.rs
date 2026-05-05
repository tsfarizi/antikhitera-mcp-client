#[test]
fn host_response_accepts_plain_text() {
    let response =
        HostModelResponse::from_text("halo", Some("sess-1".to_string()));
    let model_response = response.into_model_response("host").unwrap();

    assert_eq!(model_response.message.content(), "halo");
    assert_eq!(model_response.session_id.as_deref(), Some("sess-1"));
}

#[test]
fn host_response_accepts_structured_message() {
    let response = HostModelResponse {
        text: None,
        message: Some(ChatMessage::with_parts(
            MessageRole::Assistant,
            vec![
                MessagePart::text("ringkasan: "),
                MessagePart::text("siap"),
            ],
        )),
        session_id: Some("sess-2".to_string()),
        raw_response_json: Some("{\"id\":\"abc\"}".to_string()),
    };

    let model_response = response.into_model_response("host").unwrap();

    assert_eq!(model_response.message.content(), "ringkasan: siap");
    assert_eq!(model_response.session_id.as_deref(), Some("sess-2"));
}

#[test]
fn host_response_requires_text_or_message() {
    let response = HostModelResponse {
        text: None,
        message: None,
        session_id: None,
        raw_response_json: None,
    };

    let error = response.into_model_response("host").unwrap_err();
    assert!(matches!(error, ModelError::InvalidResponse { .. }));
}
