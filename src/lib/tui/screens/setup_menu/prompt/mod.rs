//! Prompts configuration editing TUI
//!
//! Provides UI for editing all fields in the `[prompts]` section:
//! - template - System prompt template
//! - tool_guidance - Instructions for tool usage
//! - fallback_guidance - Instructions when request is outside tool scope
//! - json_retry_message - Message sent on JSON parse failure
//! - tool_result_instruction - Instructions for tool result formatting

mod dialogs;
mod editors;
mod viewers;

use super::load_config;
use crate::config::PromptsConfig;
use crate::config::wizard::generators::model as generator;
use crate::tui::terminal::{NavAction, Tui, read_key};
use crate::tui::widgets::{Menu, MenuItem};
use ratatui::style::Color;
use std::error::Error;

pub use dialogs::centered_rect;
use dialogs::{run_message_tui, run_reset_all_confirmation, run_reset_field_confirmation};
use editors::{run_edit_multiline, run_edit_singleline};
use viewers::{run_view_all_prompts, run_view_content};

/// Prompts field information for menu display
pub(crate) struct PromptField {
    pub name: &'static str,
    pub icon: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub is_multiline: bool,
}

pub(crate) const PROMPT_FIELDS: &[PromptField] = &[
    PromptField {
        name: "template",
        icon: "📄",
        label: "System Template",
        description: "Main system prompt template with placeholders",
        is_multiline: true,
    },
    PromptField {
        name: "tool_guidance",
        icon: "🔧",
        label: "Tool Guidance",
        description: "Instructions when tools are available",
        is_multiline: false,
    },
    PromptField {
        name: "fallback_guidance",
        icon: "⚠️",
        label: "Fallback Guidance",
        description: "Response when request is outside tool scope",
        is_multiline: false,
    },
    PromptField {
        name: "json_retry_message",
        icon: "🔄",
        label: "JSON Retry Message",
        description: "Message sent to LLM on JSON parse failure",
        is_multiline: false,
    },
    PromptField {
        name: "tool_result_instruction",
        icon: "📋",
        label: "Tool Result Instruction",
        description: "Instructions for tool result formatting",
        is_multiline: false,
    },
];

/// Get field value from PromptsConfig
pub(crate) fn get_field_value(prompts: &PromptsConfig, field_name: &str) -> String {
    match field_name {
        "template" => prompts.template().to_string(),
        "tool_guidance" => prompts.tool_guidance().to_string(),
        "fallback_guidance" => prompts.fallback_guidance().to_string(),
        "json_retry_message" => prompts.json_retry_message().to_string(),
        "tool_result_instruction" => prompts.tool_result_instruction().to_string(),
        _ => String::new(),
    }
}

/// Get default value for a field
fn get_default_value(field_name: &str) -> &'static str {
    match field_name {
        "template" => PromptsConfig::default_template(),
        "tool_guidance" => PromptsConfig::default_tool_guidance(),
        "fallback_guidance" => PromptsConfig::default_fallback_guidance(),
        "json_retry_message" => PromptsConfig::default_json_retry_message(),
        "tool_result_instruction" => PromptsConfig::default_tool_result_instruction(),
        _ => "",
    }
}

/// Update a field in config
pub(crate) fn update_field(field_name: &str, value: &str) -> Result<(), Box<dyn Error>> {
    match field_name {
        "template" => generator::update_prompt_template(value),
        "tool_guidance" => generator::update_tool_guidance(value),
        "fallback_guidance" => generator::update_fallback_guidance(value),
        "json_retry_message" => generator::update_json_retry_message(value),
        "tool_result_instruction" => generator::update_tool_result_instruction(value),
        _ => Ok(()),
    }
}

