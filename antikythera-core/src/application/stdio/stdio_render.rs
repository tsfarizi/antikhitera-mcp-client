use super::StdioError;
use super::{ACCENT, DIM, RESET, SUCCESS, SessionState, WARN};
use crate::application::agent::AgentStep;
use crate::application::client::McpClient;
use crate::application::model_provider::ModelProvider;
use crate::config::CONFIG_PATH;
use crate::logging::StdioLogger;
use serde_json::to_string_pretty;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use tokio::io::{self, AsyncWriteExt};

pub(super) async fn show_config<P: ModelProvider>(
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
            let cmd_display = server
                .command
                .as_ref()
                .map(|p| p.display().to_string())
                .or_else(|| server.url.clone())
                .unwrap_or_else(|| "(no command/url)".to_string());
            let mut line = format!("  - {} -> {}", server.name, cmd_display);
            if !server.args.is_empty() {
                line.push_str(&format!(" {}", server.args.join(" ")));
            }
            write_line(stdout, &line).await?;
        }
    }

    write_line(stdout, "- Providers        : (dikelola oleh CLI)").await?;

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
            StdioLogger::new("stdio").warn(format!(
                "Gagal membaca berkas konfigurasi | error={}",
                error
            ));
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

pub(super) async fn print_tool_steps(
    stdout: &mut io::Stdout,
    steps: &[AgentStep],
) -> io::Result<()> {
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

pub(super) async fn print_logs(stdout: &mut io::Stdout, logs: &[String]) -> io::Result<()> {
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

pub(super) async fn print_banner(stdout: &mut io::Stdout) -> io::Result<()> {
    write_line(
        stdout,
        &format!("{ACCENT}============================================================{RESET}"),
    )
    .await?;
    write_line(
        stdout,
        &format!("{ACCENT}Antikythera Interactive CLI (STDIO / TUI-Style){RESET}"),
    )
    .await?;
    write_line(
        stdout,
        &format!("{ACCENT}============================================================{RESET}"),
    )
    .await?;
    write_line(stdout, "Mode STDIO interaktif siap digunakan.").await?;
    write_line(
        stdout,
        "Mode agent aktif secara default untuk memastikan jawaban final.",
    )
    .await?;
    write_line(stdout, "Ketik pesan lalu tekan Enter untuk mengirim.").await?;
    write_line(stdout, "Gunakan /help untuk daftar perintah.").await?;
    write_line(
        stdout,
        &format!(
            "{DIM}Tip: ketik '/' lalu nama perintah sebagian (contoh: /co) untuk rekomendasi.{RESET}"
        ),
    )
    .await?;
    Ok(())
}

pub(super) async fn print_help(stdout: &mut io::Stdout) -> io::Result<()> {
    write_line(stdout, &format!("\n{ACCENT}Perintah yang tersedia:{RESET}")).await?;
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
    write_line(
        stdout,
        &format!("{DIM}Rekomendasi cepat: '/', '/he', '/co', '/ag'.{RESET}"),
    )
    .await?;
    Ok(())
}

pub(super) async fn prompt(stdout: &mut io::Stdout, state: &SessionState) -> io::Result<()> {
    let label = if state.agent_mode { "agent" } else { "chat" };
    let session_chip = state
        .session_id
        .as_ref()
        .map(|id| {
            let short = id.chars().take(10).collect::<String>();
            format!(" {DIM}[session:{short}]{RESET}")
        })
        .unwrap_or_default();
    let rendered = format!("{SUCCESS}{label}{RESET}>{session_chip} ");
    stdout.write_all(rendered.as_bytes()).await?;
    stdout.flush().await
}

pub(super) async fn write_line(stdout: &mut io::Stdout, line: &str) -> io::Result<()> {
    stdout.write_all(line.as_bytes()).await?;
    stdout.write_all(b"\n").await?;
    Ok(())
}

pub(super) fn preview(text: &str) -> String {
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

pub(super) async fn print_command_recommendations(
    stdout: &mut io::Stdout,
    prefix: &str,
) -> Result<(), StdioError> {
    let suggestions = super::suggest_commands(prefix);
    if suggestions.is_empty() {
        write_line(
            stdout,
            &format!("{WARN}Tidak ada rekomendasi perintah untuk '/{prefix}'.{RESET}"),
        )
        .await?;
        return Ok(());
    }

    write_line(
        stdout,
        &format!(
            "{ACCENT}Rekomendasi perintah:{RESET} {}",
            suggestions.join(", ")
        ),
    )
    .await?;
    Ok(())
}
