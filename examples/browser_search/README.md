# Browser Search MCP Examples

This directory contains examples demonstrating how to implement a browser search tool using the Model Context Protocol (MCP).

## Contents

- `server.rs`: An MCP server that provides a browser search tool
- `client.rs`: An MCP client that can interact with the browser search server

## Running the Examples

### Server

To run the browser search server:

```bash
cargo run --example browser_search_server
```

Or use the provided script:

```bash
./run_browser_search.sh
```

### Client

To run the browser search client:

```bash
cargo run --example browser_search_client
```

## What These Examples Demonstrate

These examples demonstrate:

1. How to implement a more complex MCP tool (browser search)
2. How to handle user input in an MCP client
3. How to process and return structured data from an MCP server
4. How to implement proper error handling in MCP communication

The browser search tool allows users to search the web directly from the command line, with results processed and formatted for easy consumption. 