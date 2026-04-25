//! JSON structure validation

use serde_json::Value;

pub struct JSONValidator {
    pub max_nesting_depth: u32,
    pub max_array_length: u32,
}

impl JSONValidator {
    pub fn new(max_depth: u32, max_array_len: u32) -> Self {
        Self {
            max_nesting_depth: max_depth,
            max_array_length: max_array_len,
        }
    }

    pub fn validate_structure(&self, value: &Value, depth: u32) -> Result<(), String> {
        if depth > self.max_nesting_depth {
            return Err(format!(
                "JSON nesting depth {} exceeds maximum {}",
                depth, self.max_nesting_depth
            ));
        }

        match value {
            Value::Array(arr) => {
                if arr.len() as u32 > self.max_array_length {
                    return Err(format!(
                        "JSON array length {} exceeds maximum {}",
                        arr.len(),
                        self.max_array_length
                    ));
                }
                for item in arr {
                    self.validate_structure(item, depth + 1)?;
                }
            }
            Value::Object(obj) => {
                for (_, v) in obj {
                    self.validate_structure(v, depth + 1)?;
                }
            }
            _ => {}
        }

        Ok(())
    }
}
