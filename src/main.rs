mod chat;
mod settings;
mod tools;
mod ui;

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{chat::OllamaChat, settings::SettingsManager, tools::ToolManager};

pub type AppResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub enum ConfigFile {
    Settings,
    MCPServers,
}

impl ConfigFile {
    pub fn file_name(&self) -> &'static str {
        match self {
            ConfigFile::Settings => "settings.json",
            ConfigFile::MCPServers => "mcp_servers.json",
        }
    }
}

pub fn get_config_path(file: ConfigFile) -> PathBuf {
    let crate_name = env!("CARGO_PKG_NAME");
    let mut config_path = dirs::config_dir()
        .expect("Failed to get config directory")
        .join(crate_name);
    if !config_path.exists() {
        std::fs::create_dir_all(&config_path).expect("Failed to create config directory");
    }
    config_path.push(file.file_name());
    config_path
}

#[tokio::main]
async fn main() -> AppResult<()> {
    let settings_manager =
        match SettingsManager::load_from_file(&get_config_path(ConfigFile::Settings)) {
            Ok(settings) => Arc::new(Mutex::new(settings)),
            Err(e) => {
                eprintln!("Failed to load settings from config file. Loading default config",);
                Arc::new(Mutex::new(SettingsManager::default()))
            }
        };

    settings_manager
        .lock()
        .unwrap()
        .save_to_file(&get_config_path(ConfigFile::Settings))
        .expect("Failed to save settings to config file");

    let tool_manager = Arc::new(tokio::sync::Mutex::new(ToolManager::new(
        ToolManager::load_mcp_servers_from_config(&get_config_path(ConfigFile::MCPServers))
            .unwrap_or_default(),
    )));

    tool_manager.lock().await.initialize().await?;

    let ollama_chat = OllamaChat::new(Arc::clone(&tool_manager), Arc::clone(&settings_manager));

    let mut app_ui = ui::AppUI::new(ollama_chat, tool_manager, settings_manager);
    app_ui.run().await?;

    Ok(())
}
