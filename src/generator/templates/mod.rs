//! Templates for generating MCP server and client stubs

pub mod sse;
pub mod stdio;
// WebSocket templates will be implemented in a future release

// Re-export all templates from transport modules
pub use sse::{
    PROJECT_CLIENT_CARGO_TEMPLATE as SSE_CLIENT_CARGO_TEMPLATE,
    PROJECT_CLIENT_TEMPLATE as SSE_CLIENT_TEMPLATE,
    PROJECT_SERVER_CARGO_TEMPLATE as SSE_SERVER_CARGO_TEMPLATE,
    PROJECT_SERVER_TEMPLATE as SSE_SERVER_TEMPLATE,
    PROJECT_TEST_SCRIPT_TEMPLATE as SSE_TEST_SCRIPT_TEMPLATE,
};

pub use stdio::{
    PROJECT_CLIENT_CARGO_TEMPLATE as STDIO_CLIENT_CARGO_TEMPLATE,
    PROJECT_CLIENT_TEMPLATE as STDIO_CLIENT_TEMPLATE,
    PROJECT_SERVER_CARGO_TEMPLATE as STDIO_SERVER_CARGO_TEMPLATE,
    PROJECT_SERVER_TEMPLATE as STDIO_SERVER_TEMPLATE,
    PROJECT_TEST_SCRIPT_TEMPLATE as STDIO_TEST_SCRIPT_TEMPLATE,
};

// WebSocket templates will be added when the WebSocket transport is implemented

// Original templates from templates.rs
/// Template for server main.rs
pub const SERVER_MAIN_TEMPLATE: &str = r#"//! MCP Server: {{name}}

use clap::Parser;
use mcpr::schema::{
    CallToolParams, CallToolResult, Implementation, InitializeResult, JSONRPCError, JSONRPCMessage,
    JSONRPCResponse, ServerCapabilities, TextContent, Tool, ToolInputSchema, ToolResultContent,
    ToolsCapability,
};
use mcpr::transport::stdio::StdioTransport;
use serde_json::{json, Value};
use std::error::Error;
use std::collections::HashMap;
use log::{info, error, debug, warn};

/// CLI arguments
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Enable debug output
    #[arg(short, long)]
    debug: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    
    if args.debug {
        println!("Debug mode enabled");
    }
    
    println!("Starting MCP server: {{name}}");
    
    // Create a transport for communication
    let mut transport = StdioTransport::new();
    
    // Wait for initialize request
    let message: JSONRPCMessage = transport.receive()?;
    
    // Server implementation here...
    
    Ok(())
}
"#;

/// Template for server Cargo.toml
pub const SERVER_CARGO_TEMPLATE: &str = r#"[package]
name = "{{name}}"
version = "0.1.0"
edition = "2021"
description = "MCP server generated using mcpr CLI"

[dependencies]
mcpr = "0.2.0"
clap = { version = "4.4", features = ["derive"] }
serde_json = "1.0"
log = "0.4"
env_logger = "0.10"
"#;

/// Template for client main.rs
pub const CLIENT_MAIN_TEMPLATE: &str = r#"//! MCP Client: {{name}}

use clap::Parser;
use mcpr::schema::{JSONRPCMessage, JSONRPCRequest, RequestId};
use mcpr::transport::stdio::StdioTransport;
use serde_json::Value;
use std::error::Error;
use std::io::{self, Write};
use std::process::{Command, Stdio};
use log::{info, error, debug, warn};

/// CLI arguments
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Enable debug output
    #[arg(short, long)]
    debug: bool,
    
    /// Server URI
    #[arg(short, long)]
    uri: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    
    if args.debug {
        println!("Debug mode enabled");
    }
    
    println!("Starting MCP client: {{name}}");
    println!("Connecting to server: {}", args.uri);
    
    // Client implementation here...
    
    Ok(())
}
"#;

/// Template for client Cargo.toml
pub const CLIENT_CARGO_TEMPLATE: &str = r#"[package]
name = "{{name}}"
version = "0.1.0"
edition = "2021"
description = "MCP client generated using mcpr CLI"

[dependencies]
mcpr = "0.2.0"
clap = { version = "4.4", features = ["derive"] }
serde_json = "1.0"
log = "0.4"
env_logger = "0.10"
"#;

