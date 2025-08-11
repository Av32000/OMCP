use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{self, Clear, ClearType, disable_raw_mode, enable_raw_mode},
};
use std::io::{self, Write};
use tokio::io::{AsyncWriteExt, stdout};

use crate::ui::utils::{AnsiColor, colorize_text};

pub fn text_input(
    prompt: &str,
    current_input: Option<String>,
    history: &mut Vec<String>,
) -> String {
    const HISTORY_LIMIT: usize = 100;

    let mut stdout = io::stdout();
    let mut input = current_input.unwrap_or_default();
    let mut cursor_pos = input.len();
    let mut history_index: Option<usize> = None;
    let mut history_snapshot = String::new();

    print!("{}{}", prompt, input);
    stdout.flush().unwrap();
    enable_raw_mode().unwrap();

    let mut redraw = false;
    loop {
        if redraw {
            execute!(
                stdout,
                Clear(ClearType::CurrentLine),
                cursor::MoveToColumn(0)
            )
            .unwrap();
            print!("{}{}", prompt, input);
            let prompt_len = prompt.chars().count();
            let cursor_col = prompt_len + input[..cursor_pos].chars().count();
            execute!(stdout, cursor::MoveToColumn(cursor_col as u16)).unwrap();
            stdout.flush().unwrap();
            redraw = false;
        }
        if let Event::Key(key_event) = event::read().unwrap() {
            match key_event.code {
                KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                    disable_raw_mode().unwrap();
                    println!();
                    return String::new();
                }
                KeyCode::Char('u') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                    input.clear();
                    cursor_pos = 0;
                    redraw = true;
                }
                KeyCode::Char('h') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                    if cursor_pos > 0 {
                        let char_len = input[..cursor_pos]
                            .chars()
                            .last()
                            .map(|c| c.len_utf8())
                            .unwrap_or(1);
                        input.replace_range(cursor_pos - char_len..cursor_pos, "");
                        cursor_pos -= char_len;
                        redraw = true;
                    }
                }
                KeyCode::Backspace => {
                    if cursor_pos > 0 {
                        let char_len = input[..cursor_pos]
                            .chars()
                            .last()
                            .map(|c| c.len_utf8())
                            .unwrap_or(1);
                        input.replace_range(cursor_pos - char_len..cursor_pos, "");
                        cursor_pos -= char_len;
                        redraw = true;
                    }
                }
                KeyCode::Delete => {
                    if cursor_pos < input.len() {
                        let char_len = input[cursor_pos..]
                            .chars()
                            .next()
                            .map(|c| c.len_utf8())
                            .unwrap_or(1);
                        input.replace_range(cursor_pos..cursor_pos + char_len, "");
                        redraw = true;
                    }
                }
                KeyCode::Left => {
                    if cursor_pos > 0 {
                        let char_len = input[..cursor_pos]
                            .chars()
                            .last()
                            .map(|c| c.len_utf8())
                            .unwrap_or(1);
                        cursor_pos -= char_len;
                        redraw = true;
                    }
                }
                KeyCode::Right => {
                    if cursor_pos < input.len() {
                        let char_len = input[cursor_pos..]
                            .chars()
                            .next()
                            .map(|c| c.len_utf8())
                            .unwrap_or(1);
                        cursor_pos += char_len;
                        redraw = true;
                    }
                }
                KeyCode::Home => {
                    cursor_pos = 0;
                    redraw = true;
                }
                KeyCode::End => {
                    cursor_pos = input.len();
                    redraw = true;
                }
                KeyCode::Up => {
                    if history.len() > 0 {
                        if let Some(idx) = history_index {
                            if idx > 0 {
                                history_index = Some(idx - 1);
                            }
                        } else {
                            history_snapshot = input.clone();
                            history_index = Some(history.len() - 1);
                        }
                        if let Some(idx) = history_index {
                            if let Some(val) = history.get(idx) {
                                input = val.clone();
                                cursor_pos = input.len();
                                redraw = true;
                            }
                        }
                    }
                }
                KeyCode::Down => {
                    if let Some(idx) = history_index {
                        if idx + 1 < history.len() {
                            history_index = Some(idx + 1);
                            if let Some(val) = history.get(idx + 1) {
                                input = val.clone();
                                cursor_pos = input.len();
                                redraw = true;
                            }
                        } else {
                            input = history_snapshot.clone();
                            cursor_pos = input.len();
                            history_index = None;
                            redraw = true;
                        }
                    }
                }
                KeyCode::Char(c) => {
                    input.insert(cursor_pos, c);
                    cursor_pos += c.len_utf8();
                    redraw = true;
                }
                KeyCode::Enter => {
                    let trimmed = input.trim();
                    if !trimmed.is_empty() {
                        if history.last().map(|s| s.as_str()) != Some(trimmed) {
                            history.push(trimmed.to_string());
                            if history.len() > HISTORY_LIMIT {
                                history.remove(0);
                            }
                        }
                    }
                    disable_raw_mode().unwrap();
                    println!();
                    return trimmed.to_string();
                }
                _ => {}
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct MenuChoice {
    pub name: String,
    pub shortcut: char,
}

impl MenuChoice {
    pub fn to_display_string(&self) -> String {
        if let Some(pos) = self
            .name
            .to_lowercase()
            .find(self.shortcut.to_lowercase().to_string().as_str())
        {
            let mut result = String::with_capacity(self.name.len() + 2);
            result.push_str(&self.name[..pos]);
            result.push('[');
            result.push(self.shortcut);
            result.push(']');
            result.push_str(&self.name[pos + self.shortcut.len_utf8()..]);
            result
        } else {
            self.name.to_string()
        }
    }
}

pub async fn menu_selection(prompt: &str, choices: Vec<MenuChoice>, column: bool) -> u8 {
    let mut stdout = stdout();
    enable_raw_mode().unwrap();

    let item_separator = if column { "\n" } else { "   " };

    let render_choices = |choices: &Vec<MenuChoice>, current: u8| -> String {
        let mut items = String::new();

        for (i, choice) in choices.iter().enumerate() {
            if i as u8 == current {
                items.push_str(&colorize_text(
                    &format!(
                        "{}> {}{}",
                        if column { "\r" } else { "" },
                        choice.to_display_string(),
                        item_separator
                    ),
                    AnsiColor::Green,
                ));
            } else {
                items.push_str(&format!(
                    "{}  {}{}",
                    if column { "\r" } else { "" },
                    choice.to_display_string(),
                    item_separator
                ));
            }
        }

        format!("\r{}{}{}", prompt, if column { "\n" } else { " " }, items)
    };

    let mut current: u8 = 0;
    let mut lines_to_clear = 0;

    loop {
        for _ in 0..lines_to_clear {
            execute!(
                std::io::stdout(),
                cursor::MoveUp(1),
                terminal::Clear(terminal::ClearType::CurrentLine)
            )
            .unwrap();
        }

        let rendered = render_choices(&choices, current);
        lines_to_clear = rendered.matches('\n').count();
        stdout.write_all(rendered.as_bytes()).await.unwrap();
        stdout.flush().await.unwrap();

        if let Event::Key(key_event) = event::read().unwrap() {
            match key_event.code {
                KeyCode::Up => {
                    if current > 0 && column {
                        current -= 1;
                    }
                }
                KeyCode::Down => {
                    if current < (choices.len() as u8 - 1) && column {
                        current += 1;
                    }
                }
                KeyCode::Left => {
                    if current > 0 && !column {
                        current -= 1;
                    }
                }
                KeyCode::Right => {
                    if current < (choices.len() as u8 - 1) && !column {
                        current += 1;
                    }
                }
                KeyCode::Enter => {
                    break;
                }
                KeyCode::Char(c) => {
                    for (i, choice) in choices.iter().enumerate() {
                        if choice.shortcut.to_lowercase().next().unwrap_or_default() == c {
                            current = i as u8;
                            break;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    disable_raw_mode().unwrap();
    println!();
    current
}

pub async fn menu_toggle(
    prompt: &str,
    choices: Vec<(MenuChoice, bool)>,
) -> Vec<(MenuChoice, bool)> {
    let mut stdout = stdout();
    let mut choices: Vec<(MenuChoice, bool)> = choices;
    enable_raw_mode().unwrap();

    let render_choices = |choices: &Vec<(MenuChoice, bool)>, current: u8| -> String {
        let mut items = String::new();

        for (i, item) in choices.iter().enumerate() {
            items.push_str(&colorize_text(
                &format!(
                    "\r{} {}\n",
                    if item.1 == true { "●" } else { "○" },
                    item.0.to_display_string(),
                ),
                if i as u8 == current {
                    AnsiColor::Green
                } else {
                    AnsiColor::White
                },
            ));
        }

        format!("\r{}\n{}", prompt, items)
    };

    let mut current: u8 = 0;
    let mut lines_to_clear = 0;

    loop {
        for _ in 0..lines_to_clear {
            execute!(
                std::io::stdout(),
                cursor::MoveUp(1),
                terminal::Clear(terminal::ClearType::CurrentLine)
            )
            .unwrap();
        }

        let rendered = render_choices(&choices, current);
        lines_to_clear = rendered.matches('\n').count();
        stdout.write_all(rendered.as_bytes()).await.unwrap();
        stdout.flush().await.unwrap();

        if let Event::Key(key_event) = event::read().unwrap() {
            match key_event.code {
                KeyCode::Up => {
                    if current > 0 {
                        current -= 1;
                    }
                }
                KeyCode::Down => {
                    if current < (choices.len() as u8 - 1) {
                        current += 1;
                    }
                }
                KeyCode::Char(' ') => {
                    if let Some(choice) = choices.get_mut(current as usize) {
                        choice.1 = !choice.1;
                    }
                }
                KeyCode::Enter => {
                    break;
                }
                KeyCode::Char(c) => {
                    for (i, choice) in choices.iter().enumerate() {
                        if choice.0.shortcut.to_lowercase().next().unwrap_or_default() == c {
                            current = i as u8;
                            break;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    disable_raw_mode().unwrap();
    println!();
    choices
}
