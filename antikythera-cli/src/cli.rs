use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "antikythera",
    version,
    about = "Antikythera MCP client dengan penyedia model yang dapat dikonfigurasi"
)]
pub struct Cli {
    #[arg(long, default_value = "http://127.0.0.1:11434")]
    pub ollama_url: String,
    /// Override the active provider ID without editing app.pc.
    #[arg(long)]
    pub provider: Option<String>,
    /// Override the active model name without editing app.pc.
    #[arg(long)]
    pub model: Option<String>,
    /// Override the endpoint for the selected provider.
    #[arg(long)]
    pub provider_endpoint: Option<String>,
    #[arg(long)]
    pub config: Option<String>,
    #[arg(long)]
    pub system: Option<String>,
    #[arg(long, short, value_enum)]
    pub mode: Option<RunMode>,

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

    /// Enable token streaming output in CLI mode.
    #[arg(long)]
    pub stream: bool,

    /// Path to WASM module used by `--mode wasm-harness`.
    #[arg(long)]
    pub wasm: Option<String>,

    /// Stub LLM response returned by host callback in `--mode wasm-harness`.
    ///
    /// If omitted, a deterministic JSON stub response is used.
    #[arg(long)]
    pub wasm_llm_response: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum RunMode {
    /// Interactive TUI mode
    Stdio,
    /// Configuration wizard/setup menu
    Setup,
    /// Multi-agent orchestrator test harness
    #[value(name = "multi-agent")]
    MultiAgent,
    /// Execute a local WASM artifact through the host runtime bridge
    #[value(name = "wasm-harness")]
    WasmHarness,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    // ── RunMode slice ─────────────────────────────────────────────────────────

    #[test]
    fn run_mode_all_variants_are_distinct() {
        let modes = [
            RunMode::Stdio,
            RunMode::Setup,
            RunMode::MultiAgent,
            RunMode::WasmHarness,
        ];
        // Each variant must be unique (PartialEq derived).
        for (i, a) in modes.iter().enumerate() {
            for (j, b) in modes.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn cli_default_ollama_url_points_to_localhost() {
        let cli = Cli::parse_from(["antikythera"]);
        assert_eq!(cli.ollama_url, "http://127.0.0.1:11434");
    }

    #[test]
    fn cli_mode_defaults_to_none_which_resolves_to_stdio() {
        let cli = Cli::parse_from(["antikythera"]);
        assert!(cli.mode.is_none());
        // Callers use unwrap_or(RunMode::Stdio) — verify the sentinel value.
        assert_eq!(cli.mode.unwrap_or(RunMode::Stdio), RunMode::Stdio);
    }

    #[test]
    fn cli_stream_flag_is_false_by_default() {
        let cli = Cli::parse_from(["antikythera"]);
        assert!(!cli.stream);
    }

    #[test]
    fn cli_stream_flag_enabled_by_long_flag() {
        let cli = Cli::parse_from(["antikythera", "--stream"]);
        assert!(cli.stream);
    }

    #[test]
    fn cli_mode_wasm_harness_parsed_from_value_name() {
        let cli = Cli::parse_from(["antikythera", "--mode", "wasm-harness"]);
        assert_eq!(cli.mode, Some(RunMode::WasmHarness));
    }

    #[test]
    fn cli_mode_multi_agent_parsed_from_value_name() {
        let cli = Cli::parse_from(["antikythera", "--mode", "multi-agent"]);
        assert_eq!(cli.mode, Some(RunMode::MultiAgent));
    }

    #[test]
    fn cli_provider_and_model_overrides_are_optional() {
        let cli = Cli::parse_from(["antikythera"]);
        assert!(cli.provider.is_none());
        assert!(cli.model.is_none());
        assert!(cli.provider_endpoint.is_none());
    }

    #[test]
    fn cli_provider_override_accepted() {
        let cli = Cli::parse_from(["antikythera", "--provider", "gemini"]);
        assert_eq!(cli.provider.as_deref(), Some("gemini"));
    }

    #[test]
    fn cli_execution_mode_defaults_to_auto() {
        let cli = Cli::parse_from(["antikythera"]);
        assert_eq!(cli.execution_mode, "auto");
    }
}
