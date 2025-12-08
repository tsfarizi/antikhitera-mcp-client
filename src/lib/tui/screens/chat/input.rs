//! Chat input handling

use super::state::ChatState;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

/// Input action result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputAction {
    /// No action needed
    None,
    /// Submit the current input
    Submit,
    /// Exit the chat
    Exit,
    /// Execute a command
    Command(String),
    /// Scroll up
    ScrollUp,
    /// Scroll down
    ScrollDown,
    /// Scroll to top
    ScrollTop,
    /// Scroll to bottom
    ScrollBottom,
}

/// Handle keyboard input and update state
pub fn handle_input(state: &mut ChatState, event: Event) -> InputAction {
    if state.loading {
        if let Event::Key(key) = event {
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                return InputAction::None;
            }
            if key.code == KeyCode::Char('q')
                || (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('q'))
            {
                return InputAction::Exit;
            }
        }
        return InputAction::None;
    }

    match event {
        Event::Key(key) => handle_key(state, key),
        Event::Resize(_, _) => InputAction::None,
        _ => InputAction::None,
    }
}

fn handle_key(state: &mut ChatState, key: KeyEvent) -> InputAction {
    use crossterm::event::KeyEventKind;
    if key.kind != KeyEventKind::Press {
        return InputAction::None;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('q') {
        return InputAction::Exit;
    }
    if key.code == KeyCode::Char('q') && state.input.is_empty() {
        return InputAction::Exit;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        state.input.clear();
        state.cursor_pos = 0;
        return InputAction::None;
    }

    match key.code {
        KeyCode::Enter => {
            if state.input.is_empty() {
                return InputAction::None;
            }

            if state.is_command() {
                let cmd = state.take_input();
                return InputAction::Command(cmd);
            }

            InputAction::Submit
        }
        KeyCode::Esc => {
            if !state.input.is_empty() {
                state.input.clear();
                state.cursor_pos = 0;
            }
            InputAction::None
        }
        KeyCode::Backspace => {
            state.delete_char();
            InputAction::None
        }
        KeyCode::Delete => {
            state.delete_char_forward();
            InputAction::None
        }
        KeyCode::Left => {
            state.move_cursor_left();
            InputAction::None
        }
        KeyCode::Right => {
            state.move_cursor_right();
            InputAction::None
        }
        KeyCode::Home => {
            state.move_cursor_home();
            InputAction::None
        }
        KeyCode::End => {
            state.move_cursor_end();
            InputAction::None
        }
        KeyCode::Up | KeyCode::PageUp => InputAction::ScrollUp,
        KeyCode::Down | KeyCode::PageDown => InputAction::ScrollDown,
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            InputAction::ScrollTop
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            InputAction::ScrollBottom
        }
        KeyCode::Char(c) => {
            state.insert_char(c);
            InputAction::None
        }
        KeyCode::Tab => {
            state.insert_char(' ');
            state.insert_char(' ');
            InputAction::None
        }

        _ => InputAction::None,
    }
}

/// Parse and execute a command, return response message
pub fn parse_command(input: &str) -> CommandResult {
    let cmd = input.trim_start_matches(|c| c == '/' || c == ':');
    let mut parts = cmd.split_whitespace();
    let name = parts.next().unwrap_or("").to_ascii_lowercase();
    let args: Vec<&str> = parts.collect();

    match name.as_str() {
        "" => CommandResult::None,

        "help" | "?" => CommandResult::ShowHelp,

        "agent" => {
            if args.is_empty() {
                CommandResult::ToggleAgent
            } else {
                match args[0].to_lowercase().as_str() {
                    "on" | "true" | "1" => CommandResult::SetAgent(true),
                    "off" | "false" | "0" => CommandResult::SetAgent(false),
                    _ => CommandResult::ToggleAgent,
                }
            }
        }

        "reset" | "clear" | "new" => CommandResult::Reset,

        "log" | "logs" => CommandResult::ShowLogs,

        "steps" | "tools" | "toolsteps" => CommandResult::ShowSteps,

        "exit" | "quit" | "bye" => CommandResult::Exit,

        _ => CommandResult::Unknown(name),
    }
}

#[derive(Debug, Clone)]
pub enum CommandResult {
    None,
    ShowHelp,
    ToggleAgent,
    SetAgent(bool),
    Reset,
    ShowLogs,
    ShowSteps,
    Exit,
    Unknown(String),
}
