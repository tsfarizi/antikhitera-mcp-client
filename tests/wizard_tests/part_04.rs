    #[test]
    fn test_mask_empty_value() {
        assert_eq!(mask_sensitive(""), "");
    }


    #[test]
    fn test_mask_exact_boundary() {
        // 9 characters should trigger long format
        let result = mask_sensitive("123456789");
        assert_eq!(result, "1234...6789");
    }
}

mod transport_config_tests {
    use antikythera_core::config::{ServerConfig, TransportType};
    use std::collections::HashMap;
    use std::path::PathBuf;

