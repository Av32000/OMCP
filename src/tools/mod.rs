pub mod server;
pub mod tool;

use rmcp::{
    model::{CallToolRequestParam, CallToolResult},
};
use serde_json::{Map, Value as JsonValue};
use server::MCPServer;

use crate::{AppResult, tools::tool::MCPTool};

#[derive(Debug)]
pub struct ToolManager {
    services: Vec<MCPServer>,
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
}
