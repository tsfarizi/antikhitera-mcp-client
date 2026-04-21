//! Main CLI Binary Entry Point
//!
//! Thin wrapper over `antikythera_core`: parses CLI arguments, loads the
//! shared `app.pc` config, constructs an `McpClient`, then dispatches to the
//! core's STDIO loop (`tui` mode) or REST server (`rest` mode).
//!
//! All provider resolution, session management, and protocol handling live in
//! `antikythera-core`; this binary only handles argument-to-run-mode wiring.

use std::path::Path;
use std::sync::Arc;

use antikythera_cli::domain::use_cases::{render_wasm_stream_report, run_wasm_stream_probe};
use antikythera_cli::infrastructure::llm::install_terminal_stream_sink;
use antikythera_cli::presentation::tui;
use antikythera_cli::runtime::{build_runtime_client, materialize_runtime_config};
use antikythera_core::application::agent::multi_agent::task::AgentTask;
use antikythera_cli::cli::{Cli, RunMode};
use antikythera_core::infrastructure::model::DynamicModelProvider;
use antikythera_core::{AppConfig, McpClient};
use clap::Parser;

#[cfg(feature = "multi-agent")]
use antikythera_core::application::agent::multi_agent::{
    AgentProfile, DirectRouter, ExecutionMode, MultiAgentOrchestrator, RoundRobinRouter,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env at process startup so GEMINI_API_KEY / OPENAI_API_KEY are
    // available for provider auto-detection. Missing .env file is fine.
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();

    let config_path = cli.config.as_deref().map(Path::new);
    let config = AppConfig::load(config_path)?;
    let runtime_config = materialize_runtime_config(
        &config,
        cli.provider.as_deref(),
        cli.model.as_deref(),
        cli.provider_endpoint.as_deref(),
        Some(cli.ollama_url.as_str()),
        cli.system.as_deref().or(config.system_prompt.as_deref()),
    )?;

    if cli.stream {
        install_terminal_stream_sink();
    }

    let mode = cli.mode.unwrap_or(RunMode::Stdio);

    match mode {
        RunMode::Stdio => {
            tui::run_chat_app(runtime_config).await?;
        }
        RunMode::Setup => {
            eprintln!(
                "Setup mode requires the wizard feature. \
                 Run `antikythera-config init` to create a default config."
            );
        }
        RunMode::MultiAgent => {
            let client = build_runtime_client(&runtime_config)?;
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

    eprintln!(
        "Multi-agent orchestrator ready: {} agent(s), mode={}",
        orch.agent_count(),
        exec_mode
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
