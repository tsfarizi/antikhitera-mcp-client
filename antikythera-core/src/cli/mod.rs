use std::net::SocketAddr;

use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "mcp",
    version,
    about = "MCP client dengan penyedia model yang dapat dikonfigurasi"
)]
pub struct Cli {
    #[arg(long, default_value = "http://127.0.0.1:11434")]
    pub ollama_url: String,
    #[arg(long)]
    pub config: Option<String>,
    #[arg(long)]
    pub system: Option<String>,
    #[arg(long, short, value_enum)]
    pub mode: Option<RunMode>,
    /// REST API bind address (overrides config if specified)
    #[arg(long)]
    pub rest_addr: Option<SocketAddr>,

    // ------------------------------------------------------------------
    // Multi-agent flags (used when --mode multi-agent)
    // ------------------------------------------------------------------
    /// Path to a JSON file containing agent profile definitions.
    ///
    /// The file must be a JSON array of objects matching:
    /// `[{ "id": "...", "name": "...", "role": "...", "system_prompt": "...", "max_steps": 8 }]`
    #[arg(long)]
    pub agents: Option<String>,

    /// Task prompt to dispatch in multi-agent mode.
    ///
    /// When omitted the prompt is read from stdin.
    #[arg(long)]
    pub task: Option<String>,

    /// Target a specific agent by ID (uses DirectRouter).
    ///
    /// When omitted the orchestrator uses the default FirstAvailableRouter.
    #[arg(long)]
    pub target_agent: Option<String>,

    /// Execution mode for the multi-agent orchestrator.
    ///
    /// Accepted values: `auto` (default), `sequential`, `concurrent`, `parallel:N`.
    #[arg(long, default_value = "auto")]
    pub execution_mode: String,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum RunMode {
    /// Interactive STDIO mode
    Stdio,
    /// REST API server
    Rest,
    /// Run both STDIO and REST simultaneously
    All,
    /// Configuration wizard/setup menu
    Setup,
    /// Multi-agent orchestrator test harness
    #[value(name = "multi-agent")]
    MultiAgent,
}
