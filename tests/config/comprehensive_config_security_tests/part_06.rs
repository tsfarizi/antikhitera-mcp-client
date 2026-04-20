// ============================================================================
// TRANSPORT TYPE TESTS
// ============================================================================

#[test]
fn test_transport_type_stdio() {
    let transport = TransportType::Stdio;
    assert_eq!(transport, TransportType::Stdio);
}

#[test]
fn test_transport_type_http() {
    let transport = TransportType::Http;
    assert_eq!(transport, TransportType::Http);
}

#[test]
fn test_transport_type_clone() {
    let original = TransportType::Http;
    let cloned = original.clone();
    assert_eq!(original, cloned);
}

