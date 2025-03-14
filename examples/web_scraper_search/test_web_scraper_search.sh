#!/bin/bash

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== MCP Web Scraper Search Example ===${NC}"
echo -e "This example demonstrates using MCP to create a web scraper that searches for information in webpages using Claude."

# Check if Cargo is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${YELLOW}Cargo is not installed. Please install Rust and Cargo to run this example.${NC}"
    exit 1
fi

# Check if ANTHROPIC_API_KEY is set
if [ -z "$ANTHROPIC_API_KEY" ]; then
    echo -e "${YELLOW}ANTHROPIC_API_KEY environment variable is not set.${NC}"
    echo -e "Please set it with: export ANTHROPIC_API_KEY=your_api_key"
    exit 1
fi

echo -e "\n${GREEN}Building the example...${NC}"
cargo build --bin web-scraper-search-server --bin web-scraper-search-client

echo -e "\n${BLUE}=== How to Test This Example ===${NC}"
echo -e "This example uses the MCP protocol with SSE transport for the server and HTTP transport for the client."
echo -e "It demonstrates how to scrape a webpage and use Claude to search for specific information in the content."

echo -e "\n${YELLOW}Testing Instructions:${NC}"
echo -e "1. Open two terminal windows."
echo -e "2. In the first terminal, run the server:"
echo -e "   ${GREEN}cargo run --bin web-scraper-search-server${NC}"
echo -e "3. In the second terminal, run the client in interactive mode:"
echo -e "   ${GREEN}cargo run --bin web-scraper-search-client --interactive${NC}"
echo -e "   Or run a one-shot search:"
echo -e "   ${GREEN}cargo run --bin web-scraper-search-client --uri https://example.com --search-term \"example\"${NC}"

echo -e "\n${BLUE}=== MCP Communication Flow ===${NC}"
echo -e "1. Client sends an initialization request to the server"
echo -e "2. Server responds with available tools (web_scraper_search)"
echo -e "3. Client sends a tool call with URI and search term"
echo -e "4. Server scrapes the webpage and saves it to /tmp"
echo -e "5. Server uses Claude to search for the requested information"
echo -e "6. Server sends the results back to the client"
echo -e "7. Client displays the results"

echo -e "\n${YELLOW}Expected Behavior:${NC}"
echo -e "- The server will scrape the provided URI and save the HTML to a temporary file"
echo -e "- Claude will analyze the content and search for the requested information"
echo -e "- The client will display the search results or any errors that occurred"

echo -e "\n${BLUE}=== Requirements ===${NC}"
echo -e "- Anthropic API key (set as ANTHROPIC_API_KEY environment variable)"
echo -e "- Internet connection to scrape webpages"
echo -e "- Rust and Cargo installed"

echo -e "\n${GREEN}Example is ready to run!${NC}" 