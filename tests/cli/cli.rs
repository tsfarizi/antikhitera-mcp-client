use antikythera_cli::cli::{Cli, RunMode};
use clap::Parser;

#[test]
fn run_mode_all_variants_are_distinct() {
    let modes = [RunMode::Stdio, RunMode::MultiAgent, RunMode::WasmHarness];
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
