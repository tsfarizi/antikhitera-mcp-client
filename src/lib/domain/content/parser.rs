//! Parser for MCP tool output.

use super::types::{ContentItem, FileContent};
use serde_json::Value;

/// Parsed output from a tool step.
#[derive(Debug, Clone, Default)]
pub struct ParsedOutput {
    /// Text messages from the output
    pub texts: Vec<String>,
    /// File contents extracted from resources
    pub files: Vec<FileContent>,
    /// Raw output value (preserved for backward compatibility)
    pub raw: Value,
}

impl ParsedOutput {
    /// Check if there are any files in the output.
    pub fn has_files(&self) -> bool {
        !self.files.is_empty()
    }

    /// Get the first text message.
    pub fn first_text(&self) -> Option<&String> {
        self.texts.first()
    }

    /// Get combined text messages.
    pub fn combined_text(&self) -> String {
        self.texts.join("\n")
    }

    /// Get PDF files only.
    pub fn pdf_files(&self) -> Vec<&FileContent> {
        self.files.iter().filter(|f| f.is_pdf()).collect()
    }

    /// Get image files only.
    pub fn image_files(&self) -> Vec<&FileContent> {
        self.files.iter().filter(|f| f.is_image()).collect()
    }
}

/// Parse raw tool output into structured format.
pub fn parse_step_output(output: &Value) -> ParsedOutput {
    let mut parsed = ParsedOutput {
        raw: output.clone(),
        ..Default::default()
    };

    // Try to parse as MCP tool result with content array
    if let Some(content) = output.get("content").and_then(|c| c.as_array()) {
        for item in content {
            if let Ok(content_item) = serde_json::from_value::<ContentItem>(item.clone()) {
                if content_item.is_text() {
                    if let Some(text) = &content_item.text {
                        parsed.texts.push(text.clone());
                    }
                } else if content_item.is_resource() {
                    if let Some(file) = content_item.to_file_content() {
                        parsed.files.push(file);
                    }
                }
            }
        }
    }

    parsed
}

/// Parse multiple tool outputs from an array.
pub fn parse_tool_outputs(outputs: &[Value]) -> Vec<ParsedOutput> {
    outputs.iter().map(parse_step_output).collect()
}

/// Extract all files from multiple outputs.
pub fn extract_all_files(outputs: &[ParsedOutput]) -> Vec<&FileContent> {
    outputs.iter().flat_map(|o| o.files.iter()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
}
