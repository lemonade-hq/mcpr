# MCP Web Scraper Search Example

This example demonstrates how to use the Model Context Protocol (MCP) to create a web scraper that searches for specific information in webpages using Claude.

## Overview

The example consists of:

1. **Server**: Scrapes webpages, saves the HTML to a temporary file, and uses Claude to search for specific information.
2. **Client**: Communicates with the server using the MCP protocol to request web scraping and information retrieval.

## Features

- Uses stdio transport for both the server and client
- Scrapes webpages and saves HTML to temporary files
- Uses Claude to analyze webpage content and extract specific information
- Supports both interactive and one-shot modes

## Requirements

- Rust and Cargo installed
- Anthropic API key (set as `ANTHROPIC_API_KEY` environment variable)
- Internet connection to scrape webpages

## How to Run

### Build the Example

```bash
cargo build --bin web-scraper-search-server --bin web-scraper-search-client
```

### Run the Server

```bash
cargo run --bin web-scraper-search-server
```

### Run the Client

#### Interactive Mode

```bash
cargo run --bin web-scraper-search-client --interactive
```

This will prompt you for a URI to scrape and a search term.

#### One-Shot Mode

```bash
cargo run --bin web-scraper-search-client --uri https://example.com --search-term "example"
```

## MCP Communication Flow

1. Client sends an initialization request to the server
2. Server responds with available tools (`web_scraper_search`)
3. Client sends a tool call with URI and search term
4. Server scrapes the webpage and saves it to a temporary file
5. Server uses Claude to search for the requested information
6. Server sends the results back to the client
7. Client displays the results

## Quick Test

You can use the provided test script to build and get instructions for running the example:

```bash
./test_web_scraper_search.sh
``` 