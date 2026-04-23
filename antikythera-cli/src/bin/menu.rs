//! Main CLI Binary Entry Point
//!
//! Thin wrapper over `antikythera_core`: parses CLI arguments, loads the
//! shared `app.pc` config, constructs an `McpClient`, then dispatches to one
//! of the supported run modes:
//!
//! | Mode | Description |
//! |:-----|:------------|
//! | `stdio` (default) | Interactive ratatui TUI chat session |
//! | `setup` | Configuration wizard for providers and servers |
//! | `multi-agent` | Multi-agent orchestrator harness |
//! | `wasm-harness` | Host-FFI WASM probe for runtime/session/tool validation |
//!
//! All provider resolution, session management, and protocol handling live in
//! `antikythera-core`; this binary only handles argument-to-run-mode wiring.

use std::path::Path;
use std::sync::Arc;

use antikythera_cli::domain::use_cases::{render_wasm_stream_report, run_wasm_stream_probe};
use antikythera_cli::infrastructure::llm::install_terminal_stream_sink;
use antikythera_cli::infrastructure::llm::providers_from_postcard;
use antikythera_cli::presentation::tui;
use antikythera_cli::presentation::tui_tracing::AntikytheraTuiLayer;
use antikythera_cli::runtime::{build_runtime_client, materialize_runtime_config};
use antikythera_cli::config::load_app_config;
use antikythera_core::application::agent::multi_agent::task::AgentTask;
use antikythera_cli::cli::{Cli, RunMode};
use antikythera_core::infrastructure::model::DynamicModelProvider;
use antikythera_core::{AppConfig, McpClient};
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(feature = "multi-agent")]
use antikythera_core::application::agent::multi_agent::{
    AgentProfile, DirectRouter, ExecutionMode, MultiAgentOrchestrator, RoundRobinRouter,
};

#[cfg(feature = "multi-agent")]
use antikythera_core::application::agent::multi_agent::budget::OrchestratorBudget;

#[cfg(feature = "multi-agent")]
use antikythera_core::application::agent::multi_agent::guardrails::{
    BudgetGuardrail, GuardrailChain, TimeoutGuardrail,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Install the TUI tracing bridge first — routes all tracing::debug!/info!/
    // warn!/error! events from antikythera-core into the LOGGERS system so they
    // appear in the WASM/FFI Logs panel. Must be installed before any crate code runs.
    tracing_subscriber::registry()
        .with(AntikytheraTuiLayer)
        .init();

    // Load .env at process startup so GEMINI_API_KEY / OPENAI_API_KEY are
    // available for provider auto-detection. Missing .env file is fine.
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();

    let config_path = cli.config.as_deref().map(Path::new);
    let config = AppConfig::load(config_path)?;
    // Load provider definitions and last-saved routing choices from app.pc.
    let pc_config = load_app_config(config_path).unwrap_or_default();
    let initial_providers = providers_from_postcard(&pc_config.providers);

    // Resolve provider/model: CLI flags > saved app.pc > TOML defaults.
    let provider_override = cli.provider.clone().or_else(|| {
        let p = pc_config.model.default_provider.trim().to_string();
        if p.is_empty() { None } else { Some(p) }
    });
    let model_override = cli.model.clone().or_else(|| {
        let m = pc_config.model.model.trim().to_string();
        if m.is_empty() { None } else { Some(m) }
    });
    // Resolve system prompt: CLI flag > saved custom["system_prompt"] > TOML default.
    let system_override = cli.system.clone()
        .or_else(|| pc_config.custom.get("system_prompt").cloned())
        .or_else(|| config.system_prompt.clone());

    let (runtime_config, providers) = materialize_runtime_config(
        &config,
        &initial_providers,
        provider_override.as_deref(),
        model_override.as_deref(),
        cli.provider_endpoint.as_deref(),
        Some(cli.ollama_url.as_str()),
        system_override.as_deref(),
    )?;

    if cli.stream {
        install_terminal_stream_sink();
    }

    let mode = cli.mode.unwrap_or(RunMode::Stdio);

    match mode {
        RunMode::Stdio => {
            tui::run_chat_app(runtime_config, providers).await?;
        }
        RunMode::Setup => {
            eprintln!(
                "Setup mode requires the wizard feature. \
                 Run `antikythera-config init` to create a default config."
            );
        }
        RunMode::MultiAgent => {
            let client = build_runtime_client(&runtime_config, &providers)?;
            run_multi_agent(cli, client).await?;
        }
        RunMode::WasmHarness => {
            run_wasm_harness(cli).await?;
        }
    }

    Ok(())
}

async fn run_wasm_harness(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let wasm_path = cli
        .wasm
        .unwrap_or_else(|| "target/wasm32-wasip1/release/antikythera_sdk.wasm".to_string());

    let task_input = if let Some(t) = cli.task.as_deref() {
        t.to_string()
    } else {
        "WASM harness smoke test".to_string()
    };

    let default_response = r#"{"content":"harness-ok","model":"wasm-harness"}"#.to_string();
    let llm_payload = cli.wasm_llm_response.unwrap_or(default_response);

    // In harness mode we force stream diagnostics on to expose all runtime phases.
    if !cli.stream {
        eprintln!("[wasm-harness] enabling stream diagnostics for dev tooling output");
    }
    let stream_report = run_wasm_stream_probe(&task_input, &llm_payload, true)?;

    println!("== WASM Host FFI Harness ==");
    println!("artifact: {}", wasm_path);
    println!("mode: ffi-host-probe");
    println!();
    println!("{}", render_wasm_stream_report(&stream_report)?);

    println!("\n== WASM Dev Summary JSON ==");
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "artifact": wasm_path,
            "ffi_stream_probe": stream_report,
        }))?
    );

    Ok(())
}

