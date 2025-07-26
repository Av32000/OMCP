pub mod server;
pub mod tool;

use server::MCPServer;
// pub struct AvailableTool {
//     tool: Tool,
//     enabled: bool,
// }

// impl AvailableTool {
//     pub fn new(tool: Tool) -> Self {
//         AvailableTool {
//             tool,
//             enabled: true,
//         }
//     }
// }

pub struct ToolManager {
    services: Vec<MCPServer>,
}

impl ToolManager {
    pub fn new() -> Self {
        ToolManager {
            services: Vec::new(),
        }
    }
}
