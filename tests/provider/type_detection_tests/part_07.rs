#[test]
fn test_parse_empty_output() {
    let output = json!({});
    let parsed = parse_step_output(&output);

    assert!(parsed.texts.is_empty());
    assert!(parsed.files.is_empty());
}

#[test]
fn test_parse_text_content() {
    let output = json!({
        "content": [
            {
                "type": "text",
                "text": "Hello World"
            }
        ]
    });

    let parsed = parse_step_output(&output);

    assert_eq!(parsed.texts.len(), 1);
    assert_eq!(parsed.texts[0], "Hello World");
    assert!(parsed.files.is_empty());
}

#[test]
fn test_parse_resource_content() {
    let output = json!({
        "content": [
            {
                "type": "text",
                "text": "Document created"
            },
            {
                "type": "resource",
                "text": "Generated file: document.pdf",
                "data": "cGRmZGF0YQ==",
                "mimeType": "application/pdf"
            }
        ]
    });

    let parsed = parse_step_output(&output);

    assert_eq!(parsed.texts.len(), 1);
    assert_eq!(parsed.files.len(), 1);
    assert!(parsed.has_files());
    assert_eq!(parsed.files[0].metadata.filename, "document.pdf");
    assert!(parsed.files[0].is_pdf());
}

#[test]
fn test_parse_multiple_files() {
    let output = json!({
        "content": [
            {
                "type": "resource",
                "text": "Generated file: doc1.pdf",
                "data": "cGRm",
                "mimeType": "application/pdf"
            },
            {
                "type": "resource",
                "text": "Generated file: image.png",
                "data": "cG5n",
                "mimeType": "image/png"
            }
        ]
    });

    let parsed = parse_step_output(&output);

    assert_eq!(parsed.files.len(), 2);
    assert_eq!(parsed.pdf_files().len(), 1);
    assert_eq!(parsed.image_files().len(), 1);
}

#[test]
fn test_combined_text() {
    let output = json!({
        "content": [
            { "type": "text", "text": "Line 1" },
            { "type": "text", "text": "Line 2" }
        ]
    });

    let parsed = parse_step_output(&output);
    assert_eq!(parsed.combined_text(), "Line 1\nLine 2");
}
