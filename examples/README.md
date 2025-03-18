# MCP (Model Context Protocol) Examples

This directory contains examples demonstrating the usage of the MCP library with different transport methods and patterns.

## Overview

These examples demonstrate:

- Creating servers that handle tool calls
- Creating clients that connect to servers
- Using different transport methods (stdio, WebSocket)
- Handling errors gracefully
- Making concurrent tool calls

## Async Examples

### Echo Server (stdio)

A simple server that listens on standard input/output and registers two tools:

- `echo`: Echoes back the input message
- `hello`: Says hello to a specified name

To run:

```bash
cargo run --example echo_server
```

### Interactive Client (stdio)

A client that connects to a server over standard input/output, retrieves available tools, and allows interactively calling tools in a command-line interface.

To run (in a separate terminal after starting the server):

```bash
cargo run --example interactive_client
```

Usage:

- Type commands in the format: `<tool_name> <json_params>`
- Example: `echo {"message": "Hello, world!"}`
- Type `exit` to quit

### WebSocket Server

A server that listens for WebSocket connections on localhost:8080 and provides an echo tool.

To run:

```bash
cargo run --example websocket_server
```

This server runs until you press Ctrl+C to exit.

### Concurrent Client (WebSocket)

A client that demonstrates making multiple tool calls concurrently to a WebSocket server.

To run (after starting the WebSocket server):

```bash
cargo run --example concurrent_client
```

This example spawns multiple tasks, each with its own transport connection, to make concurrent requests to the server.

## Testing the Examples

To test these examples, you can run them in pairs:

### Testing stdio transport:

1. Terminal 1: `cargo run --example echo_server`
2. Terminal 2: `cargo run --example interactive_client`

### Testing WebSocket transport:

1. Terminal 1: `cargo run --example websocket_server`
2. Terminal 2: `cargo run --example concurrent_client`

## Notes

- These examples use `env_logger` for logging. Set the `RUST_LOG` environment variable to control log levels.
  Example: `RUST_LOG=info cargo run --example echo_server`

- The WebSocket examples require a network connection, but only communicate locally (127.0.0.1).

- Error handling is demonstrated in all examples, showing how to properly propagate and handle errors in an async context.
