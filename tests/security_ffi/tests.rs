//! Security FFI tests

use antikythera_sdk::security_ffi::*;
use std::ffi::CString;
use std::sync::Mutex;

static TEST_MUTEX: Mutex<()> = Mutex::new(());

#[cfg(test)]
pub mod validation_ffi_tests {
    use super::*;

    #[test]
    fn test_init_validator() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let result = unsafe {
            let ptr = mcp_security_init_validator();
            let json = CString::from_raw(ptr);
            json.into_string().unwrap()
        };

        assert!(result.contains("\"success\":true"));
    }

    #[test]
    fn test_validate_input_valid() {
        let _guard = TEST_MUTEX.lock().unwrap();
        unsafe {
            mcp_security_init_validator();

            let input = CString::new("normal input").unwrap();
            let result_ptr = mcp_security_validate_input(input.as_ptr());
            let result = CString::from_raw(result_ptr).into_string().unwrap();

            assert!(result.contains("\"valid\":true"));
        }
    }

    #[test]
    fn test_validate_input_invalid() {
        let _guard = TEST_MUTEX.lock().unwrap();
        unsafe {
            mcp_security_init_validator();

            let input = CString::new("<script>alert('xss')</script>").unwrap();
            let result_ptr = mcp_security_validate_input(input.as_ptr());
            let result = CString::from_raw(result_ptr).into_string().unwrap();

            assert!(result.contains("\"valid\":false"));
        }
    }

    #[test]
    fn test_validate_url_valid() {
        let _guard = TEST_MUTEX.lock().unwrap();
        unsafe {
            mcp_security_init_validator();

            let url = CString::new("https://example.com").unwrap();
            let result_ptr = mcp_security_validate_url(url.as_ptr());
            let result = CString::from_raw(result_ptr).into_string().unwrap();

            assert!(result.contains("\"valid\":true"));
        }
    }

    #[test]
    fn test_validate_url_blocked() {
        let _guard = TEST_MUTEX.lock().unwrap();
        unsafe {
            mcp_security_init_validator();

            let url = CString::new("file:///etc/passwd").unwrap();
            let result_ptr = mcp_security_validate_url(url.as_ptr());
            let result = CString::from_raw(result_ptr).into_string().unwrap();

            assert!(result.contains("\"valid\":false"));
        }
    }

    #[test]
    fn test_validate_json_valid() {
        let _guard = TEST_MUTEX.lock().unwrap();
        unsafe {
            mcp_security_init_validator();

            let json = CString::new(r#"{"key": "value"}"#).unwrap();
            let result_ptr = mcp_security_validate_json(json.as_ptr());
            let result = CString::from_raw(result_ptr).into_string().unwrap();

            assert!(result.contains("\"valid\":true"));
        }
    }

    #[test]
    fn test_validate_json_invalid() {
        let _guard = TEST_MUTEX.lock().unwrap();
        unsafe {
            mcp_security_init_validator();

            let json = CString::new(r#"{"key": "value""#).unwrap();
            let result_ptr = mcp_security_validate_json(json.as_ptr());
            let result = CString::from_raw(result_ptr).into_string().unwrap();

            assert!(result.contains("\"valid\":false"));
        }
    }

    #[test]
    fn test_sanitize_html() {
        let _guard = TEST_MUTEX.lock().unwrap();
        unsafe {
            mcp_security_init_validator();

            let html = CString::new("<script>alert('xss')</script>").unwrap();
            let result_ptr = mcp_security_sanitize_html(html.as_ptr());
            let result = CString::from_raw(result_ptr).into_string().unwrap();

            assert!(!result.contains("<script>"));
        }
    }

    #[test]
    fn test_get_validation_config() {
        let _guard = TEST_MUTEX.lock().unwrap();
        unsafe {
            mcp_security_init_validator();

            let result_ptr = mcp_security_get_validation_config();
            let result = CString::from_raw(result_ptr).into_string().unwrap();

            assert!(result.contains("max_input_size_bytes"));
            assert!(result.contains("max_message_length"));
        }
    }

    #[test]
    fn test_set_validation_config() {
        let _guard = TEST_MUTEX.lock().unwrap();
        unsafe {
            mcp_security_init_validator();

            let config = r#"{
                "max_input_size_bytes": 5242880,
                "max_message_length": 50000,
                "max_concurrent_tool_calls": 5,
                "allowed_url_patterns": ["^https?://.*$"],
                "blocked_url_patterns": ["^file://.*$"],
                "sanitize_html": true,
                "validate_json_schema": true,
                "max_json_nesting_depth": 10,
                "max_json_array_length": 1000,
                "blocked_keywords": ["<script", "javascript:"]
            }"#;

            let config_cstr = CString::new(config).unwrap();
            let result_ptr = mcp_security_set_validation_config(config_cstr.as_ptr());
            let result = CString::from_raw(result_ptr).into_string().unwrap();

            assert!(result.contains("\"success\":true"));
        }
    }
}

