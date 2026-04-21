use std::io::{self, Stdout};
use std::sync::Arc;
use std::time::Duration;

use antikythera_core::application::agent::{Agent, AgentOptions, AgentOutcome, AgentStep};
use antikythera_core::application::client::{
    ChatRequest, ChatResult, ClientConfigSnapshot, McpClient,
};
use antikythera_core::config::{AppConfig, ModelProviderConfig, loader as config_loader};
use antikythera_core::infrastructure::model::DynamicModelProvider;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};

use crate::CliResult;
use crate::runtime::{build_runtime_client, materialize_runtime_config};

const MAX_VISIBLE_MESSAGES: usize = 12;
const SLASH_COMMANDS: [(&str, &str); 10] = [
    ("help", "Tampilkan perintah yang tersedia"),
    ("providers", "Tampilkan provider dan model yang tersedia"),
    ("use", "Pilih provider aktif: /use <provider> [model]"),
    ("model", "Ganti model provider aktif: /model <nama-model>"),
    ("config", "Ringkasan provider, prompt, tools, dan server"),
    ("tools", "Daftar tools aktif pada sesi ini"),
    ("agent", "Toggle atau set mode agent: /agent on|off|toggle"),
    ("reset", "Mulai sesi baru dan hapus riwayat UI"),
    ("clear", "Alias untuk /reset"),
    ("exit", "Tutup TUI"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UiTone {
    User,
    Assistant,
    System,
    Error,
}

#[derive(Debug, Clone)]
struct UiMessage {
    title: String,
    body: String,
    tone: UiTone,
}

impl UiMessage {
    fn new(title: impl Into<String>, body: impl Into<String>, tone: UiTone) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            tone,
        }
    }
}

struct ChatApp {
    runtime_config: AppConfig,
    provider: String,
    model: String,
    session_id: Option<String>,
    input: String,
    agent_mode: bool,
    status: String,
    tools: usize,
    providers: Vec<ModelProviderConfig>,
    snapshot: ClientConfigSnapshot,
    messages: Vec<UiMessage>,
    loading: bool,
    should_quit: bool,
}

impl ChatApp {
    fn new(runtime_config: AppConfig, snapshot: ClientConfigSnapshot, tools: usize) -> Self {
        let mut app = Self {
            provider: runtime_config.default_provider.clone(),
            model: runtime_config.model.clone(),
            session_id: None,
            input: String::new(),
            agent_mode: true,
            status: "Siap. Ketik pesan atau gunakan /help.".to_string(),
            tools,
            providers: runtime_config.providers.clone(),
            runtime_config,
            snapshot,
            messages: Vec::new(),
            loading: false,
            should_quit: false,
        };
        app.messages.push(UiMessage::new(
            "Welcome",
            "Interactive mode siap. Gunakan /use <provider> [model] atau /model <nama-model> untuk mengganti backend langsung dari TUI.",
            UiTone::System,
        ));
        app
    }

    fn push_message(&mut self, message: UiMessage) {
        self.messages.push(message);
        if self.messages.len() > 64 {
            let excess = self.messages.len() - 64;
            self.messages.drain(0..excess);
        }
    }

    fn suggestions(&self) -> Vec<(&'static str, &'static str)> {
        slash_command_suggestions(&self.input)
    }

    fn reset_session(&mut self) {
        self.session_id = None;
        self.status =
            "Sesi direset. Riwayat host baru akan dimulai pada pesan berikutnya.".to_string();
        self.push_message(UiMessage::new(
            "Session Reset",
            "Riwayat sesi UI dibersihkan. Context baru akan dibuat saat Anda mengirim pesan berikutnya.",
            UiTone::System,
        ));
    }
}

pub async fn run_chat_app(config: AppConfig) -> CliResult<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let client = build_runtime_client(&config)?;
    let snapshot = client.config_snapshot();
    let tools = client.tools().len();
    let result = run_loop(&mut terminal, client, ChatApp::new(config, snapshot, tools)).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

async fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    mut client: Arc<McpClient<DynamicModelProvider>>,
    mut app: ChatApp,
) -> CliResult<()> {
    loop {
        terminal.draw(|frame| draw(frame, &app))?;

        if app.should_quit {
            break;
        }

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }

        if let Event::Key(key) = event::read()? {
            match handle_key_event(key, &mut app) {
                KeyAction::None => {}
                KeyAction::Submit => {
                    app.loading = true;
                    terminal.draw(|frame| draw(frame, &app))?;
                    submit_input(&mut client, &mut app).await;
                    app.loading = false;
                }
                KeyAction::Quit => app.should_quit = true,
            }
        }
    }

    Ok(())
}

