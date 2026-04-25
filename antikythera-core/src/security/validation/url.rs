//! URL Validation Logic

use super::types::ValidationResult;
use regex::Regex;

pub struct URLValidator {
    pub allowed_patterns: Vec<Regex>,
    pub blocked_patterns: Vec<Regex>,
}

impl URLValidator {
    pub fn new(allowed: Vec<Regex>, blocked: Vec<Regex>) -> Self {
        Self {
            allowed_patterns: allowed,
            blocked_patterns: blocked,
        }
    }

    pub fn validate(&self, url: &str) -> ValidationResult {
        // Check blocked patterns first
        for regex in &self.blocked_patterns {
            if regex.is_match(url) {
                return ValidationResult::Invalid(format!("URL matches blocked pattern: {}", url));
            }
        }

        // Check allowed patterns
        let is_allowed = self
            .allowed_patterns
            .iter()
            .any(|regex| regex.is_match(url));

        if !is_allowed {
            return ValidationResult::Invalid(format!(
                "URL does not match any allowed pattern: {}",
                url
            ));
        }

        ValidationResult::Valid
    }
}
