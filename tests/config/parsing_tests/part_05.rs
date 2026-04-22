#[test]
fn parses_prompt_template_from_postcard() {
    let dir = tempdir().expect("tempdir");
    let mut pc = minimal_postcard_config();
    pc.prompts.template = "Be helpful.".to_string();
    let path = write_postcard_config(dir.path(), &pc);

    let config = AppConfig::load(Some(&path)).expect("load config");

    assert_eq!(config.prompt_template(), "Be helpful.");
}