#[cfg(test)]
pub mod rate_limit_ffi_tests {
    use super::*;

    #[test]
    fn test_init_rate_limiter() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let result = unsafe {
            let ptr = mcp_security_init_rate_limiter();
            let json = CString::from_raw(ptr);
            json.into_string().unwrap()
        };

        assert!(result.contains("\"success\":true"));
    }

    #[test]
    fn test_check_rate_limit_allowed() {
        let _guard = TEST_MUTEX.lock().unwrap();
        unsafe {
            mcp_security_init_rate_limiter();

            let session_id = CString::new("test-session-1").unwrap();
            let result_ptr = mcp_security_check_rate_limit(session_id.as_ptr());
            let result = CString::from_raw(result_ptr).into_string().unwrap();

            assert!(result.contains("\"allowed\":true"));
        }
    }

    #[test]
    fn test_check_rate_limit_multiple_requests() {
        let _guard = TEST_MUTEX.lock().unwrap();
        unsafe {
            mcp_security_init_rate_limiter();

            let session_id = CString::new("test-session-2").unwrap();

            // Multiple requests within limit
            for _ in 0..10 {
                let result_ptr = mcp_security_check_rate_limit(session_id.as_ptr());
                let result = CString::from_raw(result_ptr).into_string().unwrap();
                assert!(result.contains("\"allowed\":true"));
            }
        }
    }

    #[test]
    fn test_get_usage() {
        let _guard = TEST_MUTEX.lock().unwrap();
        unsafe {
            mcp_security_init_rate_limiter();

            let session_id = CString::new("test-session-3").unwrap();
            mcp_security_check_rate_limit(session_id.as_ptr());
            mcp_security_check_rate_limit(session_id.as_ptr());

            let result_ptr = mcp_security_get_usage(session_id.as_ptr());
            let result = CString::from_raw(result_ptr).into_string().unwrap();

            assert!(
                result.contains("\"requests_per_minute\":") || result.contains("\"success\":true"),
                "Result should contain usage statistics or success flag: {}",
                result
            );
        }
    }

    #[test]
    fn test_reset_session() {
        let _guard = TEST_MUTEX.lock().unwrap();
        unsafe {
            mcp_security_init_rate_limiter();

            let session_id = CString::new("test-session-4").unwrap();
            mcp_security_check_rate_limit(session_id.as_ptr());
            mcp_security_check_rate_limit(session_id.as_ptr());

            let result_ptr = mcp_security_reset_session(session_id.as_ptr());
            let result = CString::from_raw(result_ptr).into_string().unwrap();

            assert!(result.contains("\"success\":true"));

            // Check usage after reset
            let usage_ptr = mcp_security_get_usage(session_id.as_ptr());
            let usage = CString::from_raw(usage_ptr).into_string().unwrap();

            // Allow for varying whitespace in JSON output
            let is_reset = usage.contains("\"requests_per_minute\":0")
                || usage.contains("\"requests_per_minute\": 0");
            assert!(
                is_reset,
                "Session was not reset correctly. Usage: {}",
                usage
            );
        }
    }

    #[test]
    fn test_remove_session() {
        let _guard = TEST_MUTEX.lock().unwrap();
        unsafe {
            mcp_security_init_rate_limiter();

            let session_id = CString::new("test-session-5").unwrap();
            mcp_security_check_rate_limit(session_id.as_ptr());

            let result_ptr = mcp_security_remove_session(session_id.as_ptr());
            let result = CString::from_raw(result_ptr).into_string().unwrap();

            assert!(result.contains("\"success\":true"));

            // Session should not exist anymore
            let usage_ptr = mcp_security_get_usage(session_id.as_ptr());
            let usage = CString::from_raw(usage_ptr).into_string().unwrap();
            assert!(usage.contains("not found") || usage.contains("error"));
        }
    }

    #[test]
    fn test_get_rate_limit_config() {
        let _guard = TEST_MUTEX.lock().unwrap();
        unsafe {
            mcp_security_init_rate_limiter();

            let result_ptr = mcp_security_get_rate_limit_config();
            let result = CString::from_raw(result_ptr).into_string().unwrap();

            assert!(result.contains("requests_per_minute"));
            assert!(result.contains("requests_per_hour"));
            assert!(result.contains("requests_per_day"));
        }
    }

    #[test]
    fn test_set_rate_limit_config() {
        let _guard = TEST_MUTEX.lock().unwrap();
        unsafe {
            mcp_security_init_rate_limiter();

            let config = r#"{
                "enabled": true,
                "requests_per_minute": 100,
                "requests_per_hour": 1000,
                "requests_per_day": 10000,
                "max_concurrent_sessions": 10,
                "window_size_secs": 60,
                "burst_allowance": 10,
                "cleanup_interval_secs": 300
            }"#;

            let config_cstr = CString::new(config).unwrap();
            let result_ptr = mcp_security_set_rate_limit_config(config_cstr.as_ptr());
            let result = CString::from_raw(result_ptr).into_string().unwrap();

            assert!(result.contains("\"success\":true"));
        }
    }
}

