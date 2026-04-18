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
        RunMode::Rest => {
            let addr = cli.rest_addr.unwrap_or_else(|| {
                config
                    .rest_server
                    .bind
                    .parse()
                    .expect("Invalid bind address in config")
            });
            antikythera_core::infrastructure::server::serve(
                client,
                addr,
                &config.rest_server.cors_origins,
                &config.rest_server.docs,
            )
            .await?;
        }
        RunMode::All => {
            let addr = cli.rest_addr.unwrap_or_else(|| {
                config
                    .rest_server
                    .bind
                    .parse()
                    .expect("Invalid bind address in config")
            });
            let rest_client = client.clone();
            let cors = config.rest_server.cors_origins.clone();
            let docs = config.rest_server.docs.clone();
            let rest_handle = tokio::spawn(async move {
                if let Err(e) = antikythera_core::infrastructure::server::serve(
                    rest_client,
                    addr,
                    &cors,
                    &docs,
                )
                .await
                {
                    eprintln!("REST server error: {}", e);
                }
            });
            stdio::run(client).await?;
            rest_handle.abort();
        }
        RunMode::Setup => {
            eprintln!(
                "Setup mode requires the wizard feature. \
                 Run `antikythera-config init` to create a default config."
            );
        }
    }

    Ok(())
}


