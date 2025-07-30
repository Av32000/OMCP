# OMCP - Ollama Model Context Protocol Client

A powerful CLI client that connects [Ollama](https://ollama.com/) language models to [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) servers, enabling LLMs to access external tools and data sources locally.

![OMCP Demo](assets/omcp.gif)

## 🚀 Features

- **Multiple MCP Server Support**: Connect to MCP servers via STDIO, SSE, and Streamable HTTP transports
- **Interactive Chat Interface**: Terminal-based chat with real-time streaming responses
- **Tool Integration**: Automatic discovery and execution of MCP server tools
- **Model Management**: Built-in Ollama model selection, loading, and pulling
- **Configurable Settings**: Persistent configuration with JSON-based settings
- **Batch Mode**: Execute single prompts without entering interactive mode
- **Tool Control**: Enable/disable tools dynamically during conversations
- **Thinking Mode**: Display model reasoning process (for supported models)

## 📦 Installation

### From Arch User Repository (AUR)

OMCP is available in the AUR as `omcp-git`. You can install it using an AUR helper like `yay`:

```bash
yay -S omcp-git
```

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable version)
- [Ollama](https://ollama.com/) installed and running
- MCP servers you want to connect to

### Build from Source

```bash
git clone https://github.com/Av32000/omcp.git
cd omcp
cargo build --release
cargo install --path .
```

The compiled binary will be available at `target/release/omcp` and installed into your PATH.

## 🔧 Usage

### Basic Usage

Start OMCP with default settings:
```bash
omcp
```

### Command Line Options

```bash
omcp [OPTIONS]

Options:
  -s, --stdio-server <PATH>           Path to a Python or JavaScript file for a stdio MCP server (require node or python)
  -S, --sse-server <URL>              URL for an SSE (Server-Sent Events) MCP server
  -H, --streamable-http-server <URL>  URL for a streamable HTTP MCP server
  -j, --json-mcp-config <PATH>        Path to a JSON configuration file containing MCP server definitions
  -m, --model <MODEL>                 Specify the default Ollama model to use
  -c, --config <PATH>                 Path to a custom JSON configuration file
  -o, --ollama-host <URL>             Specify the Ollama host URL (e.g., http://localhost:11434)
  -p, --prompt <TEXT>                 Execute a prompt immediately and return the result
  -h, --help                          Print help
  -V, --version                       Print version
```

### Examples

#### Connect to a Python MCP Server
```bash
omcp -s ~/mcp-servers/filesystem.py
```

#### Connect to Multiple Servers
```bash
omcp -s ~/servers/filesystem.py -s ~/servers/database.py -S http://localhost:8080/mcp
```

#### Use Custom Model and Configuration
```bash
omcp -m llama3.1:8b -c ~/my-omcp-config.json
```

#### Batch Mode (Non-Interactive)
```bash
omcp -p "List the files in the current directory" -s ~/mcp-servers/filesystem.py
```

## ⚙️ Configuration

OMCP uses JSON configuration files stored in your system's config directory (`~/.config/omcp/` on Linux/macOS).

### Settings Configuration (`settings.json`)

```json
{
  "model_name": "qwen2.5:7b",
  "show_thinking": true,
  "model_seed": 0,
  "model_temperature": 0.8,
  "model_system_prompt": "",
  "verbose_tool_calls": true,
  "tool_confirmation": true,
  "auto_save_config": true,
  "config_file_path": "~/.config/omcp/settings.json"
}
```

### MCP Servers Configuration (`mcp_servers.json`)

```json
{
  "mcpServers": {
    "time": {
      "command": "uvx",
      "args": ["mcp-server-time"],
      "disabled": false
    },
    "web-search": {
      "type": "sse",
      "url": "http://localhost:8080/mcp",
      "headers": {
        "Authorization": "Bearer your-token"
      },
      "disabled": false
    },
    "database": {
      "type": "streamable_http",
      "url": "http://localhost:9000/mcp",
      "disabled": false
    }
  }
}
```

## 🎮 Interactive Commands

While in interactive mode, you can use the following commands:

- `/quit` - Exit the application
- `/clear` - Clear the chat context
- `/history` - Show chat history
- `/tools show` - List all available tools
- `/tools toggle` - Enable/disable specific tools
- `/settings show` - Display current settings
- `/settings edit` - Edit configuration interactively
- `/model info` - Show current model information
- `/model select` - Choose a different model
- `/model load` - Load the current model into memory
- `/model pull` - Download/update the current model
- `/help` - Show all available commands

## 🔌 MCP Server Types

OMCP supports three types of MCP server connections:

### STDIO Servers
- **Use case**: Local Python/JavaScript MCP servers
- **Example**: File system tools, local databases
- **Configuration**: Specify the command and arguments to run the server

### SSE (Server-Sent Events) Servers
- **Use case**: Remote servers that support streaming
- **Example**: Web APIs, cloud services
- **Configuration**: Provide the SSE endpoint URL and optional headers

### Streamable HTTP Servers
- **Use case**: HTTP-based MCP servers
- **Example**: REST API wrappers, microservices
- **Configuration**: Specify the base URL and optional headers

## 🛠️ Development

### Project Structure

```
src/
├── args.rs          # Command line argument parsing
├── chat.rs          # Ollama chat integration and streaming
├── main.rs          # Application entry point
├── model.rs         # Model selection and management
├── settings.rs      # Configuration management
├── tools/
│   ├── mod.rs       # Tool manager and MCP server loading
│   ├── server.rs    # MCP server connection
│   └── tool.rs      # Tool definitions and conversion
└── ui/
    ├── input.rs     # User input handling
    ├── mod.rs       # Main UI logic and command parsing
    ├── tools.rs     # Tool-related UI rendering
    └── utils.rs     # UI utilities and styling
```

### Key Dependencies

- **[ollama-rs](https://crates.io/crates/ollama-rs)**: Ollama API client
- **[rmcp](https://crates.io/crates/rmcp)**: Model Context Protocol implementation
- **[clap](https://crates.io/crates/clap)**: Command line argument parsing
- **[tokio](https://crates.io/crates/tokio)**: Async runtime
- **[serde](https://crates.io/crates/serde)**: Serialization framework

### Building

```bash
# Development build
cargo build

# Release build
cargo build --release
```

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Ollama](https://ollama.com/) for providing the local LLM infrastructure
- [Model Context Protocol](https://modelcontextprotocol.io/) for the standardized tool integration protocol
- The Rust community for crates and documentation

## 📞 Support

If you encounter any issues or have questions:

1. Check the [Issues](https://github.com/Av32000/omcp/issues) section
2. Create a new issue with a detailed description
3. Include your configuration and error logs if applicable

---

**Note**: This project is in active development. Features may change between versions.
