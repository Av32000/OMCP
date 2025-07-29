mod chat;
mod settings;
mod tools;
mod ui;

use std::sync::{Arc, Mutex};

use crate::{
    chat::OllamaChat,
    settings::SettingsManager,
    tools::{ToolManager, server::MCPServer},
};

pub type AppResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[tokio::main]
async fn main() -> AppResult<()> {
    let settings_manager = match SettingsManager::load_from_file(SettingsManager::get_config_path())
    {
        Ok(settings) => Arc::new(Mutex::new(settings)),
        Err(e) => {
            eprintln!("Failed to load settings from config file. Loading default config",);
            Arc::new(Mutex::new(SettingsManager::default()))
        }
    };

    settings_manager
        .lock()
        .unwrap()
        .save_to_file(SettingsManager::get_config_path())
        .expect("Failed to save settings to config file");

    let time_mcp_server = MCPServer::new(tools::server::MCPServerConfig::Stdio {
        name: "time".to_string(),
        command: "uvx".to_string(),
        args: Some(vec!["mcp-server-time".to_string()]),
        env: None,
        disabled: false,
    });

    let tool_manager = Arc::new(tokio::sync::Mutex::new(ToolManager::new(vec![
        time_mcp_server,
    ])));

    tool_manager.lock().await.initialize().await?;

    let ollama_chat = OllamaChat::new(Arc::clone(&tool_manager), Arc::clone(&settings_manager));

    let mut app_ui = ui::AppUI::new(ollama_chat, tool_manager, settings_manager);
    app_ui.run().await?;

    Ok(())
}
