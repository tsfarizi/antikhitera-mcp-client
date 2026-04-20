#[test]
#[serial_test::serial]
fn wit_contract_signatures_match_golden() {
    let wit_file = repo_root().join("wit").join("antikythera.wit");
    let wit_content = fs::read_to_string(wit_file).expect("read WIT file");

    let actual = extract_wit_signatures(&wit_content);
    let expected_path = fixture_path("wit_signatures.golden.txt");
    let expected = fs::read_to_string(expected_path)
        .expect("read golden WIT signatures")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.to_string())
        .collect::<Vec<_>>();

    assert_eq!(
        actual, expected,
        "WIT contract changed; this is a breaking-contract detector failure"
    );
}

