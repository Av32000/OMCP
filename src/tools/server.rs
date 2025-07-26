use std::collections::HashMap;

use rmcp::{
    RoleClient, ServiceExt, model::InitializeResult, service::RunningService,
    transport::TokioChildProcess,
};
use tokio::process::Command;

use crate::{AppResult, tools::tool::MCPTool};

#[derive(Debug, Clone)]
pub enum MCPServerConfig {
    Stdio {
        name: String,
        command: String,
        args: Option<Vec<String>>,
        env: Option<Vec<String>>,
        disabled: bool,
    },
    SSE {
        name: String,
        url: String,
        headers: Option<HashMap<String, String>>,
        disabled: bool,
    },
    StreamableHttp {
        name: String,
        url: String,
        headers: Option<HashMap<String, String>>,
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
        match &self.config {
            MCPServerConfig::Stdio {
                name,
                command,
                args,
                env,
                disabled,
            } => {
                if !*disabled {
                    let mut command = Command::new(command);

                    if let Some(args) = args {
                        command.args(args.iter());
                    }

                    let client = ().serve(TokioChildProcess::new(command)?).await?;

                    let (peer_info, tools) = MCPServer::fetch_info_from_client(&client).await?;

                    self.client = Some(client);
                    self.peer_info = Some(peer_info);
                    self.tools = tools;
                }
            }
            _ => {}
        }

        Ok(())
    }

    pub async fn fetch_info_from_client(
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
