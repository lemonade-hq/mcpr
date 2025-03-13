# Simple MCP Examples

This directory contains simple examples demonstrating the basic usage of the Model Context Protocol (MCP).

## Contents

- `server.rs`: A minimal MCP server implementation
- `client.rs`: A minimal MCP client implementation

## Running the Examples

### Server

To run the simple server:

```bash
cargo run --example simple_server
```

### Client

To run the simple client:

```bash
cargo run --example simple_client
```

## What These Examples Demonstrate

These examples demonstrate the basic communication pattern between an MCP client and server:

1. The client sends an initialize request to the server
2. The server responds with its capabilities
3. The client can then list available tools and call them
4. The server processes tool calls and returns results

These examples are intentionally minimal to help understand the core MCP protocol without additional complexity. 