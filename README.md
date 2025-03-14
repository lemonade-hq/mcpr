# MCPR - Model Context Protocol for Rust

A Rust implementation of Anthropic's [Model Context Protocol (MCP)](https://docs.anthropic.com/claude/docs/model-context-protocol), an open standard for connecting AI assistants to data sources and tools.

## Features

- **Schema Definitions**: Complete implementation of the MCP schema
- **Transport Layer**: Multiple transport options including stdio, SSE, and WebSocket
- **High-Level Client/Server**: Easy-to-use client and server implementations
- **CLI Tools**: Generate server and client stubs
- **Examples**: Practical examples demonstrating MCP usage

## Installation

Add MCPR to your `Cargo.toml`:

```toml
[dependencies]
mcpr = "0.1.0"
```

## Usage

### High-Level Client

The high-level client provides a simple interface for communicating with MCP servers:

```rust
use mcpr::{
    client::Client,
    transport::stdio::StdioTransport,
};

// Create a client with stdio transport
let transport = StdioTransport::new();
let mut client = Client::new(transport);

// Initialize the client
client.initialize()?;

// Call a tool
let request = MyToolRequest { /* ... */ };
let response: MyToolResponse = client.call_tool("my_tool", &request)?;

// Shutdown the client
client.shutdown()?;
```

### High-Level Server

The high-level server makes it easy to create MCP-compatible servers:

```rust
use mcpr::{
    server::{Server, ServerConfig},
    transport::stdio::StdioTransport,
    Tool,
};

// Configure the server
let server_config = ServerConfig::new()
    .with_name("My MCP Server")
    .with_version("1.0.0")
    .with_tool(Tool {
        name: "my_tool".to_string(),
        description: "My awesome tool".to_string(),
        parameters_schema: serde_json::json!({
            "type": "object",
            "properties": {
                // Tool parameters schema
            },
            "required": ["param1", "param2"]
        }),
    });

// Create the server
let mut server = Server::new(server_config);

// Register tool handlers
server.register_tool_handler("my_tool", |params| {
    // Parse parameters and handle the tool call
    // ...
    Ok(serde_json::to_value(response)?)
})?;

// Start the server with stdio transport
let transport = StdioTransport::new();
server.start(transport)?;
```

## Examples

The repository includes several examples demonstrating how to use MCPR:

- **Web Scraper Search**: Scrapes a webpage and uses Claude to search for specific information
- **Simple**: A basic example demonstrating MCP communication
- **Tool Caller**: An example showing how to call tools via MCP

To run an example:

```bash
cd examples/web_scraper_search
cargo run --bin web-scraper-search-server
```

In another terminal:

```bash
cd examples/web_scraper_search
cargo run --bin web-scraper-search-client --interactive
```

## Contributing

Contributions are welcome! Here are some ways you can contribute:

- Implement additional transport options
- Add more examples
- Improve documentation
- Fix bugs and add features

## License

This project is licensed under the MIT License - see the LICENSE file for details. 