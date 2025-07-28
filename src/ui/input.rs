use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
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
            let mut result = String::with_capacity(self.name.len() + 2); // +2 for the brackets
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

    let mut render_choices = async move |choices: Vec<MenuChoice>, current| {
        let mut items = String::new();

        for (i, choice) in choices.iter().enumerate() {
            if i as u8 == current {
                items.push_str(&colorize_text(
                    &format!("> {}{}", choice.to_display_string(), item_separator),
                    AnsiColor::Green,
                ));
            } else {
                items.push_str(&format!(
                    "  {}{}",
                    choice.to_display_string(),
                    item_separator
                ));
            }
        }

        stdout
            .write_all(format!("\r{} {}", prompt, items).as_bytes())
            .await
            .unwrap();

        stdout.flush().await.unwrap();
    };

    let mut current: u8 = 0;
    'key_loop: loop {
        render_choices(choices.clone(), current.clone()).await;
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
                            break 'key_loop;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    disable_raw_mode().unwrap();
    render_choices(choices.clone(), current.clone()).await;
    println!();
    current
}
