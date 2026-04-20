#[test]
fn cli_stream_flag_defaults_to_false() {
    let cli = Cli::try_parse_from(["mcp"]).expect("cli parse should succeed");
    assert!(!cli.stream);
}