enum KeyAction {
    None,
    Submit,
    Quit,
}

fn handle_key_event(key: KeyEvent, app: &mut ChatApp) -> KeyAction {
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
        return KeyAction::Quit;
    }

    match key.code {
        KeyCode::Esc => KeyAction::Quit,
        KeyCode::Enter => KeyAction::Submit,
        KeyCode::Backspace => {
            app.input.pop();
            KeyAction::None
        }
        KeyCode::Tab => {
            if let Some((command, _)) = app.suggestions().first() {
                app.input = format!("/{command}");
            }
            KeyAction::None
        }
        KeyCode::Char(character) => {
            if !key.modifiers.contains(KeyModifiers::CONTROL) {
                app.input.push(character);
            }
            KeyAction::None
        }
        _ => KeyAction::None,
    }
}

async fn submit_input(client: &mut Arc<McpClient<DynamicModelProvider>>, app: &mut ChatApp) {
    let input = app.input.trim().to_string();
    app.input.clear();

    if input.is_empty() {
        app.status = "Ketik pesan atau slash command untuk melanjutkan.".to_string();
        return;
    }

    if input.starts_with('/') {
        process_command(app, client, &input);
        return;
    }

    app.push_message(UiMessage::new("You", &input, UiTone::User));
    app.status = format!("Mengirim ke {}/{}...", app.provider, app.model);

    if app.agent_mode {
        let options = AgentOptions {
            session_id: app.session_id.clone(),
            ..AgentOptions::default()
        };
        let agent = Agent::new(client.clone());
        match agent.run(input, options).await {
            Ok(outcome) => apply_agent_outcome(app, outcome),
            Err(error) => {
                app.status = "Agent gagal menyelesaikan permintaan.".to_string();
                app.push_message(UiMessage::new(
                    "Agent Error",
                    error.user_message(),
                    UiTone::Error,
                ));
            }
        }
    } else {
        match client
            .chat(ChatRequest {
                prompt: input,
                attachments: Vec::new(),
                system_prompt: None,
                session_id: app.session_id.clone(),
                raw_mode: false,
                bypass_template: false,
                force_json: false,
            })
            .await
        {
            Ok(result) => apply_chat_result(app, result),
            Err(error) => {
                app.status = "Model gagal menjawab.".to_string();
                app.push_message(UiMessage::new(
                    "Model Error",
                    error.user_message(),
                    UiTone::Error,
                ));
            }
        }
    }
}

fn apply_chat_result(app: &mut ChatApp, result: ChatResult) {
    app.session_id = Some(result.session_id.clone());
    app.status = format!(
        "Respons diterima dari {}/{}.",
        result.provider, result.model
    );
    app.push_message(UiMessage::new(
        format!("Assistant [{}]", result.provider),
        result.content,
        UiTone::Assistant,
    ));
}

fn apply_agent_outcome(app: &mut ChatApp, outcome: AgentOutcome) {
    app.session_id = Some(outcome.session_id.clone());
    app.status = format!("Agent selesai dengan {} langkah tool.", outcome.steps.len());
    app.push_message(UiMessage::new(
        "Agent",
        format_agent_response(&outcome.response),
        UiTone::Assistant,
    ));

    if !outcome.steps.is_empty() {
        app.push_message(UiMessage::new(
            "Tool Trace",
            render_steps_summary(&outcome.steps),
            UiTone::System,
        ));
    }
}

