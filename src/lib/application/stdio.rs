use crate::agent::{Agent, AgentOptions, AgentOutcome, AgentStep};
use crate::client::{ChatRequest, McpClient};
use crate::config::{AppConfig, CONFIG_PATH};
use crate::model::ModelProvider;
use serde_json::{Value, to_string_pretty};
use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, error, info, warn};

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
    let command = input.trim_start_matches(|c| c == '/' || c == ':');
    let mut parts = command.split_whitespace();
    let name = parts.next().unwrap_or("").to_ascii_lowercase();
    let args: Vec<String> = parts.map(|part| part.to_string()).collect();

    debug!(command = %name, "Processing STDIO command");

    match name.as_str() {
        "" => Ok(LoopControl::Continue),
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
            match AppConfig::load(Some(Path::new(CONFIG_PATH))) {
                Ok(config) => {
                    write_line(stdout, "Konfigurasi berhasil dimuat dari file.").await?;
                    write_line(stdout, &format!("- Provider: {}", config.default_provider)).await?;
                    write_line(stdout, &format!("- Model: {}", config.model)).await?;
                    write_line(
                        stdout,
                        &format!(
                            "- Prompt template: {} karakter",
                            config.prompt_template.len()
                        ),
                    )
                    .await?;
                    write_line(stdout, &format!("- Providers: {}", config.providers.len())).await?;
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
                    // Run the setup menu
                    match crate::config::wizard::run_setup_menu().await {
                        Ok(_) => {
                            write_line(stdout, "\nKembali ke mode STDIO.").await?;
                        }
                        Err(e) => {
                            write_line(stdout, &format!("Error dalam editor: {}", e)).await?;
                        }
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
    if state.agent_mode {
        info!("Processing interactive STDIO request in agent mode");
        let mut options = AgentOptions::default();
        options.session_id = state.session_id.clone();
        run_agent_interaction(client, state, message, stdout, options).await?;
    } else {
        info!("Processing interactive STDIO chat request");
        let direct_prompt = message.clone();
        match client
            .chat(ChatRequest {
                prompt: message,
                provider: None,
                model: None,
                system_prompt: None,
                session_id: state.session_id.clone(),
            })
            .await
        {
            Ok(result) => {
                let crate::client::ChatResult {
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
                error!(%err, "STDIO chat request failed");
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

async fn run_agent_interaction<P: ModelProvider>(
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
            write_line(stdout, &response).await?;
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
            error!(%err, "Agent processing failed via STDIO");
            write_line(stdout, "\nAgent mengalami kegagalan:").await?;
            write_line(stdout, &err.user_message()).await?;
            state.clear_logs();
            state.clear_steps();
        }
    }

    Ok(())
}

async fn show_config<P: ModelProvider>(
    stdout: &mut io::Stdout,
    client: &McpClient<P>,
) -> Result<(), StdioError> {
    let snapshot = client.config_snapshot();

    write_line(stdout, "\n=== Konfigurasi Aktif ===").await?;
    write_line(
        stdout,
        &format!("- Default provider : {}", snapshot.default_provider),
    )
    .await?;
    write_line(stdout, &format!("- Model            : {}", snapshot.model)).await?;
    match snapshot.system_prompt.as_deref() {
        Some(value) => {
            write_line(stdout, &format!("- System prompt    : {}", preview(value))).await?
        }
        None => write_line(stdout, "- System prompt    : (tidak disetel)").await?,
    }
    write_line(
        stdout,
        &format!(
            "- Prompt template  : {}",
            preview(&snapshot.prompt_template)
        ),
    )
    .await?;

    if snapshot.tools.is_empty() {
        write_line(stdout, "- Tools            : (tidak ada)").await?;
    } else {
        write_line(stdout, "- Tools:").await?;
        for tool in &snapshot.tools {
            let mut line = format!("  - {}", tool.name);
            if let Some(description) = &tool.description {
                line.push_str(&format!(" - {}", description));
            }
            if let Some(server) = &tool.server {
                line.push_str(&format!(" [server: {server}]"));
            }
            write_line(stdout, &line).await?;
        }
    }

    if snapshot.servers.is_empty() {
        write_line(stdout, "- MCP servers      : (tidak ada)").await?;
    } else {
        write_line(stdout, "- MCP servers:").await?;
        for server in &snapshot.servers {
            let mut line = format!("  - {} -> {}", server.name, server.command.display());
            if !server.args.is_empty() {
                line.push_str(&format!(" {}", server.args.join(" ")));
            }
            write_line(stdout, &line).await?;
        }
    }

    if snapshot.providers.is_empty() {
        write_line(stdout, "- Providers        : (tidak ada)").await?;
    } else {
        write_line(stdout, "- Providers:").await?;
        for provider in &snapshot.providers {
            let mut line = format!(
                "  - {} [{}] -> {}",
                provider.id, provider.provider_type, provider.endpoint
            );
            if !provider.models.is_empty() {
                let names: Vec<&str> = provider
                    .models
                    .iter()
                    .map(|model| model.name.as_str())
                    .collect();
                line.push_str(&format!(" (models: {})", names.join(", ")));
            }
            write_line(stdout, &line).await?;
        }
    }

    write_line(
        stdout,
        &format!("\n=== Berkas {} ===", Path::new(CONFIG_PATH).display()),
    )
    .await?;

    match fs::read_to_string(Path::new(CONFIG_PATH)) {
        Ok(raw) => {
            if raw.is_empty() {
                write_line(stdout, "(Berkas kosong)").await?;
            } else {
                for line in raw.lines() {
                    write_line(stdout, line).await?;
                }
            }
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {
            write_line(
                stdout,
                "(Berkas belum tersedia. Menampilkan konfigurasi aktif dalam bentuk TOML.)",
            )
            .await?;
            for line in snapshot.raw.lines() {
                write_line(stdout, line).await?;
            }
        }
        Err(error) => {
            warn!(%error, "Gagal membaca berkas konfigurasi");
            write_line(
                stdout,
                &format!("Gagal membaca berkas konfigurasi: {error}"),
            )
            .await?;
            write_line(
                stdout,
                "Konfigurasi aktif tetap seperti yang ditampilkan di atas.",
            )
            .await?;
        }
    }

    Ok(())
}

async fn print_tool_steps(stdout: &mut io::Stdout, steps: &[AgentStep]) -> io::Result<()> {
    if steps.is_empty() {
        return Ok(());
    }

    write_line(stdout, "\nLangkah tool:").await?;
    for (index, step) in steps.iter().enumerate() {
        let status = if step.success { "sukses" } else { "gagal" };
        write_line(
            stdout,
            &format!("  {}. {} [{}]", index + 1, step.tool, status),
        )
        .await?;
        if let Some(message) = &step.message {
            write_line(stdout, &format!("     catatan: {}", message)).await?;
        }

        if !step.input.is_null() {
            let input = to_string_pretty(&step.input).unwrap_or_else(|_| step.input.to_string());
            for line in input.lines() {
                write_line(stdout, &format!("     in : {}", line)).await?;
            }
        }

        if !step.output.is_null() {
            let output = to_string_pretty(&step.output).unwrap_or_else(|_| step.output.to_string());
            for line in output.lines() {
                write_line(stdout, &format!("     out: {}", line)).await?;
            }
        }
    }

    Ok(())
}

async fn print_logs(stdout: &mut io::Stdout, logs: &[String]) -> io::Result<()> {
    if logs.is_empty() {
        return Ok(());
    }

    write_line(stdout, "").await?;
    write_line(stdout, "Log:").await?;
    for log in logs {
        write_line(stdout, &format!("  - {}", log)).await?;
    }
    Ok(())
}

async fn print_banner(stdout: &mut io::Stdout) -> io::Result<()> {
    write_line(stdout, "Mode STDIO interaktif siap digunakan.").await?;
    write_line(
        stdout,
        "Mode agent aktif secara default untuk memastikan jawaban final.",
    )
    .await?;
    write_line(stdout, "Ketik pesan lalu tekan Enter untuk mengirim.").await?;
    write_line(stdout, "Gunakan /help untuk daftar perintah.").await?;
    Ok(())
}

async fn print_help(stdout: &mut io::Stdout) -> io::Result<()> {
    write_line(stdout, "\nPerintah yang tersedia:").await?;
    write_line(stdout, "  /help               Tampilkan bantuan ini").await?;
    write_line(stdout, "  /config             Lihat konfigurasi MCP aktif").await?;
    write_line(
        stdout,
        "  /config edit        Buka editor konfigurasi interaktif",
    )
    .await?;
    write_line(
        stdout,
        "  /log                Tampilkan log interaksi terakhir",
    )
    .await?;
    write_line(
        stdout,
        "  /steps              Tampilkan langkah tool terakhir",
    )
    .await?;
    write_line(
        stdout,
        "  /agent [on|off]     Aktifkan atau nonaktifkan mode agent",
    )
    .await?;
    write_line(
        stdout,
        "  /reset              Hapus session dan mulai percakapan baru",
    )
    .await?;
    write_line(
        stdout,
        "  /reload             Muat ulang konfigurasi dari file",
    )
    .await?;
    write_line(stdout, "  /exit               Keluar dari mode STDIO").await?;
    write_line(
        stdout,
        "Ketik pesan tanpa awalan / untuk mengirim ke model.",
    )
    .await?;
    Ok(())
}

async fn prompt(stdout: &mut io::Stdout, state: &SessionState) -> io::Result<()> {
    let label = if state.agent_mode {
        "agent> "
    } else {
        "chat> "
    };
    stdout.write_all(label.as_bytes()).await?;
    stdout.flush().await
}

async fn write_line(stdout: &mut io::Stdout, line: &str) -> io::Result<()> {
    stdout.write_all(line.as_bytes()).await?;
    stdout.write_all(b"\n").await?;
    Ok(())
}

fn looks_like_tool_call(content: &str) -> bool {
    fn parse_candidate(text: &str) -> Option<Value> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }
        if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
            return Some(value);
        }
        if trimmed.starts_with("```") {
            let stripped = trimmed
                .trim_start_matches("```json")
                .trim_start_matches("```JSON")
                .trim_start_matches("```");
            if let Some(end) = stripped.rfind("```") {
                let slice = &stripped[..end];
                if let Ok(value) = serde_json::from_str::<Value>(slice.trim()) {
                    return Some(value);
                }
            }
        }
        if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
            if start < end {
                let slice = &trimmed[start..=end];
                if let Ok(value) = serde_json::from_str::<Value>(slice) {
                    return Some(value);
                }
            }
        }
        None
    }

    fn matches_tool_signature(value: &Value) -> bool {
        match value {
            Value::Object(map) => {
                if let Some(action) = map.get("action").and_then(Value::as_str) {
                    if action.eq_ignore_ascii_case("call_tool") {
                        return true;
                    }
                }
                if map.contains_key("tool_code") {
                    return true;
                }
                if let Some(tool) = map.get("tool") {
                    if tool.is_string() && !map.contains_key("response") {
                        return true;
                    }
                }
                if let Some(tool_calls) = map.get("tool_calls") {
                    return matches_tool_signature(tool_calls);
                }
                false
            }
            Value::Array(items) => items.iter().any(matches_tool_signature),
            _ => false,
        }
    }

    parse_candidate(content)
        .map(|value| matches_tool_signature(&value))
        .unwrap_or(false)
}

fn preview(text: &str) -> String {
    const LIMIT: usize = 120;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return "(kosong)".to_string();
    }
    let mut result = String::new();
    for (idx, ch) in trimmed.chars().enumerate() {
        if idx >= LIMIT {
            result.push_str("...");
            break;
        }
        result.push(ch);
    }
    result
}