// Common templates that are not transport-specific
pub const SERVER_README_TEMPLATE: &str = r#"# {{name}}

An MCP server generated using the mcpr CLI.

## Running the Server

```bash
cargo run
```

## Connecting to the Server

You can connect to this server using any MCP client. For example:

```bash
mcpr connect --uri stdio://./target/debug/{{name}}
```

## Available Tools

This server provides the following tools:

- `example`: A simple example tool that processes a query string
"#;

pub const CLIENT_README_TEMPLATE: &str = r#"# {{name}}

An MCP client generated using the mcpr CLI.

## Running the Client

```bash
cargo run -- --uri <server_uri>
```

For example, to connect to a local server:

```bash
cargo run -- --uri stdio://./path/to/server
```

## Usage

Once connected, you can enter queries that will be processed by the server's tools.
Type 'exit' to quit the client.
"#;

pub const PROJECT_README_SSE_TEMPLATE: &str = r#"# {{name}} - MCP Project

A complete MCP project with both client and server components, using sse transport.

This example demonstrates the use of the SSE transport implementation in MCPR, which has been enhanced to properly handle client-server communication with robust error handling and proper shutdown procedures.

## Project Structure

- `server/`: The MCP server implementation
- `client/`: The MCP client implementation
- `test.sh`: A test script to run both client and server

## Building the Project

```bash
# Build the server
cd server
cargo build

# Build the client
cd ../client
cargo build
```

## Running the Server

```bash
cd server
RUST_LOG=debug,mcpr=trace cargo run -- --port 8081
```

## Running the Client

```bash
cd client
RUST_LOG=debug,mcpr=trace cargo run -- --uri "http://localhost:8081" --name "Your Name"
```

## Running the Test Script

The test script will build and run both the server and client with debug logging enabled:

```bash
./test.sh
```

## Available Tools

This server provides the following tools:

- `hello`: A simple tool that greets a person by name

## SSE Transport Features

The SSE transport implementation in this example includes:

1. Robust client-server communication using HTTP
2. Proper handling of client registration and message polling
3. Graceful shutdown with proper cleanup of resources
4. Detailed logging for debugging
5. Error handling for network issues and JSON serialization/deserialization
"#;

pub const PROJECT_README_STDIO_TEMPLATE: &str = r#"# {{name}} - MCP Project

A complete MCP project with both client and server components, using stdio transport.

## Project Structure

- `server/`: The MCP server implementation
- `client/`: The MCP client implementation
- `test.sh`: A test script to run both client and server

## Building the Project

```bash
# Build the server
cd server
cargo build

# Build the client
cd ../client
cargo build
```

## Running the Server

```bash
cd server
cargo run
```

## Running the Client

```bash
cd client
cargo run -- --interactive
```

## Running the Test Script

```bash
./test.sh
```

## Available Tools

This server provides the following tools:

- `hello`: A simple tool that greets a person by name
"#;

// For templates that might be used in the future, we'll keep them but mark them with #[allow(dead_code)]
#[allow(dead_code)]
pub const PROJECT_README_WEBSOCKET_TEMPLATE: &str = r#"# {{name}} - MCP Project

A complete MCP project with both client and server components, using WebSocket transport (Coming Soon).

**Note: The WebSocket transport is currently under development and not yet available.**

This example will demonstrate the use of the WebSocket transport implementation in MCPR, which will provide real-time bidirectional communication between client and server with robust error handling and proper shutdown procedures.

## Project Structure

```
{{name}}/
├── client/             # Client implementation
│   ├── src/
│   │   └── main.rs     # Client code
│   └── Cargo.toml      # Client dependencies
├── server/             # Server implementation
│   ├── src/
│   │   └── main.rs     # Server code
│   └── Cargo.toml      # Server dependencies
└── test.sh             # Test script
```

## WebSocket Transport Features (Coming Soon)

The WebSocket transport implementation will include:
1. Full-duplex communication between client and server
2. Proper handling of WebSocket connections and message routing
3. Automatic reconnection handling
4. Error handling and logging
5. Support for all MCP message types

## Building and Running

**Note: The WebSocket transport is currently under development and not yet available.**

Once implemented, you will be able to:

### Build the Server

```bash
cd {{name}}/server
cargo build
```

### Build the Client

