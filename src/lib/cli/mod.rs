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
    #[arg(long, default_value = "127.0.0.1:8080")]
    pub rest_addr: SocketAddr,
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
}
