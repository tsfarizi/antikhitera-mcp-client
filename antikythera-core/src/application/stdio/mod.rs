mod stdio_render;
mod tool_detection;

use self::stdio_render::{
    print_banner, print_command_recommendations, print_help, print_logs, print_tool_steps, prompt,
    show_config, write_line,
};
use self::tool_detection::looks_like_tool_call;
use crate::application::agent::{Agent, AgentOptions, AgentOutcome, AgentStep};
use crate::application::client::{ChatRequest, ChatResult, McpClient};
use crate::application::model_provider::ModelProvider;
use crate::config::AppConfig;
use crate::logging::StdioLogger;
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Debug, Error)]
pub enum StdioError {
    #[error("stdin/stdout I/O error: {0}")]
    Io(#[from] std::io::Error),
}

struct SessionState {
    session_id: Option<String>,
    agent_mode: bool,
    last_logs: Vec<String>,
    last_steps: Vec<AgentStep>,
}

impl SessionState {
    fn new() -> Self {
        Self {
            session_id: None,
            agent_mode: true,
            last_logs: Vec::new(),
            last_steps: Vec::new(),
        }
    }

    fn reset(&mut self) {
        self.session_id = None;
        self.last_logs.clear();
        self.last_steps.clear();
    }

    fn update_session(&mut self, session_id: String) -> bool {
        let changed = self
            .session_id
            .as_ref()
            .map(|current| current != &session_id)
            .unwrap_or(true);
        self.session_id = Some(session_id);
        changed
    }

    fn record_logs(&mut self, logs: Vec<String>) {
        self.last_logs = logs;
    }

    fn clear_logs(&mut self) {
        self.last_logs.clear();
    }

    fn record_steps(&mut self, steps: Vec<AgentStep>) {
        self.last_steps = steps;
    }

    fn clear_steps(&mut self) {
        self.last_steps.clear();
    }

    fn has_logs(&self) -> bool {
        !self.last_logs.is_empty()
    }

    fn logs(&self) -> &[String] {
        &self.last_logs
    }

    fn has_steps(&self) -> bool {
        !self.last_steps.is_empty()
    }

