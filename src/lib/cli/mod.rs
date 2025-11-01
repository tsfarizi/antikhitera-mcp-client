use std::net::SocketAddr;

use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "antikhitera-mcp-client",
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
    #[arg(long)]
    pub session: Option<String>,
    #[arg(long)]
    pub prompt_file: Option<String>,
    #[arg(long, value_enum, default_value_t = RunMode::Cli)]
    pub mode: RunMode,
    #[arg(long, default_value = "127.0.0.1:8080")]
    pub rest_addr: SocketAddr,
    #[arg()]
    pub prompt: Vec<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum RunMode {
    Cli,
    Stdio,
    Rest,
    Agent,
}
