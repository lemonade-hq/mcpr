//! Templates for generating MCP server and client stubs

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
description = "MCP server generated from mcpr template"

[dependencies]
mcpr = "0.1.0"
clap = { version = "4.4", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
"#;

/// Template for server README.md
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

/// Template for client main.rs
pub const CLIENT_MAIN_TEMPLATE: &str = r#"//! MCP Client: {{name}}

use clap::Parser;
use mcpr::schema::{
    CallToolParams, CallToolResult, ClientCapabilities, Implementation, InitializeParams,
    JSONRPCMessage, JSONRPCRequest, RequestId, TextContent, ToolResultContent,
};
use mcpr::transport::stdio::StdioTransport;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::error::Error;
use std::io::{self, Write};

/// CLI arguments
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// URI of the server to connect to
    #[arg(short, long)]
    uri: String,

    /// Enable debug output
    #[arg(short, long)]
    debug: bool,
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
description = "MCP client generated from mcpr template"

[dependencies]
mcpr = "0.1.0"
clap = { version = "4.4", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
"#;

/// Template for client README.md
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