/// Main prompts management screen
pub fn run_edit_prompt_with_terminal(terminal: &mut Tui) -> Result<(), Box<dyn Error>> {
    let mut selected_idx: usize = 0;

    loop {
        let config = load_config()?;
        let prompts = &config.prompts;

        // Build menu items
        let mut items: Vec<MenuItem> = PROMPT_FIELDS
            .iter()
            .map(|f| {
                let value = get_field_value(prompts, f.name);
                let preview = if value.len() > 30 {
                    format!("{}...", value.chars().take(30).collect::<String>())
                } else {
                    value.lines().next().unwrap_or("").to_string()
                };
                MenuItem::new(format!("{} {} - {}", f.icon, f.label, preview))
            })
            .collect();

        items.push(MenuItem::new("👁️  View All Prompts"));
        items.push(MenuItem::new("🔄 Reset All to Defaults"));
        items.push(MenuItem::new("← Back"));

        let mut menu = Menu::new("📝 Manage Prompts Configuration", items)
            .with_subtitle("Select a prompt field to edit");
        menu.select(selected_idx);

        terminal.draw(|frame| {
            menu.render(frame, frame.area());
        })?;

        let action = NavAction::from(read_key()?);
        match action {
            NavAction::Up => {
                menu.previous();
                selected_idx = menu.selected_index().unwrap_or(0);
            }
            NavAction::Down => {
                menu.next();
                selected_idx = menu.selected_index().unwrap_or(0);
            }
            NavAction::Select => {
                let field_count = PROMPT_FIELDS.len();
                match menu.selected_index() {
                    Some(idx) if idx < field_count => {
                        // Edit specific field
                        let field = &PROMPT_FIELDS[idx];
                        run_field_submenu(terminal, field)?;
                    }
                    Some(idx) if idx == field_count => {
                        // View All Prompts
                        run_view_all_prompts(terminal, prompts)?;
                    }
                    Some(idx) if idx == field_count + 1 => {
                        // Reset All
                        if run_reset_all_confirmation(terminal)? {
                            for field in PROMPT_FIELDS {
                                update_field(field.name, get_default_value(field.name))?;
                            }
                            run_message_tui(
                                terminal,
                                "✓ All prompts reset to defaults!",
                                Color::Green,
                            )?;
                        }
                    }
                    Some(idx) if idx == field_count + 2 => break, // Back
                    _ => {}
                }
            }
            NavAction::ForceQuit | NavAction::Back => break,
            NavAction::None => {}
        }
    }

    Ok(())
}

/// Submenu for a specific field
fn run_field_submenu(terminal: &mut Tui, field: &PromptField) -> Result<(), Box<dyn Error>> {
    let mut selected_idx: usize = 0;

    loop {
        let config = load_config()?;
        let current_value = get_field_value(&config.prompts, field.name);
        let preview = if current_value.len() > 50 {
            format!("{}...", current_value.chars().take(50).collect::<String>())
        } else {
            current_value.lines().next().unwrap_or("").to_string()
        };

        let items = vec![
            MenuItem::new("✏️  Edit"),
            MenuItem::new("👁️  View Full Content"),
            MenuItem::new("🔄 Reset to Default"),
            MenuItem::new("← Back"),
        ];

        let title = format!("{} {}", field.icon, field.label);
        let mut menu = Menu::new(&title, items)
            .with_subtitle(format!("{}\nCurrent: {}", field.description, preview));
        menu.select(selected_idx);

        terminal.draw(|frame| {
            menu.render(frame, frame.area());
        })?;

        let action = NavAction::from(read_key()?);
        match action {
            NavAction::Up => {
                menu.previous();
                selected_idx = menu.selected_index().unwrap_or(0);
            }
            NavAction::Down => {
                menu.next();
                selected_idx = menu.selected_index().unwrap_or(0);
            }
            NavAction::Select => match menu.selected_index() {
                Some(0) => {
                    // Edit
                    if field.is_multiline {
                        run_edit_multiline(terminal, field.name, &current_value)?;
                    } else {
                        run_edit_singleline(terminal, field.name, field.label, &current_value)?;
                    }
                }
                Some(1) => {
                    // View
                    run_view_content(terminal, field.label, &current_value)?;
                }
                Some(2) => {
                    // Reset
                    if run_reset_field_confirmation(terminal, field.label)? {
                        update_field(field.name, get_default_value(field.name))?;
                        run_message_tui(
                            terminal,
                            &format!("✓ {} reset to default!", field.label),
                            Color::Green,
                        )?;
                    }
                }
                Some(3) => break,
                _ => {}
            },
            NavAction::ForceQuit | NavAction::Back => break,
            NavAction::None => {}
        }
    }

    Ok(())
}
