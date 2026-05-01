//! Scenario Integration Test — Hands-on CLI Session
//!
//! Test ini mensimulasikan penggunaan CLI secara nyata: konfigurasi, provider,
//! dan model dipilih **sama persis** seperti yang dilakukan binary `antikythera`
//! saat dijalankan pengguna. Tidak ada mock LLM atau hardcoded provider.
//!
//! Urutan resolusi (identik dengan binary CLI):
//!   1. Load `.env` dari root repo untuk API key
//!   2. `AppConfig::load()` dari TOML (atau default jika belum ada)
//!   3. `load_app_config()` untuk provider catalog dari `app.pc`
//!   4. `materialize_runtime_config()` — pilih provider via `detect_provider_from_env`
//!   5. `build_runtime_client()` — bangun `McpClient` identik dengan binary
//!   6. `McpClient::chat()` — API yang dipakai event loop TUI
//!
//! Jalankan:
//!   cargo test -p antikythera-cli --test scenario -- --nocapture
//!
//! Syarat: setidaknya satu provider tersedia:
//!   GEMINI_API_KEY, OPENAI_API_KEY, atau Ollama di localhost:11434

use antikythera_cli::config::load_app_config;
use antikythera_cli::infrastructure::llm::providers_from_postcard;
use antikythera_cli::runtime::{build_runtime_client, materialize_runtime_config};
use antikythera_core::application::client::ChatRequest;
use antikythera_core::AppConfig;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

// ── Helper: bangun runtime persis seperti binary CLI ─────────────────────────

fn build_cli_runtime() -> (
    antikythera_core::AppConfig,
    Vec<antikythera_cli::infrastructure::llm::ModelProviderConfig>,
) {
    // 1. Load .env dari root repo
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    dotenvy::from_path(root.join(".env")).ok();

    // 2. Load TOML config (AppConfig) — fallback ke default jika belum ada
    let toml_config = AppConfig::load(None).unwrap_or_default();

    // 3. Load provider catalog dari app.pc — fallback ke kosong jika belum ada
    let pc_config = load_app_config(None).unwrap_or_default();
    let initial_providers = providers_from_postcard(&pc_config.providers);

    // 4. Resolve provider/model override dari app.pc (sama seperti menu.rs)
    let provider_override = {
        let p = pc_config.model.default_provider.trim().to_string();
        if p.is_empty() { None } else { Some(p) }
    };
    let model_override = {
        let m = pc_config.model.model.trim().to_string();
        if m.is_empty() { None } else { Some(m) }
    };
    let system_override = pc_config.custom.get("system_prompt").cloned()
        .or_else(|| toml_config.system_prompt.clone());

    // 5. materialize — detect_provider_from_env() dipanggil secara internal
    materialize_runtime_config(
        &toml_config,
        &initial_providers,
        provider_override.as_deref(),
        model_override.as_deref(),
        None, // endpoint_override
        None, // ollama_url
        system_override.as_deref(),
    )
    .expect("CLI runtime config harus berhasil dibangun")
}

// ── Skenario ──────────────────────────────────────────────────────────────────

/// Skenario hands-on: sapa CLI → tanya waktu Unix.
///
/// Provider dan model dipilih otomatis dari environment, identik dengan saat
/// pengguna menjalankan binary `antikythera`.
#[tokio::test]
async fn scenario_greet_then_ask_time() {
    let (runtime_config, providers) = build_cli_runtime();

    let selected_provider = runtime_config.default_provider.clone();
    let selected_model = runtime_config.model.clone();

    eprintln!("[SCENARIO] Provider : {selected_provider}");
    eprintln!("[SCENARIO] Model    : {selected_model}");

    // 6. Build McpClient — identik dengan build_runtime_client di menu.rs
    let client = build_runtime_client(&runtime_config, &providers)
        .expect("McpClient harus berhasil dibangun");

    let session_id = "scenario-test".to_string();

    // ── Giliran 1: sapaan ─────────────────────────────────────────────────────
    eprintln!("[USER] Halo!");
    let result1 = client
        .chat(ChatRequest {
            prompt: "Halo!".to_string(),
            attachments: vec![],
            system_prompt: Some(
                "Kamu adalah asisten CLI berbahasa Indonesia. Jawab singkat dan ramah.".to_string(),
            ),
            session_id: Some(session_id.clone()),
            raw_mode: false,
            bypass_template: false,
            force_json: false,
        })
        .await
        .expect("giliran sapaan harus berhasil");

    let reply1 = result1.content.clone();
    eprintln!("[ASSISTANT] {reply1}");

    // ── Giliran 2: tanya waktu Unix ───────────────────────────────────────────
    eprintln!("[USER] Berapa waktu Unix sekarang (detik sejak epoch)?");
    let now_before = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let result2 = client
        .chat(ChatRequest {
            prompt: "Berapa waktu Unix sekarang (detik sejak epoch)?".to_string(),
            attachments: vec![],
            system_prompt: Some(
                "Kamu adalah asisten CLI berbahasa Indonesia. \
                 Jawab dengan menyebutkan nilai Unix timestamp saat ini dalam detik."
                    .to_string(),
            ),
            session_id: Some(session_id.clone()),
            raw_mode: false,
            bypass_template: false,
            force_json: false,
        })
        .await
        .expect("giliran tanya waktu harus berhasil");

    let now_after = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let reply2 = result2.content.clone();
    eprintln!("[ASSISTANT] {reply2}");

    // ── Assertions ────────────────────────────────────────────────────────────
    assert!(!reply1.is_empty(), "reply sapaan tidak boleh kosong");
    assert!(!reply2.is_empty(), "reply waktu tidak boleh kosong");

    // ── Tulis scenario.txt ke root repo ───────────────────────────────────────
    let scenario_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("scenario.txt");

    let mut out = String::new();
    out.push_str("================================================================================\n");
    out.push_str("SKENARIO: Percakapan Hands-on via CLI Runtime\n");
    out.push_str(&format!("Dijalankan pada Unix time : {now_before}\n"));
    out.push_str(&format!("Provider                  : {selected_provider}\n"));
    out.push_str(&format!("Model                     : {selected_model}\n"));
    out.push_str("================================================================================\n\n");

    out.push_str("── PERCAKAPAN ────────────────────────────────────────────────────────────────\n\n");
    out.push_str("USER      : Halo!\n\n");
    out.push_str(&format!("ASSISTANT : {reply1}\n\n"));
    out.push_str("USER      : Berapa waktu Unix sekarang (detik sejak epoch)?\n\n");
    out.push_str(&format!("ASSISTANT : {reply2}\n\n"));

    out.push_str("── RINGKASAN ─────────────────────────────────────────────────────────────────\n\n");
    out.push_str(&format!("Window waktu test : {now_before} – {now_after}\n"));
    out.push_str(&format!("Reply sapaan      : \"{reply1}\"\n"));
    out.push_str(&format!("Reply waktu       : \"{reply2}\"\n"));
    out.push_str("\nSemua assertions PASSED.\n");
    out.push_str("================================================================================\n");

    fs::write(&scenario_path, &out)
        .unwrap_or_else(|e| panic!("gagal tulis scenario.txt: {e}"));

    assert!(scenario_path.exists(), "scenario.txt harus ada setelah test");
    eprintln!("[SCENARIO] Ditulis ke: {}", scenario_path.display());
}
