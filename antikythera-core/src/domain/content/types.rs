//! Content types mirroring MCP server types.

use serde::{Deserialize, Serialize};

/// Metadata for file content.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FileMetadata {
    /// Original filename with extension
    pub filename: String,
    /// MIME type (e.g., "application/pdf")
    pub mime_type: String,
    /// File size in bytes
    pub size_bytes: usize,
    /// Creation timestamp in ISO8601 format
    pub created_at: String,
}

/// File content with metadata and base64-encoded data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FileContent {
    /// File metadata
    pub metadata: FileMetadata,
    /// Base64-encoded file data
    pub data: String,
}

impl FileContent {
    /// Decode base64 data back to bytes.
    pub fn decode_data(&self) -> Result<Vec<u8>, base64::DecodeError> {
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD as BASE64;
        BASE64.decode(&self.data)
    }

    /// Check if this is a PDF file.
    pub fn is_pdf(&self) -> bool {
        self.metadata.mime_type == "application/pdf"
    }

    /// Check if this is an image file.
    pub fn is_image(&self) -> bool {
        self.metadata.mime_type.starts_with("image/")
    }

    /// Get file extension from filename.
    pub fn extension(&self) -> Option<&str> {
        self.metadata.filename.rsplit('.').next()
    }
}

/// Content item from MCP tool output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentItem {
    /// Content type ("text" or "resource")
    #[serde(rename = "type")]
    pub content_type: String,
    /// Text content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Base64-encoded data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    /// MIME type
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// File metadata (extended field)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<FileMetadata>,
}

impl ContentItem {
    /// Check if this is a text content.
    pub fn is_text(&self) -> bool {
        self.content_type == "text"
    }

    /// Check if this is a resource/file content.
    pub fn is_resource(&self) -> bool {
        self.content_type == "resource"
    }

    /// Convert to FileContent if this is a resource.
    pub fn to_file_content(&self) -> Option<FileContent> {
        if !self.is_resource() {
            return None;
        }

        let data = self.data.as_ref()?;
        let mime_type = self
            .mime_type
            .clone()
            .unwrap_or_else(|| "application/octet-stream".to_string());

        // Use provided metadata or create from available info
        let metadata = self.metadata.clone().unwrap_or_else(|| {
            let filename = self
                .text
                .as_ref()
                .and_then(|t| t.strip_prefix("Generated file: "))
                .unwrap_or("unknown")
                .to_string();

            FileMetadata {
                filename,
                mime_type: mime_type.clone(),
                size_bytes: data.len(),
                created_at: chrono::Utc::now().to_rfc3339(),
            }
        });

        Some(FileContent {
            metadata,
            data: data.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_content_decode() {
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD as BASE64;

        let original = b"Hello World";
        let encoded = BASE64.encode(original);

        let file = FileContent {
            metadata: FileMetadata {
                filename: "test.txt".to_string(),
                mime_type: "text/plain".to_string(),
                size_bytes: original.len(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
            },
            data: encoded,
        };

        let decoded = file.decode_data().unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_file_content_is_pdf() {
        let file = FileContent {
            metadata: FileMetadata {
                filename: "doc.pdf".to_string(),
                mime_type: "application/pdf".to_string(),
                size_bytes: 100,
                created_at: "2024-01-01T00:00:00Z".to_string(),
            },
            data: "test".to_string(),
        };

        assert!(file.is_pdf());
        assert!(!file.is_image());
    }

    #[test]
    fn test_content_item_to_file_content() {
        let item = ContentItem {
            content_type: "resource".to_string(),
            text: Some("Generated file: test.pdf".to_string()),
            data: Some("cGRmZGF0YQ==".to_string()),
            mime_type: Some("application/pdf".to_string()),
            metadata: None,
        };

        let file = item.to_file_content().unwrap();
        assert_eq!(file.metadata.filename, "test.pdf");
        assert_eq!(file.metadata.mime_type, "application/pdf");
    }
}