fn process_command(
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

fn draw(frame: &mut ratatui::Frame<'_>, app: &ChatApp) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(16),
            Constraint::Length(5),
            Constraint::Length(2),
        ])
        .split(frame.area());

    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(68), Constraint::Percentage(32)])
        .split(layout[1]);

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " Antikythera CLI ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{} / {}", app.provider, app.model),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            if app.agent_mode {
                "Agent Loop"
            } else {
                "Direct Chat"
            },
            Style::default().fg(Color::Yellow),
        ),
    ]))
    .block(Block::default().borders(Borders::ALL).title("Session"));
    frame.render_widget(header, layout[0]);

    let messages = app
        .messages
        .iter()
        .rev()
        .take(MAX_VISIBLE_MESSAGES)
        .collect::<Vec<_>>();
    let conversation = Paragraph::new(render_messages(messages.into_iter().rev()))
        .block(Block::default().borders(Borders::ALL).title("Conversation"))
        .wrap(Wrap { trim: false });
    frame.render_widget(conversation, content[0]);

    let sidebar_items = build_sidebar_items(app)
        .into_iter()
        .map(ListItem::new)
        .collect::<Vec<_>>();
    let sidebar =
        List::new(sidebar_items).block(Block::default().borders(Borders::ALL).title("Context"));
    frame.render_widget(sidebar, content[1]);

    let input = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title("Prompt"))
        .style(if app.loading {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });
    frame.render_widget(input, layout[2]);

    if app.input.starts_with('/') {
        let suggestions = app
            .suggestions()
            .into_iter()
            .map(|(name, description)| ListItem::new(format!("/{name:<10} {description}")))
            .collect::<Vec<_>>();
        let area = centered_rect(72, 34, frame.area());
        frame.render_widget(Clear, area);
        frame.render_widget(
            List::new(suggestions).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Command Suggestions"),
            ),
            area,
        );
    }

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(
            "Tab",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" autocomplete  "),
        Span::styled(
            "Enter",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" submit  "),
        Span::styled(
            "Esc",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" quit  "),
        Span::styled(app.status.as_str(), Style::default().fg(Color::Gray)),
    ]))
    .block(Block::default().borders(Borders::ALL).title("Status"));
    frame.render_widget(footer, layout[3]);
}

fn render_messages<'a>(messages: impl Iterator<Item = &'a UiMessage>) -> Text<'static> {
    let mut lines = Vec::new();
    for message in messages {
        let tone_style = match message.tone {
            UiTone::User => Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD),
            UiTone::Assistant => Style::default()
                .fg(Color::Black)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            UiTone::System => Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            UiTone::Error => Style::default()
                .fg(Color::White)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD),
        };
        lines.push(Line::from(vec![
            Span::styled(format!(" {} ", message.title), tone_style),
            Span::raw(" "),
        ]));
        for body_line in message.body.lines() {
            lines.push(Line::from(Span::raw(body_line.to_string())));
        }
        lines.push(Line::default());
    }
    Text::from(lines)
}

fn build_sidebar_items(app: &ChatApp) -> Vec<String> {
    let session = app.session_id.as_deref().unwrap_or("belum ada");
    let provider_lines = app
        .providers
        .iter()
        .map(|provider| {
            let marker = if provider.id == app.provider {
                "*"
            } else {
                " "
            };
            let models = provider
                .models
                .iter()
                .map(|model| {
                    model
                        .display_name
                        .clone()
                        .unwrap_or_else(|| model.name.clone())
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "{marker} {} [{}]\n  {}",
                provider.id, provider.provider_type, models
            )
        })
        .collect::<Vec<_>>();

    vec![
        format!("Provider aktif : {}", app.provider),
        format!("Model aktif    : {}", app.model),
        format!(
            "Mode           : {}",
            if app.agent_mode { "agent" } else { "chat" }
        ),
        format!("Tools aktif    : {}", app.tools),
        format!("Session        : {}", session),
        String::new(),
        "Providers".to_string(),
        provider_lines.join("\n"),
    ]
}

fn centered_rect(
    horizontal_percent: u16,
    vertical_percent: u16,
    area: ratatui::layout::Rect,
) -> ratatui::layout::Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - vertical_percent) / 2),
            Constraint::Percentage(vertical_percent),
            Constraint::Percentage((100 - vertical_percent) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - horizontal_percent) / 2),
            Constraint::Percentage(horizontal_percent),
            Constraint::Percentage((100 - horizontal_percent) / 2),
        ])
        .split(vertical[1])[1]
}

fn format_agent_response(value: &serde_json::Value) -> String {
    value.as_str().map(ToOwned::to_owned).unwrap_or_else(|| {
        serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
    })
}

