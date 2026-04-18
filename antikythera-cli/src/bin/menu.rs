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

use antikythera_core::cli::{Cli, RunMode};
use antikythera_core::application::stdio;
use antikythera_core::{AppConfig, ClientConfig, McpClient};
use antikythera_cli::infrastructure::llm::build_provider_from_configs;
use clap::Parser;

#[cfg(feature = "multi-agent")]
use antikythera_core::application::agent::multi_agent::{
    AgentProfile, AgentTask, DirectRouter, ExecutionMode, MultiAgentOrchestrator,
    RoundRobinRouter,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let config_path = cli.config.as_deref().map(Path::new);
    let config = AppConfig::load(config_path)?;

    let mut providers = config.providers.clone();
    // Apply the --ollama-url CLI flag to all Ollama provider endpoints
    for p in providers.iter_mut() {
        if p.is_ollama() {
            p.endpoint = cli.ollama_url.clone();
        }
    }

    let provider = build_provider_from_configs(&providers)?;
    let mut client_cfg = ClientConfig::new(
        config.default_provider.clone(),
        config.model.clone(),
    )
    .with_tools(config.tools.clone())
    .with_servers(config.servers.clone())
    .with_prompts(config.prompts.clone())
    .with_providers(providers.clone());

    if let Some(system) = cli.system.clone().or(config.system_prompt.clone()) {
        client_cfg = client_cfg.with_system_prompt(system);
    }

    let client = Arc::new(McpClient::new(provider, client_cfg));

    let mode = cli.mode.unwrap_or(RunMode::Stdio);

    match mode {
        RunMode::Stdio => {
            stdio::run(client).await?;
        }
        RunMode::Setup => {
            eprintln!(
                "Setup mode requires the wizard feature. \
                 Run `antikythera-config init` to create a default config."
            );
        }
        RunMode::MultiAgent => {
            run_multi_agent(cli, client).await?;
        }
    }

    Ok(())
}

/// Run the multi-agent orchestrator test harness.
///
/// Reads agent profiles from `--agents <file>`, dispatches the task from
/// `--task <text>` (or stdin), and prints the result as JSON to stdout.
#[cfg(feature = "multi-agent")]
async fn run_multi_agent(
    cli: Cli,
    client: Arc<McpClient<impl antikythera_core::infrastructure::model::ModelProvider + 'static>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // ----------------------------------------------------------------
    // Parse execution mode
    // ----------------------------------------------------------------
    let exec_mode = ExecutionMode::from_spec(&cli.execution_mode)
        .unwrap_or(ExecutionMode::Auto);

    // ----------------------------------------------------------------
    // Load agent profiles
    // ----------------------------------------------------------------
    let profiles: Vec<AgentProfile> = if let Some(agents_path) = cli.agents.as_deref() {
        let raw = std::fs::read_to_string(agents_path)
            .map_err(|e| format!("Failed to read agents file '{}': {e}", agents_path))?;
        serde_json::from_str(&raw)
            .map_err(|e| format!("Failed to parse agents JSON: {e}"))?
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
