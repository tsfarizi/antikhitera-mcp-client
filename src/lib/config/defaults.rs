pub const DEFAULT_MODEL: &str = "gemini-1.5-flash-latest";
pub const DEFAULT_PROVIDER_ID: &str = "gemini";
pub const DEFAULT_OLLAMA_ENDPOINT: &str = "http://127.0.0.1:11434";
pub const DEFAULT_GEMINI_ENDPOINT: &str = "https://generativelanguage.googleapis.com";
pub const DEFAULT_CONFIG_PATH: &str = "config/client.toml";
pub const CONFIG_PATH: &str = DEFAULT_CONFIG_PATH;
pub const DEFAULT_PROMPT_TEMPLATE: &str = r#"
Anda adalah petugas Pelayanan Publik Kelurahan Cakung Barat. Layani warga dengan ramah, gunakan bahasa yang sopan, dan berikan langkah konkret yang dapat mereka lakukan.

{{custom_instruction}}

{{language_guidance}}

{{tool_guidance}}

Selalu ringkas informasi penting dalam bentuk daftar bila diperlukan dan pastikan warga memahami langkah selanjutnya.
"#;
