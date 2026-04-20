// Wizard tests - verifying configuration wizard functionality
//
// Tests for wizard configuration generation including HTTP server support.

mod http_server_generation_tests {
    use std::collections::HashMap;

    /// Helper to generate HTTP server TOML block (mirrors client::add_http_server logic)
    fn generate_http_server_toml(
        name: &str,
        url: &str,
        headers: &HashMap<String, String>,
    ) -> String {
        let headers_toml = if headers.is_empty() {
            String::new()
        } else {
            let pairs: Vec<String> = headers
                .iter()
                .map(|(k, v)| format!("{} = \"{}\"", k, v))
                .collect();
            format!("\nheaders = {{ {} }}", pairs.join(", "))
        };

        format!(
            r#"
[[servers]]
name = "{}"
url = "{}"{}"#,
            name, url, headers_toml
        )
    }

// Split into 5 parts for consistent test organization.
include!("wizard_tests/part_01.rs");
include!("wizard_tests/part_02.rs");
include!("wizard_tests/part_03.rs");
include!("wizard_tests/part_04.rs");
include!("wizard_tests/part_05.rs");
