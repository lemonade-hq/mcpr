# Direct Web Scraper Example

This directory contains a standalone web scraper that doesn't use the MCP protocol but demonstrates similar functionality.

## Contents

- `direct_web_scraper.rs`: A standalone web scraper that uses Claude to extract information from web pages

## Running the Example

To run the direct web scraper:

```bash
cargo run --example direct_web_scraper
```

Or use the provided script:

```bash
./run_direct_web_scraper.sh
```

## What This Example Demonstrates

This example demonstrates:

1. How to implement a web scraper without the MCP protocol
2. How to use Claude to extract relevant information from web pages
3. How to handle HTML content processing and extraction
4. How to provide a simple command-line interface for web scraping

The direct web scraper is a simpler alternative to the MCP web scraper examples, showing how similar functionality can be implemented without the MCP protocol. It's useful for understanding the core web scraping functionality before adding the complexity of MCP. 