    fn steps(&self) -> &[AgentStep] {
        &self.last_steps
    }
}

enum LoopControl {
    Continue,
    Exit,
}

const ACCENT: &str = "\x1b[36m";
const SUCCESS: &str = "\x1b[32m";
const WARN: &str = "\x1b[33m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

const KNOWN_COMMANDS: [&str; 10] = [
    "help",
    "config",
    "config edit",
    "log",
    "steps",
    "agent",
    "reset",
    "reload",
    "exit",
    "quit",
];

pub async fn run<P>(client: Arc<McpClient<P>>) -> Result<(), StdioError>
where
    P: ModelProvider + 'static,
{
    let mut stdout = io::stdout();
    let stdin = BufReader::new(io::stdin());
    let mut lines = stdin.lines();
    let mut state = SessionState::new();

    print_banner(&mut stdout).await?;
    print_help(&mut stdout).await?;

    loop {
        prompt(&mut stdout, &state).await?;
        let line = match lines.next_line().await? {
            Some(line) => line,
            None => {
                write_line(
                    &mut stdout,
                    "\nInput STDIN ditutup. Keluar dari mode STDIO.",
                )
                .await?;
                break;
            }
        };

        let input = line.trim();
        if input.is_empty() {
            continue;
        }

        if matches!(input.chars().next(), Some('/') | Some(':')) {
            match handle_command(input, &mut state, client.as_ref(), &mut stdout).await? {
                LoopControl::Continue => continue,
                LoopControl::Exit => break,
            }
        } else {
            handle_prompt(client.clone(), &mut state, input.to_string(), &mut stdout).await?;
        }
    }

    stdout.flush().await?;
    Ok(())
}

async fn handle_command<P: ModelProvider>(
    input: &str,
    state: &mut SessionState,
    client: &McpClient<P>,
    stdout: &mut io::Stdout,
) -> Result<LoopControl, StdioError> {
    let command = input.trim_start_matches(['/', ':']);
    let mut parts = command.split_whitespace();
    let name = parts.next().unwrap_or("").to_ascii_lowercase();
    let args: Vec<String> = parts.map(|part| part.to_string()).collect();

    StdioLogger::new("stdio").debug(format!("Processing STDIO command | command={}", name));

    match name.as_str() {
        "" => {
            print_command_recommendations(stdout, "").await?;
            Ok(LoopControl::Continue)
        }
        "help" | "?" => {
            print_help(stdout).await?;
            Ok(LoopControl::Continue)
        }
        "exit" | "quit" | "keluar" | "q" => {
            write_line(stdout, "Menutup mode STDIO.").await?;
            Ok(LoopControl::Exit)
        }
        "reset" | "clear" => {
            state.reset();
            write_line(stdout, "Riwayat sesi dihapus. Mulai sesi baru.").await?;
            Ok(LoopControl::Continue)
        }
        "reload" => {
            write_line(stdout, "\nMemuat ulang konfigurasi...").await?;
            match AppConfig::load(Some(Path::new(crate::config::CONFIG_PATH))) {
                Ok(config) => {
                    write_line(stdout, "Konfigurasi berhasil dimuat dari file.").await?;
                    write_line(stdout, &format!("- Provider: {}", config.default_provider)).await?;
                    write_line(stdout, &format!("- Model: {}", config.model)).await?;
                    write_line(
                        stdout,
                        &format!(
                            "- Prompt template: {} characters",
                            config.prompt_template().len()
                        ),
                    )
                    .await?;
                    write_line(stdout, &format!("- Servers: {}", config.servers.len())).await?;
                    write_line(stdout, &format!("- Tools: {}", config.tools.len())).await?;
                    write_line(stdout, "").await?;
                    write_line(
                        stdout,
                        "CATATAN: Perubahan konfigurasi akan berlaku setelah restart aplikasi.",
                    )
                    .await?;
                    write_line(
                        stdout,
                        "Untuk menerapkan konfigurasi baru, gunakan /exit lalu jalankan ulang.",
                    )
                    .await?;
                }
                Err(error) => {
                    write_line(stdout, &format!("Gagal memuat konfigurasi: {}", error)).await?;
                }
            }
            Ok(LoopControl::Continue)
        }
        "agent" => {
            let action = args.first().map(|value| value.to_ascii_lowercase());
            let new_mode = match action.as_deref() {
                Some("on") => true,
                Some("off") => false,
                Some("toggle") | None => !state.agent_mode,
                Some(other) => {
                    write_line(
                        stdout,
                        &format!("Nilai agent '{other}' tidak dikenal. Gunakan on/off/toggle."),
                    )
                    .await?;
                    return Ok(LoopControl::Continue);
                }
            };
            state.agent_mode = new_mode;
            write_line(
                stdout,
                if state.agent_mode {
                    "Mode agent aktif. Pesan berikutnya akan menjalankan alur agent."
                } else {
                    "Mode chat langsung aktif. Pesan berikutnya dikirim langsung ke model."
                },
            )
            .await?;
            Ok(LoopControl::Continue)
        }
        "config" => {
            let action = args.first().map(|v| v.to_ascii_lowercase());
            match action.as_deref() {
                Some("edit") => {
                    #[cfg(feature = "wizard")]
                    {
                        match crate::config::wizard::run_setup_menu().await {
                            Ok(_) => {
                                write_line(stdout, "\nKembali ke mode STDIO.").await?;
                            }
                            Err(e) => {
                                write_line(stdout, &format!("Error dalam editor: {}", e)).await?;
                            }
                        }
                    }

                    #[cfg(not(feature = "wizard"))]
                    {
                        write_line(
                            stdout,
                            "Fitur wizard tidak aktif pada build ini. Gunakan --features wizard untuk mengedit config interaktif.",
                        )
                        .await?;
                    }
                }
                _ => {
                    show_config(stdout, client).await?;
                }
            }
            Ok(LoopControl::Continue)
        }
        "log" | "logs" => {
            if state.has_logs() {
                print_logs(stdout, state.logs()).await?;
            } else {
                write_line(stdout, "Belum ada log dari interaksi terakhir.").await?;
            }
            Ok(LoopControl::Continue)
        }
        "steps" | "tool" | "toolsteps" => {
            if state.has_steps() {
                print_tool_steps(stdout, state.steps()).await?;
            } else {
                write_line(stdout, "Belum ada eksekusi tool pada interaksi terakhir.").await?;
            }
            Ok(LoopControl::Continue)
        }
        other => {
            write_line(
                stdout,
                &format!("Perintah '{other}' tidak dikenal. Gunakan /help untuk bantuan."),
            )
            .await?;
            print_command_recommendations(stdout, other).await?;
            Ok(LoopControl::Continue)
        }
    }
}

async fn handle_prompt<P: ModelProvider + 'static>(
    client: Arc<McpClient<P>>,
    state: &mut SessionState,
    message: String,
    stdout: &mut io::Stdout,
) -> Result<(), StdioError> {
    let log = StdioLogger::new(state.session_id.as_deref().unwrap_or("stdio"));
    if state.agent_mode {
        log.info("Processing interactive STDIO request in agent mode");
        let options = AgentOptions {
            session_id: state.session_id.clone(),
            ..AgentOptions::default()
        };
        run_agent_interaction(client, state, message, stdout, options).await?;
    } else {
        log.info("Processing interactive STDIO chat request");
        let direct_prompt = message.clone();
        match client
            .chat(ChatRequest {
                prompt: message,
                attachments: Vec::new(),
                system_prompt: None,
                session_id: state.session_id.clone(),
                raw_mode: false,
                bypass_template: false,
                force_json: false,
            })
            .await
        {
            Ok(result) => {
                let ChatResult {
                    content,
                    session_id,
                    logs,
                    ..
                } = result;

                if looks_like_tool_call(&content) {
                    write_line(
                        stdout,
                        "\nRespons model memerlukan eksekusi tool. Mengalihkan ke mode agent otomatis.",
                    )
                    .await?;
                    state.reset();
                    let options = AgentOptions::default();
                    run_agent_interaction(client, state, direct_prompt, stdout, options).await?;
                    stdout.flush().await?;
                    return Ok(());
                }

                let changed = state.update_session(session_id.clone());
                if changed {
                    write_line(stdout, &format!("\nSession aktif: {}", session_id)).await?;
                } else {
                    write_line(stdout, "").await?;
                }
                write_line(stdout, "Assistant:").await?;
                write_line(stdout, &content).await?;
                state.clear_steps();
                if logs.is_empty() {
                    state.clear_logs();
                } else {
                    state.record_logs(logs);
                    write_line(stdout, "").await?;
                    write_line(stdout, "(Gunakan /log untuk melihat log terbaru.)").await?;
                }
            }
            Err(err) => {
                log.error(format!("STDIO chat request failed | error={}", err));
                write_line(stdout, "\nPermintaan gagal:").await?;
                write_line(stdout, &err.user_message()).await?;
                state.clear_logs();
                state.clear_steps();
            }
        }
    }

    stdout.flush().await?;
    Ok(())
}

