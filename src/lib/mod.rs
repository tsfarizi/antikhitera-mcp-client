pub mod application;
pub mod cli;
pub mod config;
pub mod domain;
pub mod infrastructure;

pub use application::{agent, client, stdio, tooling};
pub use cli::{Cli, RunMode};
pub use config::{AppConfig, ModelProviderConfig, ProviderKind};
pub use domain::types;
pub use infrastructure::{model, rpc, server};

use application::agent::{Agent, AgentOptions};
use application::client::{ChatRequest, ClientConfig, McpClient};
use infrastructure::model::DynamicModelProvider;
use serde_json::json;
use std::error::Error;
use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, warn};
use tracing_subscriber::{EnvFilter, fmt};

pub async fn run(cli: Cli) -> Result<(), Box<dyn Error>> {
    init_tracing();
    info!("Starting antikhitera-mcp-client");
    debug!(
        mode = ?cli.mode,
        config = ?cli.config,
        system = ?cli.system,
        session = ?cli.session,
        "CLI arguments parsed"
    );

    let config_path = cli.config.as_deref().map(Path::new);
    let file_config = AppConfig::load(config_path)?;
    if let Some(path) = config_path {
        info!(path = %path.display(), "Loaded configuration from file");
    } else {
        info!("Loaded configuration using default path or defaults");
    }

    let mut providers = file_config.providers.clone();
    apply_cli_overrides(&cli, &mut providers);
    debug!(
        provider_count = providers.len(),
        "Initializing dynamic model providers"
    );
    let provider = DynamicModelProvider::from_configs(&providers);
    let mut client_config = ClientConfig::new(
        file_config.default_provider.clone(),
        file_config.model.clone(),
    )
    .with_tools(file_config.tools.clone())
    .with_servers(file_config.servers.clone())
    .with_prompt_template(file_config.prompt_template.clone())
    .with_providers(providers.clone());
    if let Some(system_prompt) = cli.system.clone().or(file_config.system_prompt.clone()) {
        client_config = client_config.with_system_prompt(system_prompt);
    }
    let client = Arc::new(McpClient::new(provider, client_config));

    info!(mode = ?cli.mode, "Running client in selected mode");
    match cli.mode {
        RunMode::Cli => {
            let prompt = load_prompt(&cli)?;
            info!("Dispatching single prompt via CLI mode");
            let result = client
                .chat(ChatRequest {
                    prompt,
                    provider: None,
                    model: None,
                    system_prompt: None,
                    session_id: cli.session.clone(),
                })
                .await?;

            let output = json!({
                "session_id": result.session_id,
                "content": result.content,
                "provider": result.provider,
                "model": result.model,
                "logs": result.logs,
            });

            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        RunMode::Stdio => {
            info!("Launching STDIO interactive chat interface");
            stdio::run(client.clone()).await?;
        }
        RunMode::Rest => {
            info!(addr = %cli.rest_addr, "Starting REST server");
            server::serve(client.clone(), cli.rest_addr).await?;
        }
        RunMode::Agent => {
            let prompt = load_prompt(&cli)?;
            let mut options = AgentOptions::default();
            options.session_id = cli.session.clone();
            options.system_prompt = cli.system.clone().or(file_config.system_prompt.clone());
            info!("Executing agent workflow from CLI mode");
            let agent = Agent::new(client.clone());
            let outcome = agent.run(prompt, options).await?;
            let output = json!({
                "session_id": outcome.session_id,
                "content": outcome.response,
                "tool_steps": outcome.steps,
                "logs": outcome.logs,
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
    }
    info!("Client execution finished");
    Ok(())
}

fn init_tracing() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
        fmt()
            .with_env_filter(filter)
            .with_target(false)
            .with_level(true)
            .init();
    });
}

fn load_prompt(cli: &Cli) -> Result<String, Box<dyn Error>> {
    if let Some(path) = &cli.prompt_file {
        info!(path = %path, "Loading prompt from file");
        let content = fs::read_to_string(path)?;
        return Ok(normalize_prompt(content));
    }

    if !cli.prompt.is_empty() {
        info!("Using prompt provided through CLI arguments");
        let joined = cli.prompt.join(" ");
        return Ok(normalize_prompt(joined));
    }

    if atty::isnt(atty::Stream::Stdin) {
        info!("Reading prompt from standard input");
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        return Ok(normalize_prompt(buffer));
    }

    warn!("Prompt not provided via arguments, file, or stdin");
    Err("prompt required via arguments, file, or stdin".into())
}

fn apply_cli_overrides(cli: &Cli, providers: &mut [ModelProviderConfig]) {
    for provider in providers.iter_mut() {
        if provider.kind == ProviderKind::Ollama {
            if provider.endpoint != cli.ollama_url {
                info!(
                    provider = provider.id.as_str(),
                    url = %cli.ollama_url,
                    "Overriding provider endpoint based on CLI flag"
                );
            }
            provider.endpoint = cli.ollama_url.clone();
        }
    }
}

fn normalize_prompt(prompt: String) -> String {
    prompt.trim().to_string()
}
