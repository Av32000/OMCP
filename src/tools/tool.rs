use ollama_rs::generation::tools::{ToolFunctionInfo, ToolInfo};
use rmcp::model::Tool;

#[derive(Debug, Clone)]
pub struct MCPTool {
    pub tool_info: Tool,
    pub enabled: bool,
}

pub trait ToToolInfo {
    fn to_tool_info(&self) -> ToolInfo;
}

impl ToToolInfo for Tool {
    fn to_tool_info(&self) -> ToolInfo {
        ToolInfo {
            tool_type: ollama_rs::generation::tools::ToolType::Function,
            function: ToolFunctionInfo {
                name: self.name.to_string(),
                description: self
                    .description
                    .as_ref()
                    .map(|cow| cow.to_string())
                    .unwrap_or_default(),
                parameters: (*self.input_schema).clone().into(),
            },
        }
    }
}

impl MCPTool {
    pub fn new(tool_info: Tool) -> Self {
        MCPTool {
            tool_info,
            enabled: true,
        }
    }
}
