//! Main CLI Binary Entry Point
//!
//! Acts as the "host" for WASM - calls LLM APIs directly.
//! Only supports Gemini and Ollama providers.

use clap::Parser;

#[derive(Parser)]
#[command(name = "antikythera")]
#[command(about = "Antikythera MCP Client - Native Binary")]
pub struct Cli {
    /// Mode to run in
    #[arg(short, long, default_value = "tui")]
    pub mode: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.mode.as_str() {
        "tui" => run_tui().await,
        "rest" => run_rest().await,
        _ => {
            eprintln!("Unknown mode: {}. Use 'tui' or 'rest'.", cli.mode);
            std::process::exit(1);
        }
    }
}

async fn run_tui() -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Initialize TUI with clean architecture
    // 1. Load CLI config
    // 2. Create LLM provider (Gemini or Ollama)
    // 3. Create ChatUseCase
    // 4. Run TUI
    println!("TUI mode - coming soon");
    Ok(())
}

async fn run_rest() -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Initialize REST server with clean architecture
    // 1. Load CLI config
    // 2. Create LLM provider (Gemini or Ollama)
    // 3. Create ChatUseCase
    // 4. Start REST server
    println!("REST mode - coming soon");
    Ok(())
}
