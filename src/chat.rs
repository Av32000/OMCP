use std::sync::{Arc, Mutex};

use ollama_rs::{
    Ollama,
    generation::chat::{ChatMessage, ChatMessageResponse, request::ChatMessageRequest},
};
use tokio::sync::mpsc::{self, Receiver};
use tokio_stream::StreamExt;

use crate::AppResult;

#[derive(Debug, Clone)]
pub struct ChatHistory {
    pub messages: Arc<Mutex<Vec<ChatMessage>>>,
}

impl ChatHistory {
    pub fn new() -> Self {
        ChatHistory {
            messages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn get_history(&self) -> Arc<Mutex<Vec<ChatMessage>>> {
        self.messages.clone()
    }

    pub fn clear_messages(&mut self) -> Result<(), String> {
        let mut history_guard = self
            .messages
            .lock()
            .map_err(|e| format!("Failed to lock history for clearing: {}", e))?;
        history_guard.clear();
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct OllamaChat {
    ollama: Ollama,
    history: ChatHistory,
}

impl OllamaChat {
    pub fn new() -> Self {
        OllamaChat {
            ollama: Ollama::default(),
            history: ChatHistory::new(),
        }
    }

    pub async fn chat(
        &self,
        messages: Vec<ChatMessage>,
    ) -> AppResult<Receiver<ChatMessageResponse>> {
        let mut stream = match self
            .ollama
            .send_chat_messages_with_history_stream(
                self.history.get_history(),
                ChatMessageRequest::new("qwen2.5:7b".to_string(), messages),
            )
            .await
        {
            Ok(stream) => stream,
            Err(err) => return Err(Box::new(err)),
        };

        let (tx, rx) = mpsc::channel(32);

        tokio::spawn(async move {
            while let Some(Ok(res)) = stream.next().await {
                if res.message.tool_calls.len() > 0 {
                    todo!("Tool call not yet handled")
                } else {
                    if tx.send(res).await.is_err() {
                        eprintln!("Chat response stream was closed");
                        return;
                    };
                }
            }
        });

        Ok(rx)
    }

    pub fn clear(&mut self) {
        let _ = self.history.clear_messages();
    }

    pub fn get_history(&self) -> Arc<Mutex<Vec<ChatMessage>>> {
        return self.history.get_history();
    }
}
