    #[test]
    fn test_mask_short_value() {
        assert_eq!(mask_sensitive("abc"), "***");
        assert_eq!(mask_sensitive("12345678"), "********");
    }


    #[test]
    fn test_mask_long_value() {
        let result = mask_sensitive("Bearer token12345");
        assert!(result.starts_with("Bear"));
        assert!(result.ends_with("2345"));
        assert!(result.contains("..."));
    }

