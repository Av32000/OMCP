pub mod server;
pub mod tool;

use rmcp::model::{CallToolRequestParam, CallToolResult};
use serde_json::{Map, Value as JsonValue};
use server::MCPServer;

use crate::args::Args;
use crate::tools::server::MCPServerConfig;
use crate::{AppResult, tools::tool::MCPTool};
use crate::{ConfigFile, get_config_path};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct ToolManager {
    services: Vec<MCPServer>,
}

pub struct MCPServerConfigSchema {
    r#type: Option<String>,
    command: Option<String>,
    url: Option<String>,
    args: Option<Vec<String>>,
    env: Option<Vec<String>>,
    headers: Option<Map<String, JsonValue>>,
    disabled: Option<bool>,
}

impl ToolManager {
    pub fn new(services: Vec<MCPServer>) -> Self {
        ToolManager { services }
    }

    pub async fn initialize(&mut self) -> AppResult<()> {
        for service in &mut self.services {
            service.initialize().await?;
        }
        Ok(())
    }

    pub fn get_tools(&self) -> Vec<MCPTool> {
        self.services
            .iter()
            .map(|s| s.tools.clone())
            .flatten()
            .collect()
    }

    pub fn set_tool_status(&mut self, name: &str, enabled: bool) -> AppResult<()> {
        for service in &mut self.services {
            for tool in &mut service.tools {
                if tool.tool_info.name == name {
                    tool.enabled = enabled;
                    return Ok(());
                }
            }
        }
        Err(format!("Tool '{}' not found.", name).into())
    }

    pub fn get_enabled_tools(&self) -> Vec<MCPTool> {
        self.services
            .iter()
            .map(|s| s.tools.clone())
            .flatten()
            .filter(|t| t.enabled)
            .collect()
    }

    pub async fn call_tool(
        &self,
        name: String,
        arguments: Map<String, JsonValue>,
    ) -> AppResult<CallToolResult> {
        for service in &self.services {
            for tool in &service.tools {
                if tool.tool_info.name == name {
                    if let Some(client) = &service.client {
                        return Ok(client
                            .call_tool(CallToolRequestParam {
                                name: name.clone().into(),
                                arguments: Some(arguments.clone()),
                            })
                            .await?);
                    }
                }
            }
        }

        let error_message = format!("Tool '{}' not found.", name);
        Err(error_message.into())
    }

