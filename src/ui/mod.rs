use std::sync::{Arc, Mutex};

use ollama_rs::generation::chat::ChatMessage;
use tokio::io::{AsyncWriteExt, stdout};

use crate::{chat::OllamaChat, settings::SettingsManager, ui::tools::render_available_tools};

mod tools;
pub mod utils;

pub use utils::RoundedBox;

pub struct AppUI {
    ollama_chat: OllamaChat,
    tool_manager: Arc<tokio::sync::Mutex<crate::tools::ToolManager>>,
    settings_manager: Arc<Mutex<SettingsManager>>,
}

pub trait AppUIRenderable {
    fn render(&self, boxed: bool) -> String;
}

impl AppUI {
    pub fn new(
        ollama_chat: OllamaChat,
        tool_manager: Arc<tokio::sync::Mutex<crate::tools::ToolManager>>,
        settings_manager: Arc<Mutex<SettingsManager>>,
    ) -> Self {
        AppUI {
            ollama_chat,
            tool_manager,
            settings_manager,
        }
    }

    pub async fn run(&mut self) -> crate::AppResult<()> {
        let mut stdout = stdout();

        loop {
            stdout.write_all(b"\n> ").await?;
            stdout.flush().await?;

            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;

            let input = input.trim_end();
            if input.eq_ignore_ascii_case("/quit") {
                break;
            } else if input.eq_ignore_ascii_case("/clear") {
                self.ollama_chat.clear();
                stdout.write_all("Context cleared !".as_bytes()).await?;
                stdout.flush().await?;
                continue;
            } else if input.eq_ignore_ascii_case("/history") {
                dbg!(self.ollama_chat.get_history());
                continue;
            } else if input.eq_ignore_ascii_case("/tools") {
                stdout
                    .write_all(
                        render_available_tools(&self.tool_manager.lock().await.get_tools())
                            .as_bytes(),
                    )
                    .await?;
                stdout.flush().await?;
                continue;
            } else if input.eq_ignore_ascii_case("/settings") {
                stdout
                    .write_all(
                        self.settings_manager
                            .lock()
                            .unwrap()
                            .render(true)
                            .as_bytes(),
                    )
                    .await?;
                stdout.flush().await?;
                continue;
            }

            let mut stream = self
                .ollama_chat
                .chat(vec![ChatMessage::user(input.to_string())])
                .await?;

            let mut response = String::new();
            while let Some(res) = stream.recv().await {
                stdout.write_all(res.message.content.as_bytes()).await?;
                stdout.flush().await?;
                response += res.message.content.as_str();
            }
        }
        Ok(())
    }
}
