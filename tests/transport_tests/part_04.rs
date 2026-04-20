    #[test]
    fn test_transport_type_equality() {
        assert_eq!(TransportType::Stdio, TransportType::Stdio);
        assert_eq!(TransportType::Http, TransportType::Http);
        assert_ne!(TransportType::Stdio, TransportType::Http);
    }