    pub fn load_mcp_servers_from_config(config_path: &Path) -> AppResult<Vec<MCPServer>> {
        // Read the configuration file
        let config_content = fs::read_to_string(config_path)?;
        let config_json: JsonValue = serde_json::from_str(&config_content)?;

        // Extract mcpServers
        let mcp_servers = config_json
            .get("mcpServers")
            .ok_or("Missing 'mcpServers' key in configuration")?
            .as_object()
            .ok_or("'mcpServers' should be a JSON object")?;

        let mut servers = Vec::new();

        for (name, server_config) in mcp_servers {
            let server_type = server_config
                .get("type")
                .and_then(|v| v.as_str())
                .map(String::from);
            let command = server_config
                .get("command")
                .and_then(|v| v.as_str())
                .map(String::from);
            let url = server_config
                .get("url")
                .and_then(|v| v.as_str())
                .map(String::from);
            let args = server_config
                .get("args")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect::<Vec<_>>()
                });
            let env = server_config
                .get("env")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect::<Vec<_>>()
                });
            let headers = server_config
                .get("headers")
                .and_then(|v| v.as_object())
                .cloned();
            let disabled = server_config
                .get("disabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            // Auto-detect type if not provided
            let detected_type = match (server_type, command.clone(), url.clone()) {
                (Some(t), _, _) => t,
                (None, Some(_), _) => "stdio".to_string(),
                (None, None, Some(_)) => "streamable_http".to_string(),
                _ => return Err("Invalid server configuration: missing required fields".into()),
            };

            // Validate required fields based on type
            match detected_type.as_str() {
                "stdio" => {
                    if command.is_none() {
                        return Err(format!(
                            "Server '{}' is missing required 'command' field",
                            name
                        )
                        .into());
                    }
                }
                "sse" | "streamable_http" => {
                    if url.is_none() {
                        return Err(
                            format!("Server '{}' is missing required 'url' field", name).into()
                        );
                    }
                }
                _ => {
                    return Err(format!(
                        "Unknown server type '{}' for server '{}'",
                        detected_type, name
                    )
                    .into());
                }
            }

            // Create MCPServerConfig
            let server_config = match detected_type.as_str() {
                "stdio" => MCPServerConfig::Stdio {
                    name: name.clone(),
                    command: command.ok_or("Missing 'command' for stdio server")?,
                    args,
                    env,
                    disabled,
                },
                "sse" => MCPServerConfig::SSE {
                    name: name.clone(),
                    url: url.ok_or("Missing 'url' for SSE server")?,
                    headers: headers.map(|h| {
                        h.into_iter()
                            .filter_map(|(k, v)| {
                                let key = k.parse().ok();
                                let value = v.as_str().and_then(|s| s.parse().ok());
                                match (key, value) {
                                    (Some(k), Some(v)) => Some((k, v)),
                                    _ => None,
                                }
                            })
                            .collect()
                    }),
                    disabled,
                },
                "streamable_http" => MCPServerConfig::StreamableHttp {
                    name: name.clone(),
                    url: url.ok_or("Missing 'url' for Streamable HTTP server")?,
                    headers: headers.map(|h| {
                        h.into_iter()
                            .filter_map(|(k, v)| {
                                let key = k.parse().ok();
                                let value = v.as_str().and_then(|s| s.parse().ok());
                                match (key, value) {
                                    (Some(k), Some(v)) => Some((k, v)),
                                    _ => None,
                                }
                            })
                            .collect()
                    }),
                    disabled,
                },
                _ => return Err(format!("Unknown server type for server '{}'.", name).into()),
            };

            // Create MCPServer
            servers.push(MCPServer::new(server_config));
        }

        Ok(servers)
    }

    pub fn load_mcp_server_from_args(args: Args) -> AppResult<Vec<MCPServer>> {
        let mut services = Vec::new();

        let json_mcp_configs = if !args.json_mcp_config.is_empty() {
            args.json_mcp_config
                .iter()
                .map(|s| PathBuf::from(s))
                .collect::<Vec<_>>()
        } else {
            vec![get_config_path(ConfigFile::MCPServers)]
        };

        for config in json_mcp_configs {
            let loaded_services: Vec<MCPServer> =
                ToolManager::load_mcp_servers_from_config(&config).unwrap_or_else(|_| {
                    eprintln!("Failed to load MCP servers from config: {:?}", config);
                    vec![]
                });
            services.extend(loaded_services);
        }

        for stdio_server in args.stdio_server {
            let (file_path, ext) = stdio_server
                .rsplit_once('.')
                .unwrap_or((stdio_server.as_str(), ""));

            let server = MCPServer::new(MCPServerConfig::Stdio {
                name: file_path.to_string(),
                command: if ext == "js" {
                    "node".to_string()
                } else if ext == "py" {
                    "python3".to_string()
                } else {
                    eprintln!("Unsupported file extension for stdio server: {}", ext);
                    continue;
                },
                args: vec![file_path.to_string()].into(),
                env: None,
                disabled: false,
            });
            services.push(server);
        }

        for sse_server in args.sse_server {
            let server = MCPServer::new(MCPServerConfig::SSE {
                name: sse_server.clone(),
                url: sse_server,
                headers: None,
                disabled: false,
            });
            services.push(server);
        }

        for http_server in args.streamable_http_server {
            let server = MCPServer::new(MCPServerConfig::StreamableHttp {
                name: http_server.clone(),
                url: http_server,
                headers: None,
                disabled: false,
            });
            services.push(server);
        }

        Ok(services)
    }
}
