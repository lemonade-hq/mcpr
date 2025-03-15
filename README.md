# MCPR - Model Context Protocol for Rust

A Rust implementation of Anthropic's [Model Context Protocol (MCP)](https://docs.anthropic.com/claude/docs/model-context-protocol), an open standard for connecting AI assistants to data sources and tools.

> **⚠️ IMPORTANT NOTICE**: Version 0.2.0 has been yanked due to critical issues with the SSE transport implementation. Please use version 0.2.2 instead, which includes important fixes for client-server communication, message handling, and template generation. If you're already using 0.2.0, we strongly recommend upgrading to 0.2.2 to avoid potential issues.

## Examples

[![asciicast](https://asciinema.org/a/708211.svg)](https://asciinema.org/a/708211)

Check out our [GitHub Tools example](examples/github-tools/README.md) for a complete implementation of an MCP client-server application that interacts with GitHub repositories. This example demonstrates how to build a client that can query repository READMEs and search for repositories, with support for multiple servers and client-server disconnection scenarios. It's a great starting point for understanding how to build your own MCP applications.

## Features

- **Schema Definitions**: Complete implementation of the MCP schema
- **Transport Layer**: Multiple transport options including stdio and SSE
- **High-Level Client/Server**: Easy-to-use client and server implementations
- **CLI Tools**: Generate server and client stubs
- **Project Generator**: Quickly scaffold new MCP projects
- **Mock Implementations**: Built-in mock transports for testing and development

## Coming Soon

- **WebSocket Transport**: WebSocket transport implementation is planned but not yet implemented

## Installation

Add MCPR to your `Cargo.toml`:

```toml
[dependencies]
mcpr = "0.2.2"  # Make sure to use 0.2.2 or later, as 0.2.0 has been yanked
```

For CLI tools, install globally:

```bash
cargo install mcpr
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

## Creating MCP Projects

MCPR includes a project generator to quickly scaffold new MCP projects with different transport types.

### Using the CLI

```bash
# Generate a project with stdio transport
mcpr generate-project --name my-stdio-project --transport stdio

# Generate a project with SSE transport
mcpr generate-project --name my-sse-project --transport sse
```

### Project Structure

Each generated project includes:

```
my-project/
├── client/             # Client implementation
│   ├── src/
│   │   └── main.rs     # Client code
│   └── Cargo.toml      # Client dependencies
├── server/             # Server implementation
│   ├── src/
│   │   └── main.rs     # Server code
│   └── Cargo.toml      # Server dependencies
├── test.sh             # Combined test script
├── test_server.sh      # Server-only test script
├── test_client.sh      # Client-only test script
└── run_tests.sh        # Script to run all tests
```

### Building Projects

```bash
# Build the server
cd my-project/server
cargo build

# Build the client
cd my-project/client
cargo build
```

### Running Projects

#### Stdio Transport

For stdio transport, you typically run the server and pipe its output to the client:

```bash
# Run the server and pipe to client
./server/target/debug/my-stdio-project-server | ./client/target/debug/my-stdio-project-client
```

Or use the client to connect to the server:

```bash
# Run the server in one terminal
./server/target/debug/my-stdio-project-server

# Run the client in another terminal
./client/target/debug/my-stdio-project-client --uri "stdio://./server/target/debug/my-stdio-project-server"
```

#### SSE Transport

For SSE transport, you run the server first, then connect with the client. The generated project includes a mock SSE transport implementation for testing:

```bash
# Run the server (default port is 8080)
./server/target/debug/my-sse-project-server --port 8080

# In another terminal, run the client
./client/target/debug/my-sse-project-client --uri "http://localhost:8080"
```

The SSE transport supports both interactive and one-shot modes:

```bash
# Interactive mode
./client/target/debug/my-sse-project-client --interactive

# One-shot mode
./client/target/debug/my-sse-project-client --name "Your Name"
```

The mock SSE transport implementation includes:
- Automatic response generation for initialization
- Echo-back functionality for tool calls
- Proper error handling and logging
- Support for all MCP message types

#### Interactive Mode

Clients support an interactive mode for manual testing:

```bash
./client/target/debug/my-project-client --interactive
```

### Running Tests

Each generated project includes test scripts:

```bash
# Run all tests
./run_tests.sh

# Run only server tests
./test_server.sh

# Run only client tests
./test_client.sh

# Run the combined test (original test script)
./test.sh
```

## Transport Options

MCPR supports multiple transport options:

### Stdio Transport

The simplest transport, using standard input/output:

```rust
use mcpr::transport::stdio::StdioTransport;

let transport = StdioTransport::new();
```

### SSE Transport

Server-Sent Events transport for web-based applications:

```rust
use mcpr::transport::sse::SSETransport;

// For server
let transport = SSETransport::new("http://localhost:8080");

// For client
let transport = SSETransport::new("http://localhost:8080");
```

### WebSocket Transport (Coming Soon)

WebSocket transport for bidirectional communication is currently under development.

## Detailed Testing Guide

This section provides comprehensive instructions for generating and testing projects with both stdio and SSE transports.

### Generating Projects

When generating projects, make sure to specify the correct transport type and output directory:

```bash
# Generate a stdio project
mcpr generate-project --name test-stdio-project --transport stdio --output /tmp

# Generate an SSE project
mcpr generate-project --name test-sse-project --transport sse --output /tmp
```

Note: The `--output` parameter specifies where to create the project directory. If omitted, the project will be created in the current directory.

### Testing Stdio Transport Projects

1. **Build the project**:
   ```bash
   cd /tmp/test-stdio-project
   cd server && cargo build
   cd ../client && cargo build
   ```

2. **Run the server and client together**:
   ```bash
   cd /tmp/test-stdio-project
   ./server/target/debug/test-stdio-project-server | ./client/target/debug/test-stdio-project-client
   ```

   You should see output similar to:
   ```
   [INFO] Using stdio transport
   [INFO] Initializing client...
   [INFO] Server info: {"protocol_version":"2024-11-05","server_info":{"name":"test-stdio-project-server","version":"1.0.0"},"tools":[{"description":"A simple hello world tool","input_schema":{"properties":{"name":{"description":"Name to greet","type":"string"}},"required":["name"],"type":"object"},"name":"hello"}]}
   [INFO] Running in one-shot mode with name: Default User
   [INFO] Calling tool 'hello' with parameters: {"name":"Default User"}
   [INFO] Received message: Hello, Default User!
   Hello, Default User!
   [INFO] Shutting down client
   [INFO] Client shutdown complete
   ```

3. **Run with detailed logging**:
   ```bash
   RUST_LOG=debug ./server/target/debug/test-stdio-project-server | RUST_LOG=debug ./client/target/debug/test-stdio-project-client
   ```

4. **Run with a custom name**:
   ```bash
   ./server/target/debug/test-stdio-project-server | ./client/target/debug/test-stdio-project-client --name "Your Name"
   ```

### Testing SSE Transport Projects

1. **Build the project**:
   ```bash
   cd /tmp/test-sse-project
   cd server && cargo build
   cd ../client && cargo build
   ```

2. **Run the server**:
   ```bash
   cd /tmp/test-sse-project/server
   RUST_LOG=trace cargo run -- --port 8084 --debug
   ```

3. **In another terminal, run the client**:
   ```bash
   cd /tmp/test-sse-project/client
   RUST_LOG=trace cargo run -- --uri "http://localhost:8084" --name "Test User"
   ```

   You should see output similar to:
   ```
   [INFO] Using SSE transport with URI: http://localhost:8084
   [INFO] Initializing client...
   [INFO] Server info: {"protocol_version":"2024-11-05","server_info":{"name":"test-sse-project-server","version":"1.0.0"},"tools":[{"description":"A simple hello world tool","input_schema":{"properties":{"name":{"description":"Name to greet","type":"string"}},"required":["name"],"type":"object"},"name":"hello"}]}
   [INFO] Running in one-shot mode with name: Test User
   [INFO] Calling tool 'hello' with parameters: {"name":"Test User"}
   [INFO] Received message: Hello, Test User!
   Hello, Test User!
   [INFO] Shutting down client
   [INFO] Client shutdown complete
   ```

### Troubleshooting

#### Common Issues with Stdio Transport

1. **Pipe Connection Issues**:
   - Ensure that the server output is properly piped to the client input
   - Check for any terminal configuration that might interfere with piping

2. **Process Termination**:
   - The server process will terminate after the client disconnects
   - For long-running sessions, consider using the interactive mode

#### Common Issues with SSE Transport

1. **Dependency Issues**:
   
   If you encounter dependency errors when building generated projects, you may need to update the `Cargo.toml` files to point to your local MCPR crate:

   ```toml
   # For local development, use path dependency:
   mcpr = { path = "/path/to/your/mcpr" }
   ```

2. **Port Already in Use**:
   
   If the SSE server fails to start with a "port already in use" error, try a different port:

   ```bash
   ./server/target/debug/test-sse-project-server --port 8085
   ```

3. **Connection Refused**:
   
   If the client cannot connect to the server, ensure the server is running and the port is correct:

   ```bash
   # Check if the server is listening on the port
   netstat -an | grep 8084
   ```

4. **HTTP Method Not Allowed (405)**:
   
   If you see HTTP 405 errors, ensure that the server is correctly handling all required HTTP methods (GET and POST) for the SSE transport.

5. **Client Registration Issues**:
   
   The SSE transport requires client registration before message exchange. Ensure that:
   - The client successfully registers with the server
   - The client ID is properly passed in polling requests
   - The server maintains the client connection state

### Interactive Testing

Both transport types support interactive mode for manual testing:

```bash
# For stdio transport
./client/target/debug/test-stdio-project-client --interactive

# For SSE transport
./client/target/debug/test-sse-project-client --uri "http://localhost:8084" --interactive
```

In interactive mode, you can:
- Enter tool names and parameters manually
- Test different parameter combinations
- Observe the server's responses in real-time

### Advanced Testing

For more advanced testing scenarios:

1. **Testing with Multiple Clients**:
   
   The SSE transport supports multiple concurrent clients:
   ```bash
   # Start multiple client instances in different terminals
   ./client/target/debug/test-sse-project-client --uri "http://localhost:8084" --name "User 1"
   ./client/target/debug/test-sse-project-client --uri "http://localhost:8084" --name "User 2"
   ```

2. **Testing Error Handling**:
   
   Test how the system handles errors by sending invalid requests:
   ```bash
   # In interactive mode, try calling a non-existent tool
   > call nonexistent_tool {"param": "value"}
   ```

3. **Performance Testing**:
   
   For performance testing, you can use tools like Apache Bench or wrk to simulate multiple concurrent clients.

## Debugging

Enable debug logging for detailed information:

```bash
# Set environment variable for debug logging
RUST_LOG=debug cargo run
```

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details. 