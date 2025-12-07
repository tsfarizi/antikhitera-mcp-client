//! Interactive prompt functions (Single Responsibility)

use super::ui;
use std::error::Error;
use std::io::{self, Write};

/// Prompt for text input with optional default value
pub fn prompt_text(label: &str, default: Option<&str>) -> Result<String, Box<dyn Error>> {
    let prompt = match default {
        Some(d) if !d.is_empty() => format!("  {} [{}]: ", label, d),
        _ => format!("  {}: ", label),
    };

    print!("{}", prompt);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim();

    if trimmed.is_empty() {
        match default {
            Some(d) => Ok(d.to_string()),
            None => Ok(String::new()),
        }
    } else {
        Ok(trimmed.to_string())
    }
}

/// Prompt for password (hidden input on supported terminals)
pub fn prompt_password(label: &str) -> Result<String, Box<dyn Error>> {
    print!("  {}: ", label);
    io::stdout().flush()?;

    // Note: For true hidden input, would need rpassword crate
    // For now, just read normal input
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_string())
}

/// Prompt for comma-separated list
pub fn prompt_list(label: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let input = prompt_text(label, Some(""))?;

    if input.is_empty() {
        return Ok(vec![]);
    }

    let items: Vec<String> = input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Ok(items)
}

/// Prompt for yes/no confirmation
pub fn prompt_confirm(label: &str, default: bool) -> Result<bool, Box<dyn Error>> {
    let default_str = if default { "Y/n" } else { "y/N" };
    let input = prompt_text(&format!("{} [{}]", label, default_str), None)?;

    match input.to_lowercase().as_str() {
        "y" | "yes" => Ok(true),
        "n" | "no" => Ok(false),
        "" => Ok(default),
        _ => Ok(default),
    }
}

/// Prompt for multiple models with auto-generated display names
pub fn prompt_models() -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let mut models = Vec::new();

    loop {
        let model_name = prompt_text("Model name (empty to finish)", Some(""))?;

        if model_name.is_empty() {
            break;
        }

        let display_name = generate_display_name(&model_name);
        ui::print_hint(&format!("Display: {}", display_name));

        models.push((model_name, display_name));

        if !prompt_confirm("Add another model?", false)? {
            break;
        }
        println!();
    }

    Ok(models)
}

/// Generate display name from model name
/// e.g., "gemini-2.0-flash" -> "Gemini 2.0 Flash"
fn generate_display_name(name: &str) -> String {
    name.split('-')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}
