//! Slash-command processor and runtime reconfiguration.

use std::collections::HashMap;
use std::sync::Arc;

use antikythera_core::ConfigLogger;
use antikythera_core::application::client::{ClientConfigSnapshot, McpClient};
use antikythera_core::infrastructure::model::DynamicModelProvider;

use crate::config::{
    AppConfig as PostcardAppConfig, ModelConfig as PostcardModelConfig, save_app_config,
};
use crate::infrastructure::llm::{ModelProviderConfig, providers_to_postcard};
use crate::presentation::tui::app::ChatApp;
use crate::presentation::tui::types::{
    SLASH_COMMANDS, UiMessage, UiTone, slash_command_suggestions,
};
use crate::runtime::{build_runtime_client, materialize_runtime_config};

pub(crate) fn process_command(
    app: &mut ChatApp,
    client: &mut Arc<McpClient<DynamicModelProvider>>,
    input: &str,
) {
    let command = input.trim_start_matches('/').trim();
    let mut parts = command.split_whitespace();
    let name = parts.next().unwrap_or_default().to_ascii_lowercase();
    let args: Vec<&str> = parts.collect();

    match name.as_str() {
        "help" | "?" => {
            app.status = "Bantuan command diperbarui di panel chat.".to_string();
            app.push_message(UiMessage::new(
                "Slash Commands",
                SLASH_COMMANDS
                    .iter()
                    .map(|(name, description)| format!("/{name:<10} {description}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
                UiTone::System,
            ));
        }
        "providers" | "provider" => {
            app.status = "Daftar provider tersedia ditampilkan.".to_string();
            app.push_message(UiMessage::new(
                "Providers",
                render_provider_catalog(&app.providers, &app.provider, &app.model),
                UiTone::System,
            ));
        }
        "use" => {
            let Some(provider_input) = args.first().copied() else {
                app.push_message(UiMessage::new(
                    "Command Error",
                    "Gunakan /use <provider> [model]. Contoh: /use openai gpt-4o-mini",
                    UiTone::Error,
                ));
                return;
            };

            match apply_provider_selection(app, provider_input, args.get(1).copied()) {
                Ok(message) => {
                    if let Err(error) = reconfigure_runtime(app, client) {
                        app.status = "Gagal menerapkan backend runtime.".to_string();
                        app.push_message(UiMessage::new("Command Error", error, UiTone::Error));
                        return;
                    }
                    app.status = format!(
                        "Backend aktif diperbarui ke {}/{}.",
                        app.provider, app.model
                    );
                    app.push_message(UiMessage::new("Runtime Updated", message, UiTone::System));
                }
                Err(error) => {
                    app.status = "Gagal mengganti provider/model.".to_string();
                    app.push_message(UiMessage::new("Command Error", error, UiTone::Error));
                }
            }
        }
        "model" => {
            let Some(model_input) = args.first().copied() else {
                app.push_message(UiMessage::new(
                    "Command Error",
                    "Gunakan /model <nama-model>. Contoh: /model gemini-2.0-flash",
                    UiTone::Error,
                ));
                return;
            };

            match apply_model_selection(app, model_input) {
                Ok(message) => {
                    if let Err(error) = reconfigure_runtime(app, client) {
                        app.status = "Gagal menerapkan backend runtime.".to_string();
                        app.push_message(UiMessage::new("Command Error", error, UiTone::Error));
                        return;
                    }
                    app.status = format!("Model aktif diperbarui ke {}.", app.model);
                    app.push_message(UiMessage::new("Runtime Updated", message, UiTone::System));
                }
                Err(error) => {
                    app.status = "Gagal mengganti model aktif.".to_string();
                    app.push_message(UiMessage::new("Command Error", error, UiTone::Error));
                }
            }
        }
        "config" => {
            app.status = "Ringkasan konfigurasi aktif ditampilkan.".to_string();
            app.push_message(UiMessage::new(
                "Config Snapshot",
                render_config_snapshot(&app.snapshot),
                UiTone::System,
            ));
        }
        "tools" => {
            let body = if client.tools().is_empty() {
                "Tidak ada tool yang aktif pada sesi ini.".to_string()
            } else {
                client
                    .tools()
                    .iter()
                    .map(|tool| format!("- {}", tool.name))
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            app.status = "Daftar tools aktif ditampilkan.".to_string();
            app.push_message(UiMessage::new("Tools", body, UiTone::System));
        }
        "agent" => {
            let next_mode = match args.first().copied() {
                Some("on") => true,
                Some("off") => false,
                Some("toggle") | None => !app.agent_mode,
                Some(other) => {
                    app.push_message(UiMessage::new(
                        "Command Error",
                        format!(
                            "Argumen /agent '{}' tidak dikenal. Gunakan on, off, atau toggle.",
                            other
                        ),
                        UiTone::Error,
                    ));
                    return;
                }
            };
            app.agent_mode = next_mode;
            app.status = if next_mode {
                "Mode agent aktif.".to_string()
            } else {
                "Mode chat langsung aktif.".to_string()
            };
            app.push_message(UiMessage::new(
                "Mode Updated",
                if next_mode {
                    "Prompt berikutnya akan dieksekusi melalui loop agent."
                } else {
                    "Prompt berikutnya akan langsung dikirim ke model tanpa loop agent."
                },
                UiTone::System,
            ));
        }
        "reset" | "clear" => app.reset_session(),
        "history" => {
            let sessions = app.history_store.list_sessions();
            app.history.open_and_refresh_with(sessions);
            app.status =
                "Riwayat Chat. ↑↓=navigasi | Enter=lihat | d=hapus | r=ganti judul | Esc=tutup"
                    .to_string();
        }
        "exit" | "quit" => {
            app.status = "Menutup TUI...".to_string();
            app.should_quit = true;
        }
        other => {
            app.status = "Command tidak dikenal.".to_string();
            let suggestion_text = slash_command_suggestions(&format!("/{other}"))
                .into_iter()
                .map(|(name, description)| format!("/{name} - {description}"))
                .collect::<Vec<_>>()
                .join("\n");
            let body = if suggestion_text.is_empty() {
                format!("Perintah '/{other}' tidak dikenal. Gunakan /help untuk daftar command.")
            } else {
                format!(
                    "Perintah '/{other}' tidak dikenal. Mungkin yang Anda maksud:\n{}",
                    suggestion_text
                )
            };
            app.push_message(UiMessage::new("Command Error", body, UiTone::Error));
        }
    }
}

pub fn render_provider_catalog(
    providers: &[ModelProviderConfig],
    active_provider: &str,
    active_model: &str,
) -> String {
    providers
        .iter()
        .map(|provider| {
            let marker = if provider.id == active_provider {
                "*"
            } else {
                " "
            };
            let models = provider
                .models
                .iter()
                .map(|model| {
                    if provider.id == active_provider && model.name == active_model {
                        format!("{} (aktif)", model.name)
                    } else {
                        model.name.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "{marker} {} [{}]\n  endpoint: {}\n  models  : {}",
                provider.id, provider.provider_type, provider.endpoint, models
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub fn render_config_snapshot(snapshot: &ClientConfigSnapshot) -> String {
    [
        format!("default provider : {}", snapshot.default_provider),
        format!("default model    : {}", snapshot.model),
        format!(
            "system prompt    : {}",
            snapshot.system_prompt.as_deref().unwrap_or("<none>")
        ),
        format!("servers          : {}", snapshot.servers.len()),
        format!("tools            : {}", snapshot.tools.len()),
        format!("template chars   : {}", snapshot.prompt_template.len()),
    ]
    .join("\n")
}

fn apply_provider_selection(
    app: &mut ChatApp,
    provider_input: &str,
    model_input: Option<&str>,
) -> Result<String, String> {
    let (provider, model) = resolve_provider_selection(
        &app.providers,
        &app.provider,
        &app.model,
        provider_input,
        model_input,
    )?;
    apply_runtime_selection(app, provider, model)
}

fn apply_model_selection(app: &mut ChatApp, model_input: &str) -> Result<String, String> {
    apply_runtime_selection(app, app.provider.clone(), model_input.trim().to_string())
}

pub(crate) fn apply_runtime_selection(
    app: &mut ChatApp,
    provider: String,
    model: String,
) -> Result<String, String> {
    let (updated_config, updated_providers) = materialize_runtime_config(
        &app.runtime_config,
        &app.providers,
        Some(&provider),
        Some(&model),
        None,
        None,
        app.runtime_config.system_prompt.as_deref(),
    )
    .map_err(|error| error.to_string())?;

    app.runtime_config = updated_config;
    app.providers = updated_providers;
    app.provider = app.runtime_config.default_provider.clone();
    app.model = app.runtime_config.model.clone();
    app.session_id = None;
    antikythera_core::set_active_session("tui");
    // Provider/model changed — start a fresh history session next turn.
    app.current_history_session = None;

    Ok(format!(
        "Provider/model aktif sekarang {}/{}. Sesi percakapan direset agar riwayat tidak tercampur antar backend.",
        app.provider, app.model
    ))
}

pub fn resolve_provider_selection(
    providers: &[ModelProviderConfig],
    current_provider: &str,
    current_model: &str,
    provider_input: &str,
    model_input: Option<&str>,
) -> Result<(String, String), String> {
    let provider = find_provider(providers, provider_input).ok_or_else(|| {
        format!(
            "Provider '{}' tidak ditemukan. Gunakan /providers untuk melihat backend yang tersedia.",
            provider_input.trim()
        )
    })?;

    let fallback_model =
        if provider.id.eq_ignore_ascii_case(current_provider) && !current_model.trim().is_empty() {
            current_model.trim().to_string()
        } else {
            provider
                .models
                .first()
                .map(|candidate| candidate.name.clone())
                .unwrap_or_else(|| current_model.trim().to_string())
        };

    let model = model_input
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or(fallback_model);

    if model.is_empty() {
        return Err(format!(
            "Provider '{}' belum memiliki model default. Tambahkan nama model secara eksplisit, misalnya /use {} <model>.",
            provider.id, provider.id
        ));
    }

    Ok((provider.id.clone(), model))
}

pub(crate) fn reconfigure_runtime(
    app: &mut ChatApp,
    client: &mut Arc<McpClient<DynamicModelProvider>>,
) -> Result<(), String> {
    ConfigLogger::new(&antikythera_core::get_active_session()).info(format!(
        "CLI reconfiguring runtime | new_provider={} new_model={}",
        app.provider, app.model
    ));

    // Build a PostcardAppConfig to persist — merge core routing fields with CLI providers.
    // Convert runtime PromptsConfig (Option<String> fields) to the postcard form (String fields).
    let postcard_prompts = {
        use crate::config::PromptsConfig as PcPrompts;
        let defaults = PcPrompts::default();
        let r = &app.runtime_config.prompts;
        PcPrompts {
            template: r.template.clone().unwrap_or(defaults.template),
            tool_guidance: r.tool_guidance.clone().unwrap_or(defaults.tool_guidance),
            fallback_guidance: r
                .fallback_guidance
                .clone()
                .unwrap_or(defaults.fallback_guidance),
            json_retry_message: r
                .json_retry_message
                .clone()
                .unwrap_or(defaults.json_retry_message),
            tool_result_instruction: r
                .tool_result_instruction
                .clone()
                .unwrap_or(defaults.tool_result_instruction),
            agent_instructions: r
                .agent_instructions
                .clone()
                .unwrap_or(defaults.agent_instructions),
            ui_instructions: r
                .ui_instructions
                .clone()
                .unwrap_or(defaults.ui_instructions),
            language_instructions: r
                .language_instructions
                .clone()
                .unwrap_or(defaults.language_instructions),
            agent_max_steps_error: r
                .agent_max_steps_error
                .clone()
                .unwrap_or(defaults.agent_max_steps_error),
            no_tools_guidance: r
                .no_tools_guidance
                .clone()
                .unwrap_or(defaults.no_tools_guidance),
            fallback_response_keys: r
                .fallback_response_keys
                .clone()
                .unwrap_or(defaults.fallback_response_keys),
        }
    };
    // Persist system_prompt in the extensible custom map (PostcardAppConfig has no dedicated field).
    let mut custom = HashMap::new();
    if let Some(sp) = &app.runtime_config.system_prompt {
        custom.insert("system_prompt".to_string(), sp.clone());
    }
    let pc = PostcardAppConfig {
        model: PostcardModelConfig {
            default_provider: app.runtime_config.default_provider.clone(),
            model: app.runtime_config.model.clone(),
        },
        providers: providers_to_postcard(app.providers.clone()),
        prompts: postcard_prompts,
        custom,
        ..Default::default()
    };
    save_app_config(&pc, None).map_err(|error| error.to_string())?;

    let new_client = build_runtime_client(
        &app.runtime_config,
        &app.providers,
        app.builtin_transports.clone(),
    )
    .map_err(|error| error.to_string())?;
    app.snapshot = new_client.config_snapshot();
    app.tools = new_client.tools().len();
    *client = new_client;

    Ok(())
}

pub fn find_provider<'a>(
    providers: &'a [ModelProviderConfig],
    provider_input: &str,
) -> Option<&'a ModelProviderConfig> {
    let needle = provider_input.trim();
    providers
        .iter()
        .find(|provider| provider.id.eq_ignore_ascii_case(needle))
}

