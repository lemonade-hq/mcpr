# Web Scrape MCP Examples

This directory contains examples demonstrating how to implement a web scraping tool using the Model Context Protocol (MCP).

## Contents

- `server.rs`: An MCP server that provides a web scraping tool
- `client.rs`: An MCP client that can interact with the web scraping server

## Running the Examples

### Server

To run the web scrape server:

```bash
cargo run --example web_scrape_server
```

Or use the provided script:

```bash
./run_web_scrape.sh
```

### Client

To run the web scrape client:

```bash
cargo run --example web_scrape_client
```

## What These Examples Demonstrate

These examples demonstrate:

1. How to implement a web scraping tool using MCP
2. How to handle HTML content processing in an MCP server
3. How to extract relevant information from web pages
4. How to provide a user-friendly interface for web scraping

The web scrape examples show how MCP can be used to create tools that interact with web content, allowing users to extract specific information from websites without having to manually navigate and parse HTML. 