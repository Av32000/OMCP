mod chat;
mod tools;

use std::sync::{Arc, Mutex};

use ollama_rs::generation::{chat::ChatMessage, tools::ToolInfo};
use tokio::io::{AsyncWriteExt, stdout};

use crate::{
    chat::OllamaChat,
    tools::{ToolManager, server::MCPServer, tool::ToToolInfo},
};

pub type AppResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[tokio::main]
async fn main() -> AppResult<()> {
    let mut stdout = stdout();

    let deep_wiki_mcp_server = MCPServer::new(tools::server::MCPServerConfig::StreamableHttp {
        name: "deep_wiki".to_string(),
        url: "https://mcp.deepwiki.com/mcp".to_string(),
        headers: None,
        disabled: false,
    });

    let fetch_mcp_server = MCPServer::new(tools::server::MCPServerConfig::StreamableHttp {
        name: "fetch".to_string(),
        url: "https://remote.mcpservers.org/fetch/mcp".to_string(),
        headers: None,
        disabled: false,
    });

    let tool_manager = Arc::new(tokio::sync::Mutex::new(ToolManager::new(vec![
        deep_wiki_mcp_server,
        fetch_mcp_server,
    ])));

    tool_manager.lock().await.initialize().await?;

    let mut ollama_chat = OllamaChat::new(Arc::clone(&tool_manager));
    loop {
        stdout.write_all(b"\n> ").await?;
        stdout.flush().await?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        let input = input.trim_end();
        if input.eq_ignore_ascii_case("/quit") {
            break;
        } else if input.eq_ignore_ascii_case("/clear") {
            ollama_chat.clear();
            stdout.write_all("Context cleared !".as_bytes()).await?;
            stdout.flush().await?;
            continue;
        } else if input.eq_ignore_ascii_case("/history") {
            dbg!(ollama_chat.get_history());
            continue;
        } else if input.eq_ignore_ascii_case("/tools") {
            dbg!(tool_manager.lock().await.get_enabled_tools());
            continue;
        }

        let mut stream = ollama_chat
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
