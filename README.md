# Model Context Protocol (MCP) for Rust

A Rust implementation of [Anthropic's Model Context Protocol (MCP)](https://www.anthropic.com/news/model-context-protocol), an open standard for connecting AI assistants to data sources and tools.

## Overview

This project provides:

1. A complete Rust implementation of the MCP schema
2. Tools for generating server and client stubs
3. Transport layer implementations for different communication methods
4. CLI utilities for working with MCP
5. Comprehensive examples demonstrating various MCP use cases

## Installation

```bash
# Install from crates.io
cargo install mcpr

# Or build from source
git clone https://github.com/conikeec/mcpr.git
cd mcpr
cargo install --path .
```

## Usage

Once installed, you can use the `mcpr` command-line tool to generate MCP server and client stubs, run servers, connect to servers, and validate MCP messages.

### Generate a Server Stub

```bash
mcpr generate-server --name my-server --output ./my-server
```

This will create a new directory `./my-server/my-server` with a basic MCP server implementation that you can customize.

### Generate a Client Stub

```bash
mcpr generate-client --name my-client --output ./my-client
```

This will create a new directory `./my-client/my-client` with a basic MCP client implementation that you can customize.

### Run a Server

```bash
mcpr run-server --path ./my-server
```

### Connect to a Server

```bash
mcpr connect --uri stdio://./my-server
```

### Validate an MCP Message

```bash
mcpr validate --path ./message.json
```

## Library Usage

```rust
use mcpr::schema::{JSONRPCMessage, InitializeRequest, ClientCapabilities};
use mcpr::transport::stdio::StdioTransport;

// Create a client
let mut client = StdioTransport::new();

// Send an initialize request
let request = InitializeRequest {
    method: "initialize".to_string(),
    params: InitializeParams {
        protocol_version: "2024-11-05".to_string(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "my-client".to_string(),
            version: "0.1.0".to_string(),
        },
    },
};

// Send the request
client.send(&request).await?;

// Receive the response
let response = client.receive().await?;
```

## Examples

This repository includes several examples demonstrating different aspects of the MCP protocol:

### Simple Examples

Basic examples showing the core MCP protocol functionality:

```bash
# Run the simple server
cargo run --example simple_server

# Run the simple client
cargo run --example simple_client
```

See [examples/simple/README.md](examples/simple/README.md) for more details.

### Browser Search

Examples demonstrating a browser search tool using MCP:

```bash
# Run the browser search server
cd examples/browser_search
./run_browser_search.sh

# Run the browser search client
cargo run --example browser_search_client
```

See [examples/browser_search/README.md](examples/browser_search/README.md) for more details.

### Tool Caller

Examples showing how to implement tool calling functionality:

```bash
# Run the LLM tool caller
cd examples/tool_caller
./run_llm_tool_caller.sh

# Run the standalone tool caller
cd examples/tool_caller
./run_standalone_tool_caller.sh
```

See [examples/tool_caller/README.md](examples/tool_caller/README.md) for more details.

### Web Scrape

Examples demonstrating a web scraping tool using MCP:

```bash
# Run the web scrape server
cd examples/web_scrape
./run_web_scrape.sh

# Run the web scrape client
cargo run --example web_scrape_client
```

See [examples/web_scrape/README.md](examples/web_scrape/README.md) for more details.

### Direct Web Scraper

A standalone web scraper that doesn't use MCP:

```bash
# Run the direct web scraper
cd examples/direct_web_scraper
./run_direct_web_scraper.sh
```

See [examples/direct_web_scraper/README.md](examples/direct_web_scraper/README.md) for more details.

### MCP Web Scraper

Advanced examples showing different ways to implement a web scraper using MCP:

```bash
# Run the socket-based MCP web scraper server
cd examples/mcp_web_scraper
./run_mcp_web_scraper_socket.sh

# Run the socket-based MCP web scraper client
cd examples/mcp_web_scraper
./run_mcp_web_scraper_client.sh

# Run the interactive MCP web scraper
cd examples/mcp_web_scraper
./run_interactive_mcp_web_scraper.sh
```

See [examples/mcp_web_scraper/README.md](examples/mcp_web_scraper/README.md) for more details.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgements

- [Anthropic](https://www.anthropic.com/) for creating the Model Context Protocol
- [Model Context Protocol Specification](https://github.com/modelcontextprotocol/specification) 