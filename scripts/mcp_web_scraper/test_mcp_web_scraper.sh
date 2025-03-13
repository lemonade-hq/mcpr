#!/bin/bash

# ANSI color codes for better terminal output
RED="\033[0;31m"
GREEN="\033[0;32m"
YELLOW="\033[1;33m"
BLUE="\033[0;34m"
CYAN="\033[0;36m"
BOLD="\033[1m"
NC="\033[0m" # No Color

# Check if Cargo is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Cargo is not installed. Please install Rust and Cargo first.${NC}"
    exit 1
fi

# Check if ANTHROPIC_API_KEY is set
if [ -z "$ANTHROPIC_API_KEY" ]; then
    echo -e "${YELLOW}ANTHROPIC_API_KEY not set, using dummy key for testing${NC}"
    export ANTHROPIC_API_KEY="dummy-key"
fi

# Create a temporary directory for HTML files
TEMP_DIR=$(mktemp -d)
if [[ ! "$TEMP_DIR" || ! -d "$TEMP_DIR" ]]; then
    echo -e "${RED}Error: Could not create temporary directory.${NC}"
    exit 1
fi

# Create HTML directory
HTML_DIR="$TEMP_DIR/html"
mkdir -p "$HTML_DIR"

# Create logs directory
LOGS_DIR="$TEMP_DIR/logs"
mkdir -p "$LOGS_DIR"

echo -e "${GREEN}Created temporary directory for HTML files: $TEMP_DIR${NC}"
echo -e "${GREEN}HTML files will be saved in: $HTML_DIR${NC}"
echo -e "${GREEN}Logs will be saved in: $LOGS_DIR${NC}"

# Export HTML_DIR for the server to use
export HTML_DIR="$HTML_DIR"

# Build the MCP web scraper server
echo -e "${BLUE}Building MCP web scraper server...${NC}"
cargo build --example mcp_web_scraper

# Check if build was successful
if [ $? -ne 0 ]; then
    echo -e "${RED}Error: Failed to build MCP web scraper server.${NC}"
    exit 1
fi

echo -e "${GREEN}Successfully built MCP web scraper server.${NC}"

# Build the MCP client
echo -e "${BLUE}Building MCP client...${NC}"
cargo build --example simple_client

# Check if build was successful
if [ $? -ne 0 ]; then
    echo -e "${RED}Error: Failed to build MCP client.${NC}"
    exit 1
fi

echo -e "${GREEN}Successfully built MCP client.${NC}"

# Run the MCP client with the server command
echo -e "${BLUE}Running MCP client with the server command...${NC}"
echo -e "${YELLOW}The client will start the server and connect to it.${NC}"

# Use the simple_client to start and connect to the server
cargo run --example simple_client -- --server-command "cargo run --example mcp_web_scraper"

# Check if the client was successful
if [ $? -ne 0 ]; then
    echo -e "${RED}Error: MCP client failed to connect to the server.${NC}"
    exit 1
fi

echo -e "${GREEN}MCP client successfully connected to the server.${NC}"

# Display instructions for using the MCP web scraper
echo -e "\n${BOLD}${BLUE}MCP Web Scraper Server Instructions${NC}${NC}"
echo -e "${CYAN}You can use the MCP client to connect to the server and use the web scraping tool.${NC}"

echo -e "\n${BOLD}Example MCP client usage:${NC}"
echo -e "${YELLOW}cargo run --example simple_client -- --server-command \"cargo run --example mcp_web_scraper\"${NC}"

echo -e "\n${BOLD}Available tools:${NC}"
echo -e "${GREEN}web_scrape${NC} - Scrape a web page and extract content based on a query"
echo -e "  Parameters:"
echo -e "    - ${BOLD}url${NC}: The URL of the web page to scrape"
echo -e "    - ${BOLD}query${NC}: The query to extract specific content from the web page"
echo -e "    - ${BOLD}css_selector${NC}: (Optional) CSS selector to target specific elements"

echo -e "\n${YELLOW}Do you want to keep the temporary files? (y/n)${NC}"
read -r keep_files

if [[ "$keep_files" != "y" && "$keep_files" != "Y" ]]; then
    echo -e "${YELLOW}Removing temporary files...${NC}"
    rm -rf "$TEMP_DIR"
    echo -e "${GREEN}Temporary files removed.${NC}"
else
    echo -e "${GREEN}Temporary files kept at: $TEMP_DIR${NC}"
fi

echo -e "${GREEN}Test completed.${NC}"
exit 0 