fn render_steps_summary(steps: &[AgentStep]) -> String {
    steps
        .iter()
        .enumerate()
        .map(|(index, step)| {
            format!(
                "{}. {} [{}]{}",
                index + 1,
                step.tool,
                if step.success { "ok" } else { "failed" },
                step.message
                    .as_deref()
                    .map(|message| format!(" - {}", message))
                    .unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_provider_catalog(
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

fn render_config_snapshot(snapshot: &ClientConfigSnapshot) -> String {
    [
        format!("default provider : {}", snapshot.default_provider),
        format!("default model    : {}", snapshot.model),
        format!(
            "system prompt    : {}",
            snapshot.system_prompt.as_deref().unwrap_or("<none>")
        ),
        format!("providers        : {}", snapshot.providers.len()),
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

fn apply_runtime_selection(
    app: &mut ChatApp,
    provider: String,
    model: String,
) -> Result<String, String> {
    let updated_config = materialize_runtime_config(
        &app.runtime_config,
        Some(&provider),
        Some(&model),
        None,
        None,
        app.runtime_config.system_prompt.as_deref(),
    )
    .map_err(|error| error.to_string())?;

    app.runtime_config = updated_config;
    app.provider = app.runtime_config.default_provider.clone();
    app.model = app.runtime_config.model.clone();
    app.providers = app.runtime_config.providers.clone();
    app.session_id = None;

    Ok(format!(
        "Provider/model aktif sekarang {}/{}. Sesi percakapan direset agar riwayat tidak tercampur antar backend.",
        app.provider, app.model
    ))
}

fn resolve_provider_selection(
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

fn reconfigure_runtime(
    app: &mut ChatApp,
    client: &mut Arc<McpClient<DynamicModelProvider>>,
) -> Result<(), String> {
    config_loader::save_config(&app.runtime_config, None).map_err(|error| error.to_string())?;

    let new_client =
        build_runtime_client(&app.runtime_config).map_err(|error| error.to_string())?;
    app.snapshot = new_client.config_snapshot();
    app.tools = new_client.tools().len();
    app.providers = app.runtime_config.providers.clone();
    *client = new_client;

    Ok(())
}

fn find_provider<'a>(
    providers: &'a [ModelProviderConfig],
    provider_input: &str,
) -> Option<&'a ModelProviderConfig> {
    let needle = provider_input.trim();
    providers
        .iter()
        .find(|provider| provider.id.eq_ignore_ascii_case(needle))
}

fn slash_command_suggestions(input: &str) -> Vec<(&'static str, &'static str)> {
    if !input.starts_with('/') {
        return Vec::new();
    }

    let needle = input.trim_start_matches('/').trim().to_ascii_lowercase();
    SLASH_COMMANDS
        .iter()
        .copied()
        .filter(|(command, _)| needle.is_empty() || command.starts_with(&needle))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use antikythera_core::config::ModelInfo;

    fn provider(id: &str, provider_type: &str, model: &str) -> ModelProviderConfig {
        ModelProviderConfig {
            id: id.to_string(),
            provider_type: provider_type.to_string(),
            endpoint: format!("https://{}.example.test", id),
            api_key: None,
            api_path: None,
            models: vec![ModelInfo {
                name: model.to_string(),
                display_name: Some(model.to_string()),
            }],
        }
    }

    #[test]
    fn slash_command_suggestions_match_prefix() {
        let suggestions = slash_command_suggestions("/pr");
        assert!(suggestions.iter().any(|(name, _)| *name == "providers"));
        assert!(suggestions.iter().all(|(name, _)| name.starts_with("pr")));
    }

    #[test]
    fn render_provider_catalog_marks_active_model() {
        let rendered = render_provider_catalog(
            &[provider("gemini", "gemini", "gemini-2.0-flash")],
            "gemini",
            "gemini-2.0-flash",
        );
        assert!(rendered.contains("(aktif)"));
        assert!(rendered.contains("gemini-2.0-flash"));
    }

    #[test]
    fn resolve_provider_selection_uses_provider_default_model_when_missing() {
        let selection = resolve_provider_selection(
            &[
                provider("ollama", "ollama", "llama3.2"),
                provider("openai", "openai", "gpt-4o-mini"),
            ],
            "ollama",
            "llama3.2",
            "openai",
            None,
        )
        .expect("selection should resolve");

        assert_eq!(selection.0, "openai");
        assert_eq!(selection.1, "gpt-4o-mini");
    }

    #[test]
    fn resolve_provider_selection_accepts_explicit_custom_model() {
        let selection = resolve_provider_selection(
            &[provider("gemini", "gemini", "gemini-2.0-flash")],
            "gemini",
            "gemini-2.0-flash",
            "gemini",
            Some("gemini-2.5-pro"),
        )
        .expect("selection should resolve");

        assert_eq!(selection.0, "gemini");
        assert_eq!(selection.1, "gemini-2.5-pro");
    }
}
