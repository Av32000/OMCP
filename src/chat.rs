use std::sync::{Arc, Mutex};

use ollama_rs::{
    Ollama,
    generation::{
        chat::{ChatMessage, ChatMessageResponse, MessageRole, request::ChatMessageRequest},
        tools::ToolInfo,
    },
    models::ModelOptions,
};
use tokio::{
    io::{AsyncWriteExt, stdout},
    sync::mpsc::{self, Receiver},
};
use tokio_stream::StreamExt;

use crate::{
    AppResult,
    settings::SettingsManager,
    tools::{ToolManager, tool::ToToolInfo},
    ui::{
        input::{self, MenuChoice},
        tools::{render_tool_call_request, render_tool_call_result},
    },
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
    pub ollama: Ollama,
    history: ChatHistory,
    tool_manager: Arc<tokio::sync::Mutex<ToolManager>>,
    settings_manager: Arc<Mutex<SettingsManager>>,
}

impl OllamaChat {
    pub fn new(
        tool_manager: Arc<tokio::sync::Mutex<ToolManager>>,
        settings_manager: Arc<Mutex<SettingsManager>>,
        ollama_host: Option<String>,
    ) -> Self {
        let ollama = if let Some(host) = ollama_host {
            let (host, port) = host
                .split_once(':')
                .map(|(h, p)| (h.to_string(), p.parse::<u16>().unwrap_or(11434)))
                .unwrap_or((host.clone(), 11434));

            Ollama::new(host, port)
        } else {
            Ollama::default()
        };

        OllamaChat {
            ollama: Ollama::default(),
            history: ChatHistory::new(),
            tool_manager,
            settings_manager,
        }
    }

    pub async fn chat(
        &self,
        messages: Vec<ChatMessage>,
    ) -> AppResult<Receiver<ChatMessageResponse>> {
        let model_name = self.settings_manager.lock().unwrap().model_name.clone();
        let mut messages = messages;

        let tools_capability = self
            .ollama
            .show_model_info(model_name.clone())
            .await
            .unwrap()
            .capabilities
            .contains(&"tools".to_string());

        let thinking_capability = self
            .ollama
            .show_model_info(model_name.clone())
            .await
            .unwrap()
            .capabilities
            .contains(&"thinking".to_string());

        let mut model_options = ModelOptions::default();

        {
            let settings = self.settings_manager.lock().unwrap();
            model_options = model_options
                .seed(settings.model_seed)
                .temperature(settings.model_temperature);

            if !settings.model_system_prompt.is_empty() {
                if messages.is_empty() || messages[0].role != MessageRole::System {
                    messages.insert(0, ChatMessage::system(settings.model_system_prompt.clone()));
                } else {
                    messages[0].content = settings.model_system_prompt.clone();
                }
            }
        }

        let mut request =
            ChatMessageRequest::new(model_name.clone(), messages).options(model_options.clone());
        let tools: Vec<ToolInfo> = self
            .tool_manager
            .lock()
            .await
            .get_enabled_tools()
            .iter()
            .map(|t| t.tool_info.to_tool_info())
            .collect();

        if tools_capability {
            request = request.tools(tools.clone());
        }

        if thinking_capability {
            request = request.think(true);
        }

        let mut stream = match self
            .ollama
            .send_chat_messages_with_history_stream(self.history.get_history(), request)
            .await
        {
            Ok(stream) => stream,
            Err(err) => return Err(Box::new(err)),
        };

        let (tx, rx) = mpsc::channel(32);

        let tool_manager = self.tool_manager.clone();

        let history = self.history.clone();
        let tool_confirmation = self.settings_manager.lock().unwrap().tool_confirmation;
        let verbose_tool_calls = self.settings_manager.lock().unwrap().verbose_tool_calls;
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

                        let mut stdout = stdout();
                        if verbose_tool_calls || tool_confirmation {
                            stdout
                                .write_all(
                                    format!(
                                        "{}\n",
                                        render_tool_call_request(
                                            call.function.name.clone(),
                                            args.clone()
                                        )
                                    )
                                    .as_bytes(),
                                )
                                .await
                                .unwrap();
                            stdout.flush().await.unwrap();
                        }

                        let mut call_tool = true;
                        if tool_confirmation {
                            let confirm = input::menu_selection(
                                "Confirm tool call : ",
                                vec![
                                    MenuChoice {
                                        name: "Yes".to_string(),
                                        shortcut: 'Y',
                                    },
                                    MenuChoice {
                                        name: "No".to_string(),
                                        shortcut: 'N',
                                    },
                                ],
                                false,
                            )
                            .await;

                            if confirm == 1 {
                                call_tool = false;
                            }
                        }

                        if call_tool {
                            match tool_manager
                                .lock()
                                .await
                                .call_tool(call.function.name.clone(), args)
                                .await
                            {
                                Ok(result) => {
                                    if verbose_tool_calls || tool_confirmation {
                                        stdout
                                            .write_all(
                                                format!(
                                                    "{}\n",
                                                    render_tool_call_result(&result.content)
                                                )
                                                .as_bytes(),
                                            )
                                            .await
                                            .unwrap();
                                        stdout.flush().await.unwrap();
                                    }

                                    tool_messages.push(ChatMessage::tool(
                                        serde_json::to_string(&result.content).unwrap_or_default(),
                                    ));
                                }
                                Err(err) => {
                                    eprintln!(
                                        "Error calling tool {}: {:?}",
                                        call.function.name, err
                                    );
                                    continue;
                                }
                            }
                        } else {
                            tool_messages
                                .push(ChatMessage::tool("Tool cancelled by user".to_string()));
                        }
                    }

                    {
                        let mut history_guard = history.messages.lock().unwrap();
                        for msg in &tool_messages {
                            history_guard.push(msg.clone());
                        }
                    }

                    let mut request = ChatMessageRequest::new(model_name.clone(), tool_messages)
                        .options(model_options.clone());
                    if tools_capability {
                        request = request.tools(tools.clone());
                    }

                    if thinking_capability {
                        request = request.think(true);
                    }

                    let followup_stream = match Ollama::default()
                        .send_chat_messages_with_history_stream(history.get_history(), request)
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