/// Run the multi-agent orchestrator test harness.
///
/// Reads agent profiles from `--agents <file>`, dispatches the task from
/// `--task <text>` (or stdin), and prints the result as JSON to stdout.
#[cfg(feature = "multi-agent")]
async fn run_multi_agent(
    cli: Cli,
    client: Arc<McpClient<DynamicModelProvider>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // ----------------------------------------------------------------
    // Parse execution mode
    // ----------------------------------------------------------------
    let exec_mode = ExecutionMode::from_spec(&cli.execution_mode).unwrap_or(ExecutionMode::Auto);

    // ----------------------------------------------------------------
    // Load agent profiles
    // ----------------------------------------------------------------
    let profiles: Vec<AgentProfile> = if let Some(agents_path) = cli.agents.as_deref() {
        let raw = std::fs::read_to_string(agents_path)
            .map_err(|e| format!("Failed to read agents file '{}': {e}", agents_path))?;
        serde_json::from_str(&raw).map_err(|e| format!("Failed to parse agents JSON: {e}"))?
    } else {
        // Default: one general-purpose agent
        vec![AgentProfile {
            id: "default".to_string(),
            name: "Default Agent".to_string(),
            role: "general".to_string(),
            system_prompt: None,
            max_steps: None,
        }]
    };

    // ----------------------------------------------------------------
    // Build orchestrator
    // ----------------------------------------------------------------
    let mut orch = MultiAgentOrchestrator::new(client, exec_mode);
    for profile in profiles {
        orch = orch.register_agent(profile);
    }

    // Apply router based on --target-agent flag
    if let Some(target) = cli.target_agent.as_deref() {
        let target = target.to_string();
        let router = Arc::new(DirectRouter);
        orch = orch.with_router(router);
        eprintln!("Routing all tasks to agent: {target}");
    } else if orch.agent_count() > 1 {
        orch = orch.with_router(Arc::new(RoundRobinRouter::new()));
    }

    // ----------------------------------------------------------------
    // Attach resource limits: an orchestrator-wide budget (max concurrent
    // tasks + total step ceiling) and a per-task guardrail chain (wall-clock
    // timeout + step budget cap). This prevents runaway agent loops.
    // ----------------------------------------------------------------
    // OrchestratorBudget tracks global concurrency and cumulative step usage
    // across all in-flight tasks for the lifetime of this orchestrator.
    let budget = OrchestratorBudget::new()
        .with_max_concurrent_tasks(8)
        .with_max_total_steps(1_000);

    // GuardrailChain adds per-task timeout enforcement and step-budget cap.
    let guardrails = GuardrailChain::new()
        // Enforce a hard 5-minute wall-clock execution limit on each dispatched task.
        .with_guardrail(Arc::new(TimeoutGuardrail::new(300_000)))
        // Allow at most 50 reasoning steps per task to prevent runaway agent loops.
        .with_guardrail(Arc::new(BudgetGuardrail::new().with_max_task_steps(50)));

    orch = orch.with_budget(budget).with_guardrails(guardrails);

    eprintln!(
        "Multi-agent orchestrator ready: {} agent(s), mode={}, guardrails={}",
        orch.agent_count(),
        exec_mode,
        orch.guardrail_count(),
    );

    // ----------------------------------------------------------------
    // Read task input
    // ----------------------------------------------------------------
    let task_input = if let Some(t) = cli.task.as_deref() {
        t.to_string()
    } else {
        eprintln!("Reading task from stdin (send EOF when done)...");
        let mut buf = String::new();
        {
            use std::io::Read;
            std::io::stdin().read_to_string(&mut buf)?;
        }
        buf.trim().to_string()
    };

    if task_input.is_empty() {
        return Err("No task input provided. Use --task <text> or pipe to stdin.".into());
    }

    // ----------------------------------------------------------------
    // Dispatch task
    // ----------------------------------------------------------------
    let task = AgentTask::new(task_input);
    let result = orch.dispatch(task).await;

    // ----------------------------------------------------------------
    // Output result as JSON
    // ----------------------------------------------------------------
    println!("{}", serde_json::to_string_pretty(&result)?);

    if !result.success {
        std::process::exit(1);
    }

    Ok(())
}

/// Stub for when the `multi-agent` feature is disabled.
#[cfg(not(feature = "multi-agent"))]
async fn run_multi_agent(
    _cli: Cli,
    _client: Arc<McpClient<impl antikythera_core::infrastructure::model::ModelProvider + 'static>>,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("multi-agent feature is not enabled in this build.\n\
         Rebuild with: cargo build --features multi-agent"
        .into())
}
