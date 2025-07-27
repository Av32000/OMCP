pub mod server;
pub mod tool;

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
}
