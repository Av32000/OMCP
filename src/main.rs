mod args;
mod chat;
mod model;
mod settings;
mod tools;
mod ui;

use ollama_rs::generation::chat::ChatMessage;

use crate::{chat::OllamaChat, settings::SettingsManager, tools::ToolManager};
use std::{
    path::PathBuf,
    process::exit,
    sync::{Arc, Mutex},
};

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
    let args = args::Args::parse();

    let json_config_path = if let Some(config_path) = args.config.clone() {
        PathBuf::from(&config_path)
    } else {
        get_config_path(ConfigFile::Settings)
    };

    let settings_manager = match SettingsManager::load_from_file(&json_config_path) {
        Ok(settings) => Arc::new(Mutex::new(settings)),
        Err(_) => {
            eprintln!("Failed to load settings from config file. Loading default config",);
            Arc::new(Mutex::new(SettingsManager::default()))
        }
    };

    {
        let mut settings_manager_lock = settings_manager.lock().unwrap();

        settings_manager_lock.config_file_path = json_config_path;

        if args.model.is_some() {
            if let Some(model) = args.model.clone() {
                settings_manager_lock.model_name = model;
            }
        }

        if settings_manager_lock.auto_save_config {
            settings_manager_lock
                .save_to_file(&settings_manager_lock.config_file_path)
                .expect("Failed to save settings to config file");
        }
    }

    let tool_manager = Arc::new(tokio::sync::Mutex::new(ToolManager::new(
        ToolManager::load_mcp_server_from_args(args.clone())?,
    )));

    tool_manager.lock().await.initialize().await?;

    let ollama_chat = OllamaChat::new(
        Arc::clone(&tool_manager),
        Arc::clone(&settings_manager),
        args.ollama_host.clone(),
    );

    if args.prompt.is_some() {
        settings_manager.lock().unwrap().verbose_tool_calls = false;
        settings_manager.lock().unwrap().tool_confirmation = false;

        let prompt = args.prompt.unwrap_or_else(|| {
            eprintln!("No prompt provided. Exiting.");
            exit(1);
        });

        let mut output = String::new();
        let mut stream = ollama_chat.chat(vec![ChatMessage::user(prompt)]).await?;

        while let Some(chunk) = stream.recv().await {
            output.push_str(&chunk.message.content);
        }

        println!("{}", output);

        return Ok(());
    }

    let mut app_ui = ui::AppUI::new(ollama_chat, tool_manager, settings_manager);
    app_ui.run().await?;

    Ok(())
}
