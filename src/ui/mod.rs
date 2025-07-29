use std::{
    process::exit,
    sync::{Arc, Mutex},
};

use ollama_rs::generation::chat::ChatMessage;
use tokio::io::{AsyncWriteExt, stdout};

use crate::{
    chat::OllamaChat,
    settings::SettingsManager,
    tools::ToolManager,
    ui::{
        tools::render_available_tools,
        utils::{AnsiColor, colorize_text},
    },
};

pub mod input;
pub mod tools;
pub mod utils;

pub use utils::RoundedBox;

pub struct AppUI {
    ollama_chat: OllamaChat,
    tool_manager: Arc<tokio::sync::Mutex<ToolManager>>,
    settings_manager: Arc<Mutex<SettingsManager>>,
    running: bool,
}

pub trait AppUIRenderable {
    fn render(&self, boxed: bool) -> String;
}

impl AppUI {
    pub fn new(
        ollama_chat: OllamaChat,
        tool_manager: Arc<tokio::sync::Mutex<ToolManager>>,
        settings_manager: Arc<Mutex<SettingsManager>>,
    ) -> Self {
        AppUI {
            ollama_chat,
            tool_manager,
            settings_manager,
            running: true,
        }
    }

    pub async fn run(&mut self) -> crate::AppResult<()> {
        let mut stdout = stdout();

        while self.running {
            let input = input::text_input("> ");

            let input = input.trim_end();
            if self.parse_command(input).await {
                continue;
            }

            let mut stream = self
                .ollama_chat
                .chat(vec![ChatMessage::user(input.to_string())])
                .await?;

            while let Some(res) = stream.recv().await {
                stdout.write_all(res.message.content.as_bytes()).await?;
                stdout.flush().await?;
            }

            println!()
        }
        Ok(())
    }

    pub fn exit(&mut self) {
        self.running = false;
        exit(0);
    }

    async fn parse_command(&mut self, input: &str) -> bool {
        if input.starts_with('/') {
            let parts: Vec<&str> = input.split_whitespace().collect();
            let command = parts.iter().next().unwrap_or(&"");
            let args = parts
                .get(1..)
                .map(|s| s.join(" "))
                .unwrap_or("".to_string());

            match command.to_lowercase().as_str() {
                "/quit" => {
                    println!("Bye !");
                    self.exit();
                }
                "/clear" => {
                    self.ollama_chat.clear();
                    println!("Context cleared !");
                }
                "/history" => {
                    dbg!(self.ollama_chat.get_history());
                }
                "/tools" => {
                    let tools = self.tool_manager.lock().await.get_tools();
                    println!("{}", render_available_tools(&tools));
                }
                "/settings" => match args.as_str() {
                    "show" => {
                        let settings = self.settings_manager.lock().unwrap();
                        println!("{}", settings.render(true));
                    }
                    "edit" => {
                        let mut settings = self.settings_manager.lock().unwrap();
                        settings.render_edit_menu().await;
                    }
                    _ => {
                        println!("Usage: /settings [show|edit]");
                    }
                },
                "/help" => {
                    let help = vec![
                        ("/quit", "Exit the application"),
                        ("/clear", "Clear the chat context"),
                        ("/history", "Show chat history"),
                        ("/tools", "List available tools"),
                        ("/settings [show|edit]", "Show or Edit current settings"),
                        ("/help", "Show this help message"),
                    ];

                    let mut help_text = String::new();

                    for h in help {
                        help_text.push_str(
                            &format!("{} : {}\n", colorize_text(h.0, AnsiColor::BrightCyan), h.1)
                                .to_string(),
                        );
                    }

                    println!(
                        "{}",
                        RoundedBox::new(
                            &help_text,
                            Some("Help"),
                            Some(AnsiColor::BrightCyan),
                            false
                        )
                        .render()
                    );
                }
                _ => {
                    println!("Unknown command: {}", command);
                }
            };

            return true;
        }

        return false;
    }
}
