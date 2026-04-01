//! Text sanitization utilities for TOML-safe strings.

/// Characters that are problematic in TOML basic strings.
const TOML_UNSAFE_CHARS: &[char] = &['\n', '\r', '\t', '\\', '"'];

/// Sanitize a string to be TOML-safe for use in basic strings.
///
/// This function:
/// - Removes newlines and replaces with spaces
/// - Removes emojis and special Unicode characters
/// - Escapes backslashes and quotes
/// - Trims excess whitespace
pub fn sanitize_for_toml(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut prev_was_space = false;

    for c in input.chars() {
        match c {
            // Replace newlines/tabs with space
            '\n' | '\r' | '\t' => {
                if !prev_was_space {
                    result.push(' ');
                    prev_was_space = true;
                }
            }
            // Escape backslashes
            '\\' => {
                result.push_str("\\\\");
                prev_was_space = false;
            }
            // Escape quotes
            '"' => {
                result.push_str("\\\"");
                prev_was_space = false;
            }
            // Keep ASCII printable characters
            c if c.is_ascii() && !c.is_ascii_control() => {
                if c == ' ' {
                    if !prev_was_space {
                        result.push(c);
                        prev_was_space = true;
                    }
                } else {
                    result.push(c);
                    prev_was_space = false;
                }
            }
            // Keep common non-ASCII letters (Indonesian uses standard Latin)
            c if c.is_alphabetic() => {
                result.push(c);
                prev_was_space = false;
            }
            // Remove emojis and other special characters, replace with space
            _ => {
                if !prev_was_space {
                    result.push(' ');
                    prev_was_space = true;
                }
            }
        }
    }

    result.trim().to_string()
}

/// Check if a string contains characters that would break TOML basic string parsing.
pub fn needs_sanitization(input: &str) -> bool {
    input
        .chars()
        .any(|c| TOML_UNSAFE_CHARS.contains(&c) || c.is_control() || is_emoji(c))
}

/// Check if a character is likely an emoji.
fn is_emoji(c: char) -> bool {
    let code = c as u32;
    // Common emoji ranges (non-overlapping)
    matches!(code,
        0x1F300..=0x1F9FF |  // Misc Symbols, Pictographs, Emoticons, Transport, etc.
        0x2600..=0x26FF |    // Misc symbols
        0x2700..=0x27BF |    // Dingbats
        0xFE00..=0xFE0F |    // Variation Selectors
        0x200D                // Zero Width Joiner
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_removes_newlines() {
        let input = "Line 1\nLine 2\r\nLine 3";
        let result = sanitize_for_toml(input);
        assert_eq!(result, "Line 1 Line 2 Line 3");
    }

    #[test]
    fn test_sanitize_escapes_quotes() {
        let input = r#"He said "hello""#;
        let result = sanitize_for_toml(input);
        assert_eq!(result, r#"He said \"hello\""#);
    }

    #[test]
    fn test_sanitize_removes_emoji() {
        let input = "⚠️ Warning: Important message";
        let result = sanitize_for_toml(input);
        assert!(result.contains("Warning"));
        assert!(!result.contains("⚠"));
    }

    #[test]
    fn test_sanitize_collapses_whitespace() {
        let input = "Multiple   spaces   here";
        let result = sanitize_for_toml(input);
        assert_eq!(result, "Multiple spaces here");
    }

    #[test]
    fn test_sanitize_complex_description() {
        let input =
            "Membuat Surat SKTM.\n\n⚠️ INSTRUKSI:\n1. WAJIB tanyakan data\n2. DILARANG dummy";
        let result = sanitize_for_toml(input);
        assert!(!result.contains('\n'));
        assert!(!result.contains("⚠"));
        assert!(result.contains("INSTRUKSI"));
    }

    #[test]
    fn test_needs_sanitization() {
        assert!(needs_sanitization("Has\nnewline"));
        assert!(needs_sanitization("Has⚠️emoji"));
        assert!(!needs_sanitization("Normal text"));
    }

    #[test]
    fn test_clean_text_unchanged() {
        let input = "This is clean text without special chars";
        let result = sanitize_for_toml(input);
        assert_eq!(result, input);
    }
}
