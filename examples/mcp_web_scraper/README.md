# MCP Web Scraper Examples

This directory contains examples demonstrating how to implement a web scraper using the Model Context Protocol (MCP) with various communication methods and client interfaces.

## Contents

- `mcp_web_scraper.rs`: Basic MCP web scraper server using stdio for communication
- `test_mcp_web_scraper.rs`: Test client for the MCP web scraper
- `interactive_mcp_web_scraper.rs`: Interactive client for the MCP web scraper
- `interactive_mcp_client.rs`: Another interactive client implementation
- `mcp_web_scraper_socket.rs`: MCP web scraper server using TCP sockets for communication
- `mcp_web_scraper_client.rs`: Client for the socket-based MCP web scraper

## Running the Examples

### Basic MCP Web Scraper

To run the basic MCP web scraper:

```bash
./run_mcp_web_scraper.sh
```

### Socket-Based MCP Web Scraper

To run the socket-based MCP web scraper server:

```bash
./run_mcp_web_scraper_socket.sh
```

To run the socket-based MCP web scraper client:

```bash
./run_mcp_web_scraper_client.sh
```

### Interactive MCP Web Scraper

To run the interactive MCP web scraper:

```bash
./run_interactive_mcp_web_scraper.sh
```

## What These Examples Demonstrate

These examples demonstrate:

1. How to implement a web scraper using the MCP protocol
2. Different communication methods (stdio vs. TCP sockets)
3. Various client interfaces (test, interactive, socket-based)
4. How to use Claude to extract relevant information from web pages
5. How to handle complex MCP communication patterns
6. How to provide user-friendly interfaces for web scraping

The MCP web scraper examples show how to create a robust web scraping tool that leverages the MCP protocol for communication between client and server components. 