use clap::Parser;

/// OMCP - A Model Context Protocol client for interacting with various MCP servers
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to a .py or .js file for a stdio MCP server
    #[arg(
        short = 's',
        long = "stdio-server",
        help = "Path to a Python or JavaScript file for a stdio MCP server"
    )]
    pub stdio_server: Vec<String>,

    /// URL for an SSE (Server-Sent Events) MCP server
    #[arg(
        short = 'S',
        long = "sse-server",
        help = "URL for an SSE (Server-Sent Events) MCP server"
    )]
    pub sse_server: Vec<String>,

    /// URL for a streamable HTTP MCP server
    #[arg(
        short = 'H',
        long = "streamable-http-server",
        help = "URL for a streamable HTTP MCP server"
    )]
    pub streamable_http_server: Vec<String>,

    /// Path to a .json file containing MCP servers config
    #[arg(
        short = 'j',
        long = "json-mcp-config",
        help = "Path to a JSON configuration file containing MCP server definitions"
    )]
    pub json_mcp_config: Vec<String>,

    /// Ollama model to use by default
    #[arg(
        short = 'm',
        long = "model",
        help = "Specify the default Ollama model to use"
    )]
    pub model: Option<String>,

    /// Path to a .json config file overwriting the default one
    #[arg(
        short = 'c',
        long = "config",
        help = "Path to a custom JSON configuration file that overrides default settings"
    )]
    pub config: Option<String>,

    /// Ollama host URL
    #[arg(
        short = 'o',
        long = "ollama-host",
        help = "Specify the Ollama host URL (e.g., http://localhost:11434)"
    )]
    pub ollama_host: Option<String>,

    /// Start instantly with the provided prompt and return the result
    #[arg(
        short = 'p',
        long = "prompt",
        help = "Execute a prompt immediately and return the result without entering interactive mode"
    )]
    pub prompt: Option<String>,
}

impl Args {
    /// Parse command line arguments
    pub fn parse() -> Self {
        Parser::parse()
    }
}
