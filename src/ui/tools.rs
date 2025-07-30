use rmcp::model::{Annotated, RawContent};
use serde_json::{Map, Value};

use crate::{
    tools::tool::MCPTool,
    ui::{
        RoundedBox,
        utils::{AnsiColor, colorize_text},
    },
};

pub fn render_available_tools(tools: &[MCPTool]) -> String {
    let mut output = String::new();
    for tool in tools {
        let header = if tool.enabled {
            colorize_text(
                format!("{} {}", "[✔]", tool.tool_info.name).as_str(),
                AnsiColor::BrightGreen,
            )
        } else {
            colorize_text(
                format!("{} {}", "[✘]", tool.tool_info.name).as_str(),
                AnsiColor::BrightRed,
            )
        };

        let description = tool
            .tool_info
            .description
            .as_ref()
            .map(|desc| desc.to_string())
            .unwrap_or_else(|| "No description available".to_string());

        let result = format!("{} : {}", header, description);

        output.push_str(&result);
        output.push('\n');
    }

    if output.is_empty() {
        output = "No tools available".to_string();
    }

    RoundedBox::new(
        &output,
        Some("Available Tools"),
        Some(AnsiColor::BrightBlue),
        false,
    )
    .render()
}

pub fn render_tool_call_request(name: String, args: Map<String, Value>) -> String {
    RoundedBox::new(
        &format!(
            "Name: {}\nArguments: \n{}",
            name,
            serde_json::to_string_pretty(&args).unwrap_or_default()
        ),
        Some("Tool Call Request"),
        Some(AnsiColor::BrightMagenta),
        false,
    )
    .render()
}

pub fn render_tool_call_result(result: &Vec<Annotated<RawContent>>) -> String {
    let result = match result.get(0) {
        Some(first_result) => serde_json::to_value(first_result)
            .ok()
            .and_then(|v| v.get("text").cloned())
            .and_then(|v| v.as_str().map(String::from))
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .and_then(|v| serde_json::to_string_pretty(&v).ok())
            .unwrap_or_else(|| serde_json::to_string_pretty(first_result).unwrap_or_default()),
        None => String::new(),
    };
    RoundedBox::new(
        &result,
        Some("Tool Call Result"),
        Some(AnsiColor::BrightGreen),
        false,
    )
    .render()
}
