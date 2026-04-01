use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "mcp",
    version,
    about = "MCP client dengan penyedia model yang dapat dikonfigurasi"
)]
pub struct Cli {
    #[arg(long)]
    pub config: Option<String>,
    #[arg(long)]
    pub system: Option<String>,
    #[arg(long, short, value_enum)]
    pub mode: Option<RunMode>,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum RunMode {
    /// CLI mode for debugging and native builds
    Cli,
    /// WASM build target
    Wasm,
}
