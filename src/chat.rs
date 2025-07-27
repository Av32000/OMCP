use std::sync::{Arc, Mutex};

use ollama_rs::{
    Ollama,
    generation::chat::{ChatMessage, ChatMessageResponse, request::ChatMessageRequest},
};
use tokio::sync::mpsc::{self, Receiver};
use tokio_stream::StreamExt;

use crate::{
    AppResult,
    tools::{ToolManager, tool::ToToolInfo},
};

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
    tool_manager: Arc<tokio::sync::Mutex<ToolManager>>,
}

impl OllamaChat {
    pub fn new(tool_manager: Arc<tokio::sync::Mutex<ToolManager>>) -> Self {
        OllamaChat {
            ollama: Ollama::default(),
            history: ChatHistory::new(),
            tool_manager,
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
                ChatMessageRequest::new("qwen2.5:7b".to_string(), messages).tools(
                    self.tool_manager
                        .lock()
                        .await
                        .get_enabled_tools()
                        .iter()
                        .map(|t| t.tool_info.to_tool_info())
                        .collect(),
                ),
            )
            .await
        {
            Ok(stream) => stream,
            Err(err) => return Err(Box::new(err)),
        };

        let (tx, rx) = mpsc::channel(32);

        let tool_manager = self.tool_manager.clone();

        let history = self.history.clone();
        tokio::spawn(async move {
            while let Some(Ok(res)) = stream.next().await {
                {
                    let mut history_guard = history.messages.lock().unwrap();
                    history_guard.push(res.message.clone());
                }

                if !res.message.tool_calls.is_empty() {
                    let mut tool_messages = Vec::new();

                    for call in res.message.tool_calls {
                        let args = match serde_json::json!(call.function.arguments)
                            .as_object()
                            .cloned()
                        {
                            Some(args) => args,
                            None => continue,
                        };

                        match tool_manager
                            .lock()
                            .await
                            .call_tool(call.function.name.clone(), args)
                            .await
                        {
                            Ok(result) => {
                                tool_messages.push(ChatMessage::tool(
                                    serde_json::to_string(&result.content).unwrap_or_default(),
                                ));
                            }
                            Err(err) => {
                                eprintln!("Error calling tool {}: {:?}", call.function.name, err);
                                continue;
                            }
                        }
                    }

                    {
                        let mut history_guard = history.messages.lock().unwrap();
                        for msg in &tool_messages {
                            history_guard.push(msg.clone());
                        }
                    }

                    let followup_stream = match Ollama::default()
                        .send_chat_messages_with_history_stream(
                            history.get_history(),
                            ChatMessageRequest::new("qwen2.5:7b".to_string(), tool_messages),
                        )
                        .await
                    {
                        Ok(s) => s,
                        Err(err) => {
                            eprintln!("Failed to send tool response back to Ollama: {:?}", err);
                            break;
                        }
                    };

                    stream = followup_stream;
                } else {
                    if tx.send(res).await.is_err() {
                        eprintln!("Chat response stream closed");
                        break;
                    }
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
