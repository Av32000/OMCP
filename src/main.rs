mod chat;

use ollama_rs::generation::chat::ChatMessage;
use tokio::io::{AsyncWriteExt, stdout};
use tokio_stream::StreamExt;

use crate::chat::OllamaChat;

pub type AppResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[tokio::main]
async fn main() -> AppResult<()> {
    let mut ollama_chat = OllamaChat::new();
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
            ollama_chat.clear();
            stdout.write_all("Context cleared !".as_bytes()).await?;
            stdout.flush().await?;
            continue;
        } else if input.eq_ignore_ascii_case("/history") {
            dbg!(ollama_chat.get_history());
            continue;
        }

        let mut stream = ollama_chat
            .chat(vec![ChatMessage::user(input.to_string())])
            .await?;

        let mut response = String::new();
        while let Some(Ok(res)) = stream.next().await {
            stdout.write_all(res.message.content.as_bytes()).await?;
            stdout.flush().await?;
            response += res.message.content.as_str();
        }
    }

    Ok(())
}