```bash
cd {{name}}/client
cargo build
```

### Run the Server

```bash
cd {{name}}/server
cargo run
```

### Run the Client

```bash
cd {{name}}/client
cargo run
```

### Run the Tests

```bash
cd {{name}}
./test.sh
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.
"#;

#[allow(dead_code)]
pub const PROJECT_README_TEMPLATE: &str = r#"# {{name}} - MCP Project

A complete MCP project with both client and server components, using {{transport_type}} transport.

{{#if_eq transport_type "sse"}}
This example demonstrates the use of the SSE transport implementation in MCPR, which has been enhanced to properly handle client-server communication with robust error handling and proper shutdown procedures.
{{/if_eq}}

{{#if_eq transport_type "websocket"}}
This example demonstrates the use of the WebSocket transport implementation in MCPR, which provides real-time bidirectional communication between client and server with robust error handling and proper shutdown procedures.
{{/if_eq}}

## Project Structure

- `server/`: The MCP server implementation
- `client/`: The MCP client implementation
- `test.sh`: A test script to run both client and server

## Building the Project

```bash
# Build the server
cd server
cargo build

# Build the client
cd ../client
cargo build
```

## Running the Server

```bash
cd server
{{#if_eq transport_type "sse"}}
RUST_LOG=debug,mcpr=trace cargo run -- --port 8081
{{/if_eq}}
{{#if_eq transport_type "websocket"}}
RUST_LOG=debug,mcpr=trace cargo run -- --port 8081
{{/if_eq}}
{{#if_eq transport_type "stdio"}}
cargo run
{{/if_eq}}
```

## Running the Client

```bash
cd client
{{#if_eq transport_type "sse"}}
RUST_LOG=debug,mcpr=trace cargo run -- --uri "http://localhost:8081" --name "Your Name"
{{/if_eq}}
{{#if_eq transport_type "websocket"}}
RUST_LOG=debug,mcpr=trace cargo run -- --uri "ws://localhost:8081" --name "Your Name"
{{/if_eq}}
{{#if_eq transport_type "stdio"}}
cargo run -- --interactive
{{/if_eq}}
```

## Running the Test Script

{{#if_eq transport_type "sse"}}
The test script will build and run both the server and client with debug logging enabled:
{{/if_eq}}
{{#if_eq transport_type "websocket"}}
The test script will build and run both the server and client with debug logging enabled:
{{/if_eq}}

```bash
./test.sh
```

## Available Tools

This server provides the following tools:

- `hello`: A simple tool that greets a person by name

{{#if_eq transport_type "sse"}}
## SSE Transport Features

The SSE transport implementation in this example includes:

1. Robust client-server communication using HTTP
2. Proper handling of client registration and message polling
3. Graceful shutdown with proper cleanup of resources
4. Detailed logging for debugging
5. Error handling for network issues and JSON serialization/deserialization
{{/if_eq}}

{{#if_eq transport_type "websocket"}}
## WebSocket Transport Features

The WebSocket transport implementation in this example includes:

1. Real-time bidirectional communication between client and server
2. Proper handling of WebSocket connections and message routing
3. Graceful shutdown with proper cleanup of resources
4. Detailed logging for debugging
5. Error handling for network issues and JSON serialization/deserialization
{{/if_eq}}
"#;

#[allow(dead_code)]
pub const PROJECT_RUN_TESTS_TEMPLATE: &str = r#"#!/bin/bash

# Run all tests for {{name}} MCP project

# Exit on error
set -e

echo "Running server tests..."
./test_server.sh

echo "Running client tests..."
./test_client.sh

echo "All tests completed successfully!"
"#;

#[allow(dead_code)]
pub const PROJECT_SERVER_TEST_TEMPLATE: &str = r#"#!/bin/bash

# Server tests for {{name}} MCP project

# Exit on error
set -e

echo "Building server..."
cd server
cargo build

echo "Running server tests..."
cargo test

echo "Server tests completed successfully!"
"#;

#[allow(dead_code)]
pub const PROJECT_CLIENT_TEST_TEMPLATE: &str = r#"#!/bin/bash

# Client tests for {{name}} MCP project

# Exit on error
set -e

echo "Building client..."
cd client
cargo build

echo "Running client tests..."
cargo test

echo "Client tests completed successfully!"
"#;
