use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    terminal::{self, disable_raw_mode, enable_raw_mode},
};
use tokio::io::{AsyncWriteExt, stdout};

use crate::ui::utils::{AnsiColor, colorize_text};

pub fn text_input(prompt: &str) -> String {
    use std::io::{self, Write};

    print!("{}", prompt);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
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
