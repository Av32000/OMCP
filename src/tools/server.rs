use std::sync::Arc;

use reqwest::{Client, header::HeaderMap};
use rmcp::{
    RoleClient, ServiceExt,
    model::InitializeResult,
    service::RunningService,
    transport::{
        SseClientTransport, StreamableHttpClientTransport, TokioChildProcess,
        sse_client::SseClientConfig, streamable_http_client::StreamableHttpClientTransportConfig,
    },
};
use tokio::process::Command;

use crate::{AppResult, tools::tool::MCPTool};

#[derive(Debug, Clone)]
pub enum MCPServerConfig {
    Stdio {
        #[allow(dead_code)]
        name: String,
        command: String,
        args: Option<Vec<String>>,
        env: Option<Vec<String>>,
        disabled: bool,
    },
    SSE {
        #[allow(dead_code)]
        name: String,
        url: String,
        headers: Option<HeaderMap>,
        disabled: bool,
    },
    StreamableHttp {
        #[allow(dead_code)]
        name: String,
        url: String,
        headers: Option<HeaderMap>,
        disabled: bool,
    },
}

#[derive(Debug)]
pub struct MCPServer {
    pub config: MCPServerConfig,
    pub client: Option<RunningService<RoleClient, ()>>,
    pub peer_info: Option<InitializeResult>,
    pub tools: Vec<MCPTool>,
}

impl MCPServer {
    pub fn new(config: MCPServerConfig) -> Self {
        MCPServer {
            config,
            client: None,
            peer_info: None,
            tools: Vec::new(),
        }
    }

    pub async fn initialize(&mut self) -> AppResult<()> {
        let client = match &self.config {
            MCPServerConfig::Stdio {
                name: _,
                command,
                args,
                env,
                disabled,
            } => {
                if *disabled {
                    return Ok(());
                }

                let mut command = Command::new(command);

                if let Some(args) = args {
                    command.args(args.iter());
                }
                if let Some(env_vars) = env {
                    for var in env_vars {
                        let mut parts = var.splitn(2, '=');
                        if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                            command.env(key, value);
                        }
                    }
                }
                ().serve(TokioChildProcess::new(command)?).await?
            }
            MCPServerConfig::SSE {
                name: _,
                url,
                headers,
                disabled,
            } => {
                if *disabled {
                    return Ok(());
                }

                let mut reqwest_client = Client::builder();

                if let Some(headers) = headers {
                    reqwest_client = reqwest_client.default_headers(headers.clone());
                }

                let reqwest_client = reqwest_client.build()?;

                let config = SseClientConfig {
                    sse_endpoint: Arc::<str>::from(url.clone()),
                    ..Default::default()
                };

                ().serve(SseClientTransport::start_with_client(reqwest_client, config).await?)
                    .await?
            }
            MCPServerConfig::StreamableHttp {
                name: _,
                url,
                headers,
                disabled,
            } => {
                if *disabled {
                    return Ok(());
                }

                let mut reqwest_client = Client::builder();

                if let Some(headers) = headers {
                    reqwest_client = reqwest_client.default_headers(headers.clone());
                }

                let reqwest_client = reqwest_client.build()?;

                let config = StreamableHttpClientTransportConfig {
                    uri: Arc::<str>::from(url.clone()),
                    ..Default::default()
                };

                ().serve(StreamableHttpClientTransport::with_client(
                    reqwest_client,
                    config,
                ))
                .await?
            }
        };

        let (peer_info, tools) = MCPServer::fetch_info_from_client(&client).await?;

        self.client = Some(client);
        self.peer_info = Some(peer_info);
        self.tools = tools;

        Ok(())
    }

    async fn fetch_info_from_client(
        client: &RunningService<RoleClient, ()>,
    ) -> AppResult<(InitializeResult, Vec<MCPTool>)> {
        let peer_info = match client.peer_info() {
            Some(info) => info,
            None => return Err("Unable to fetch peer info".into()),
        };

        let fetched_tools_info = client.list_tools(Default::default()).await?;

        let mut tools = Vec::new();
        for tool in fetched_tools_info.tools {
            tools.push(MCPTool::new(tool));
        }

        return Ok((peer_info.clone(), tools));
    }
}
