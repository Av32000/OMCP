use rmcp::model::Tool;

#[derive(Debug, Clone)]
pub struct MCPTool {
    tool_info: Tool,
    enabled: bool,
}

impl MCPTool {
    pub fn new(tool_info: Tool) -> Self {
        MCPTool {
            tool_info,
            enabled: false,
        }
    }
}
