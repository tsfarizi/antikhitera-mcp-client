//! Scenario Test — Hands-on CLI Session (konfigurasi dari app.pc)
//!
//! Semua konfigurasi (provider, model, API key) diambil dari `app.pc`,
//! persis seperti saat pengguna menjalankan binary `antikythera`.
//!
//! Prasyarat: `app.pc` sudah dikonfigurasi.
//!   Jika belum: jalankan `task setup-config` atau `antikythera-config init`

use antikythera_cli::config::load_app_config;
use antikythera_cli::infrastructure::llm::providers_from_postcard;
use antikythera_cli::runtime::{build_runtime_client, materialize_runtime_config};
use antikythera_core::AppConfig;
use antikythera_core::application::client::ChatRequest;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::test]
async fn greet_then_ask_time() {
    let pc_config = match load_app_config(None) {
        Ok(cfg) => cfg,
        Err(_) => {
            eprintln!("[SKIP] app.pc not configured — skipping scenario test");
            return;
        }
    };

    let initial_providers = providers_from_postcard(&pc_config.providers);

    let provider_from_config = pc_config.model.default_provider.trim().to_string();
    let model_from_config = pc_config.model.model.trim().to_string();

    assert!(
        !provider_from_config.is_empty(),
        "app.pc belum dikonfigurasi: default_provider kosong.\n\
         Jalankan `antikythera-config set-model <provider> <model>`"
    );
    assert!(
        !model_from_config.is_empty(),
        "app.pc belum dikonfigurasi: model kosong.\n\
         Jalankan `antikythera-config set-model <provider> <model>`"
    );

    let toml_config = AppConfig::load(None).unwrap_or_default();
    let system_override = pc_config
        .custom
        .get("system_prompt")
        .cloned()
        .or_else(|| toml_config.system_prompt.clone());

    let (runtime_config, providers) = materialize_runtime_config(
        &toml_config,
        &initial_providers,
        Some(&provider_from_config),
        Some(&model_from_config),
        None,
        None,
        system_override.as_deref(),
    )
    .expect("materialize_runtime_config harus berhasil");

    let selected_provider = &runtime_config.default_provider;
    let selected_model = &runtime_config.model;

    eprintln!("[SCENARIO] Provider : {selected_provider}");
    eprintln!("[SCENARIO] Model    : {selected_model}");

    let client = build_runtime_client(
        &runtime_config,
        &providers,
        std::collections::HashMap::new(),
    )
    .expect("McpClient harus berhasil dibangun");

    let session_id = "scenario-test".to_string();

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

    assert!(!reply1.is_empty(), "reply sapaan tidak boleh kosong");
    assert!(!reply2.is_empty(), "reply waktu tidak boleh kosong");

    let scenario_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("scenario.txt");

    let mut out = String::new();
    out.push_str(
        "================================================================================\n",
    );
    out.push_str("SKENARIO: Percakapan Hands-on via CLI Runtime (konfigurasi dari app.pc)\n");
    out.push_str(&format!("Dijalankan pada Unix time : {now_before}\n"));
    out.push_str(&format!(
        "Provider                  : {selected_provider}\n"
    ));
    out.push_str(&format!("Model                     : {selected_model}\n"));
    out.push_str(
        "================================================================================\n\n",
    );

    out.push_str(
        "── PERCAKAPAN ────────────────────────────────────────────────────────────────\n\n",
    );
    out.push_str("USER      : Halo!\n\n");
    out.push_str(&format!("ASSISTANT : {reply1}\n\n"));
    out.push_str("USER      : Berapa waktu Unix sekarang (detik sejak epoch)?\n\n");
    out.push_str(&format!("ASSISTANT : {reply2}\n\n"));

    out.push_str(
        "── RINGKASAN ─────────────────────────────────────────────────────────────────\n\n",
    );
    out.push_str(&format!("Window waktu test : {now_before} – {now_after}\n"));
    out.push_str(&format!("Reply sapaan      : \"{reply1}\"\n"));
    out.push_str(&format!("Reply waktu       : \"{reply2}\"\n"));
    out.push_str("\nSemua assertions PASSED.\n");
    out.push_str(
        "================================================================================\n",
    );

    fs::write(&scenario_path, &out).unwrap_or_else(|e| panic!("gagal tulis scenario.txt: {e}"));

    assert!(
        scenario_path.exists(),
        "scenario.txt harus ada setelah test"
    );
    eprintln!("[SCENARIO] Ditulis ke: {}", scenario_path.display());
}
