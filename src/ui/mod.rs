use std::{
    process::exit,
    sync::{Arc, Mutex},
};

use ollama_rs::generation::{
    chat::ChatMessage, completion::request::GenerationRequest, parameters::KeepAlive,
};
use tokio::io::{AsyncWriteExt, stdout};
use tokio_stream::StreamExt;

use crate::{
    chat::OllamaChat,
    model::{render_model_info, select_model},
    settings::SettingsManager,
    tools::ToolManager,
    ui::{
        input::MenuChoice,
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

        let mut is_thinking = false;

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
                if let Some(message) = res.message.thinking {
                    if self.settings_manager.lock().unwrap().show_thinking {
                        if !is_thinking {
                            stdout
                                .write_all(
                                    colorize_text("<thinking>\n", AnsiColor::BrightBlack)
                                        .as_bytes(),
                                )
                                .await?;
                        }
                        stdout
                            .write_all(colorize_text(&message, AnsiColor::BrightBlack).as_bytes())
                            .await?;
                        stdout.flush().await?;
                    }
                    is_thinking = true;
                } else {
                    if is_thinking {
                        if self.settings_manager.lock().unwrap().show_thinking {
                            stdout
                                .write_all(
                                    colorize_text("</thinking>\n", AnsiColor::BrightBlack)
                                        .as_bytes(),
                                )
                                .await?;
                        }
                        is_thinking = false;
                    }
                    stdout.write_all(res.message.content.as_bytes()).await?;
                    stdout.flush().await?;
                }
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
            let args = parts.get(1..).unwrap_or_default();

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
                "/tools" => match *args.get(0).unwrap_or(&&"") {
                    "show" => {
                        let tools = self.tool_manager.lock().await;
                        println!("{}", render_available_tools(&tools.get_tools()));
                    }
                    "toggle" => {
                        let tool = *args.get(1).unwrap_or(&&"");
                        let mut tools = self.tool_manager.lock().await;
                        if tool.is_empty() {
                            let choices = tools
                                .get_tools()
                                .iter()
                                .map(|tool| {
                                    (
                                        MenuChoice {
                                            name: tool.tool_info.name.to_string(),
                                            shortcut: '#',
                                        },
                                        tool.enabled,
                                    )
                                })
                                .collect::<Vec<_>>();
                            let selected = input::menu_toggle("Toggle Tools : ", choices).await;

                            for choice in selected.iter() {
                                tools.set_tool_status(&choice.0.name, choice.1).unwrap();
                            }
                        } else {
                            tools.toggle_tool_status(tool).unwrap_or_else(|err| {
                                eprintln!("{}", err);
                            });
                        }
                        println!("{}", render_available_tools(&tools.get_tools()));
                    }
                    _ => {
                        println!("Usage: /tools [show|toggle]");
                    }
                },
                "/settings" => match *args.get(0).unwrap_or(&&"") {
                    "show" => {
                        let settings = self.settings_manager.lock().unwrap();
                        println!("{}", settings.render(true));
                    }
                    "edit" => {
                        let mut settings = self.settings_manager.lock().unwrap();
                        settings.render_edit_menu().await;
                    }
                    "save" => {
                        let settings = self.settings_manager.lock().unwrap();
                        settings
                            .save_to_file(&settings.config_file_path)
                            .unwrap_or_else(|err| {
                                eprintln!("Error saving settings: {}", err);
                            });
                    }
                    _ => {
                        println!("Usage: /settings [show|edit]");
                    }
                },
                "/model" => match *args.get(0).unwrap_or(&&"") {
                    "info" => {
                        let model: String = (*args.get(1).unwrap_or(
                            &self
                                .settings_manager
                                .lock()
                                .unwrap()
                                .model_name
                                .clone()
                                .as_str(),
                        ))
                        .to_string();

                        println!(
                            "{}",
                            RoundedBox::new(
                                &render_model_info(model, &self.ollama_chat.ollama).await,
                                Some("Model Info"),
                                Some(AnsiColor::BrightBlue),
                                false
                            )
                            .render()
                        );
                    }
                    "select" => {
                        let mut model: String = (*args.get(1).unwrap_or(&"")).to_string();

                        if model.is_empty() {
                            model = select_model(&self.ollama_chat.ollama).await.unwrap_or_else(
                                |err| {
                                    eprintln!("Error selecting model: {}", err);
                                    return String::new();
                                },
                            );
                        }

                        if !model.is_empty() {
                            let mut settings = self.settings_manager.lock().unwrap();
                            settings.model_name = model.to_string();
                            if settings.auto_save_config {
                                settings
                                    .save_to_file(&settings.config_file_path)
                                    .unwrap_or_else(|err| {
                                        eprintln!("Error saving settings: {}", err);
                                    });
                            }

                            println!(
                                "{}",
                                RoundedBox::new(
                                    &render_model_info(
                                        settings.model_name.clone(),
                                        &self.ollama_chat.ollama
                                    )
                                    .await,
                                    Some("Model Info"),
                                    Some(AnsiColor::BrightBlue),
                                    false
                                )
                                .render()
                            );
                        }
                    }
                    "load" => {
                        let model: String = (*args.get(1).unwrap_or(
                            &self
                                .settings_manager
                                .lock()
                                .unwrap()
                                .model_name
                                .clone()
                                .as_str(),
                        ))
                        .to_string();

                        match self
                            .ollama_chat
                            .ollama
                            .generate(GenerationRequest::new(model, ""))
                            .await
                        {
                            Ok(_) => println!("Model loaded successfully!"),
                            Err(err) => eprintln!("Error loading model: {}", err),
                        };
                    }
                    "unload" => {
                        let model: String = (*args.get(1).unwrap_or(
                            &self
                                .settings_manager
                                .lock()
                                .unwrap()
                                .model_name
                                .clone()
                                .as_str(),
                        ))
                        .to_string();

                        match self
                            .ollama_chat
                            .ollama
                            .generate(
                                GenerationRequest::new(model, "")
                                    .keep_alive(KeepAlive::UnloadOnCompletion),
                            )
                            .await
                        {
                            Ok(_) => println!("Model unloaded successfully!"),
                            Err(err) => eprintln!("Error unloading model: {}", err),
                        };
                    }
                    "pull" => {
                        let model: String = (*args.get(1).unwrap_or(
                            &self
                                .settings_manager
                                .lock()
                                .unwrap()
                                .model_name
                                .clone()
                                .as_str(),
                        ))
                        .to_string();

                        match self
                            .ollama_chat
                            .ollama
                            .pull_model_stream(model, false)
                            .await
                        {
                            Ok(mut stream) => {
                                while let Some(Ok(res)) = stream.next().await {
                                    let mut printed_message = res.message;
                                    if let Some(total) = res.total {
                                        if let Some(completed) = res.completed {
                                            printed_message.push_str(&format!(
                                                " ({}%)",
                                                completed * 100 / total
                                            ));
                                        }
                                    }

                                    if printed_message == "success" {
                                        printed_message = "Model pulled successfully!".to_string();
                                    }

                                    print!("\r\x1b[K{}", printed_message);
                                    use std::io::{self, Write};
                                    io::stdout().flush().unwrap();
                                }
                                println!(); // Add final newline when done
                            }
                            Err(err) => eprintln!("Error pulling model: {}", err),
                        }
                    }
                    _ => {
                        println!("Usage: /model [info|select|load|unload|pull] <model>");
                    }
                },
                "/help" => {
                    let help = vec![
                        ("/quit", "Exit the application"),
                        ("/clear", "Clear the chat context"),
                        ("/history", "Show chat history"),
                        ("/tools [show|toggle]", "List or Toggle available tools"),
                        (
                            "/settings [show|edit|save]",
                            "Show, Edit or Save current settings",
                        ),
                        (
                            "/model [info|select|load|unload|pull] <model>",
                            "Manage model used by Ollama. <model> is optional",
                        ),
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