#[cfg(test)]
pub mod secrets_ffi_tests {
    use super::*;

    #[test]
    fn test_init_secret_manager() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let result = unsafe {
            let ptr = mcp_security_init_secret_manager();
            let json = CString::from_raw(ptr);
            json.into_string().unwrap()
        };

        assert!(result.contains("\"success\":true"));
    }

    #[test]
    fn test_store_and_get_secret() {
        let _guard = TEST_MUTEX.lock().unwrap();
        unsafe {
            mcp_security_init_secret_manager();

            let id = CString::new("test-secret-store-get").unwrap();
            let value = CString::new("my-secret-value").unwrap();

            let store_ptr = mcp_security_store_secret(id.as_ptr(), value.as_ptr());
            let store_result = CString::from_raw(store_ptr).into_string().unwrap();
            assert!(
                store_result.contains("\"success\":true"),
                "Store failed: {}",
                store_result
            );

            let get_ptr = mcp_security_get_secret(id.as_ptr());
            let get_result = CString::from_raw(get_ptr).into_string().unwrap();
            assert!(
                get_result.contains("\"value\":\"my-secret-value\"")
                    || get_result.contains("\"value\": \"my-secret-value\""),
                "Get failed: {}",
                get_result
            );
        }
    }

    #[test]
    fn test_rotate_secret() {
        let _guard = TEST_MUTEX.lock().unwrap();
        unsafe {
            mcp_security_init_secret_manager();

            let id = CString::new("test-secret-rotate").unwrap();
            let old_value = CString::new("old-value").unwrap();
            let new_value = CString::new("new-value").unwrap();

            let store_ptr = mcp_security_store_secret(id.as_ptr(), old_value.as_ptr());
            let store_res = CString::from_raw(store_ptr).into_string().unwrap();
            assert!(
                store_res.contains("\"success\":true"),
                "Initial store failed: {}",
                store_res
            );

            let rotate_ptr = mcp_security_rotate_secret(id.as_ptr(), new_value.as_ptr());
            let rotate_result = CString::from_raw(rotate_ptr).into_string().unwrap();
            assert!(
                rotate_result.contains("\"success\":true")
                    || rotate_result.contains("\"success\": true"),
                "Rotate operation failed or returned unexpected JSON: {}",
                rotate_result
            );

            let get_ptr = mcp_security_get_secret(id.as_ptr());
            let get_result = CString::from_raw(get_ptr).into_string().unwrap();
            assert!(
                get_result.contains("\"value\":\"new-value\"")
                    || get_result.contains("\"value\": \"new-value\""),
                "Get after rotate failed: {}",
                get_result
            );
        }
    }

    #[test]
    fn test_delete_secret() {
        let _guard = TEST_MUTEX.lock().unwrap();
        unsafe {
            mcp_security_init_secret_manager();

            let id = CString::new("test-secret-delete").unwrap();
            let value = CString::new("value-to-delete").unwrap();

            let store_ptr = mcp_security_store_secret(id.as_ptr(), value.as_ptr());
            let store_res = CString::from_raw(store_ptr).into_string().unwrap();
            assert!(
                store_res.contains("\"success\":true"),
                "Store for delete failed: {}",
                store_res
            );

            let delete_ptr = mcp_security_delete_secret(id.as_ptr());
            let delete_result = CString::from_raw(delete_ptr).into_string().unwrap();
            assert!(
                delete_result.contains("\"success\":true")
                    || delete_result.contains("\"success\": true"),
                "Delete operation failed or returned unexpected JSON: {}",
                delete_result
            );

            let get_ptr = mcp_security_get_secret(id.as_ptr());
            let get_result = CString::from_raw(get_ptr).into_string().unwrap();
            assert!(
                get_result.contains("error") || get_result.contains("not found"),
                "Secret still exists after delete: {}",
                get_result
            );
        }
    }

    #[test]
    fn test_list_secrets() {
        let _guard = TEST_MUTEX.lock().unwrap();
        unsafe {
            mcp_security_init_secret_manager();

            let s1 = CString::new("list-secret1").unwrap();
            let s2 = CString::new("list-secret2").unwrap();
            let v1 = CString::new("value1").unwrap();
            let v2 = CString::new("value2").unwrap();

            let res1_ptr = mcp_security_store_secret(s1.as_ptr(), v1.as_ptr());
            let res1 = CString::from_raw(res1_ptr).into_string().unwrap();
            assert!(
                res1.contains("\"success\":true"),
                "Store secret1 failed: {}",
                res1
            );

            let res2_ptr = mcp_security_store_secret(s2.as_ptr(), v2.as_ptr());
            let res2 = CString::from_raw(res2_ptr).into_string().unwrap();
            assert!(
                res2.contains("\"success\":true"),
                "Store secret2 failed: {}",
                res2
            );

            let result_ptr = mcp_security_list_secrets();
            let result = CString::from_raw(result_ptr).into_string().unwrap();

            assert!(
                result.contains("\"secrets\""),
                "Result should contain secrets list: {}",
                result
            );
            assert!(
                result.contains("list-secret1"),
                "Result should contain list-secret1: {}",
                result
            );
            assert!(
                result.contains("list-secret2"),
                "Result should contain list-secret2: {}",
                result
            );
        }
    }

    #[test]
    fn test_get_secret_metadata() {
        let _guard = TEST_MUTEX.lock().unwrap();
        unsafe {
            mcp_security_init_secret_manager();

            let id = CString::new("test-secret-metadata").unwrap();
            let value = CString::new("value").unwrap();

            let store_ptr = mcp_security_store_secret(id.as_ptr(), value.as_ptr());
            let store_res = CString::from_raw(store_ptr).into_string().unwrap();
            assert!(
                store_res.contains("\"success\":true"),
                "Store for metadata failed: {}",
                store_res
            );

            let result_ptr = mcp_security_get_secret_metadata(id.as_ptr());
            let result = CString::from_raw(result_ptr).into_string().unwrap();

            assert!(
                result.contains("\"id\":\"test-secret-metadata\"")
                    || result.contains("\"id\": \"test-secret-metadata\""),
                "Result should contain correct ID: {}",
                result
            );
            assert!(
                result.contains("\"version\":1") || result.contains("\"version\": 1"),
                "Result should contain correct version: {}",
                result
            );
            assert!(
                result.contains("\"active\":true") || result.contains("\"active\": true"),
                "Result should contain correct active status: {}",
                result
            );
        }
    }

    #[test]
    fn test_get_secrets_config() {
        let _guard = TEST_MUTEX.lock().unwrap();
        unsafe {
            mcp_security_init_secret_manager();

            let result_ptr = mcp_security_get_secrets_config();
            let result = CString::from_raw(result_ptr).into_string().unwrap();

            assert!(result.contains("enabled"));
            assert!(result.contains("encrypt_at_rest"));
            assert!(result.contains("encryption_algorithm"));
        }
    }

    #[test]
    fn test_set_secrets_config() {
        let _guard = TEST_MUTEX.lock().unwrap();
        unsafe {
            mcp_security_init_secret_manager();

            let config = r#"{
                "enabled": true,
                "encrypt_at_rest": true,
                "encryption_algorithm": "AES256-GCM",
                "key_derivation_function": "Argon2",
                "key_derivation_iterations": 100000,
                "auto_rotate": false,
                "rotation_interval_hours": 720,
                "max_secret_age_hours": 2160,
                "storage_backend": "memory",
                "storage_path": null,
                "enable_versioning": true,
                "max_versions": 5
            }"#;

            let config_cstr = CString::new(config).unwrap();
            let result_ptr = mcp_security_set_secrets_config(config_cstr.as_ptr());
            let result = CString::from_raw(result_ptr).into_string().unwrap();

            assert!(result.contains("\"success\":true"));
        }
    }
}