async fn run_agent_interaction<P>(
    client: Arc<McpClient<P>>,
    state: &mut SessionState,
    prompt: String,
    stdout: &mut io::Stdout,
    options: AgentOptions,
) -> Result<(), StdioError>
where
    P: ModelProvider + 'static,
{
    let agent = Agent::new(client.clone());
    match agent.run(prompt, options).await {
        Ok(AgentOutcome {
            logs,
            session_id,
            response,
            steps,
        }) => {
            let changed = state.update_session(session_id.clone());
            if changed {
                write_line(
                    stdout,
                    &format!("\nSession aktif diperbarui: {}", session_id),
                )
                .await?;
            } else {
                write_line(stdout, "").await?;
            }
            write_line(stdout, "Agent:").await?;
            let response_str = match response {
                Value::String(s) => s,
                v => serde_json::to_string(&v).unwrap_or_default(),
            };
            write_line(stdout, &response_str).await?;
            if steps.is_empty() {
                state.clear_steps();
            } else {
                state.record_steps(steps);
            }
            if logs.is_empty() {
                state.clear_logs();
            } else {
                state.record_logs(logs);
            }
        }
        Err(err) => {
            StdioLogger::new(state.session_id.as_deref().unwrap_or("stdio"))
                .error(format!("Agent processing failed via STDIO | error={}", err));
            write_line(stdout, "\nAgent mengalami kegagalan:").await?;
            write_line(stdout, &err.user_message()).await?;
            state.clear_logs();
            state.clear_steps();
        }
    }

    Ok(())
}

pub fn suggest_commands(prefix: &str) -> Vec<&'static str> {
    let normalized = prefix.trim().to_ascii_lowercase();
    let mut suggestions: Vec<&'static str> = if normalized.is_empty() {
        KNOWN_COMMANDS.to_vec()
    } else {
        KNOWN_COMMANDS
            .iter()
            .copied()
            .filter(|cmd| cmd.starts_with(&normalized) || cmd.contains(&normalized))
            .collect()
    };
    suggestions.sort_unstable();
    suggestions.truncate(6);
    suggestions
}
