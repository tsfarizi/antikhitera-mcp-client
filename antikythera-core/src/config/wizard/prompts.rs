//! Interactive prompt functions (Single Responsibility)

use super::ui;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{disable_raw_mode, enable_raw_mode},
};
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
pub fn prompt_password(label: &str, default: Option<&str>) -> Result<String, Box<dyn Error>> {
    if let Some(d) = default {
        print!("  {} [{}]: ", label, d);
    } else {
        print!("  {}: ", label);
    }
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim();

    if trimmed.is_empty() {
        if let Some(d) = default {
            return Ok(d.to_string());
        }
    }

    Ok(trimmed.to_string())
}

/// Prompt for selection using arrow keys
pub fn prompt_select(label: &str, options: &[&str]) -> Result<String, Box<dyn Error>> {
    println!("  {}:", label);
    let mut selected = 0;

    enable_raw_mode()?;
    execute!(io::stdout(), cursor::Hide)?;

    loop {
        for (i, option) in options.iter().enumerate() {
            if i == selected {
                execute!(
                    io::stdout(),
                    SetForegroundColor(Color::Cyan),
                    Print(format!("    > {}\r\n", option)),
                    ResetColor
                )?;
            } else {
                execute!(io::stdout(), Print(format!("      {}\r\n", option)))?;
            }
        }

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Up => {
                        if selected > 0 {
                            selected -= 1;
                        } else {
                            selected = options.len() - 1;
                        }
                    }
                    KeyCode::Down => {
                        if selected < options.len() - 1 {
                            selected += 1;
                        } else {
                            selected = 0;
                        }
                    }
                    KeyCode::Enter => {
                        break;
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        disable_raw_mode()?;
                        execute!(io::stdout(), cursor::Show)?;
                        std::process::exit(0);
                    }
                    _ => {}
                }
            }
        }

        execute!(io::stdout(), cursor::MoveUp(options.len() as u16))?;
    }

    disable_raw_mode()?;
    execute!(io::stdout(), cursor::Show)?;

    Ok(options[selected].to_string())
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
