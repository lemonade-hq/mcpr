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
    echo -e "${RED}Error: ANTHROPIC_API_KEY environment variable is not set.${NC}"
    echo -e "${YELLOW}Please set it using: export ANTHROPIC_API_KEY=your_api_key${NC}"
    exit 1
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

# Run the MCP web scraper server
echo -e "${BLUE}Starting MCP web scraper server...${NC}"
echo -e "${YELLOW}The server will run in the background. Press Ctrl+C to stop.${NC}"

# Start the server and save its PID
cargo run --example mcp_web_scraper > "$LOGS_DIR/server.log" 2>&1 &
SERVER_PID=$!

# Wait a moment for the server to start
sleep 2

# Check if the server is running
if ! ps -p $SERVER_PID > /dev/null; then
    echo -e "${RED}Error: MCP web scraper server failed to start. Check the logs at $LOGS_DIR/server.log${NC}"
    exit 1
fi

echo -e "${GREEN}MCP web scraper server is running with PID $SERVER_PID${NC}"
echo -e "${CYAN}Server logs are being saved to $LOGS_DIR/server.log${NC}"

# Display instructions for using the MCP web scraper
echo -e "\n${BOLD}${BLUE}MCP Web Scraper Server Instructions${NC}${NC}"
echo -e "${CYAN}The MCP web scraper server is now running and ready to accept connections.${NC}"
echo -e "${CYAN}You can use the MCP client to connect to the server and use the web scraping tool.${NC}"

echo -e "\n${BOLD}Example MCP client usage:${NC}"
echo -e "${YELLOW}cargo run --example mcp_client -- --server-command \"cargo run --example mcp_web_scraper\"${NC}"

echo -e "\n${BOLD}Available tools:${NC}"
echo -e "${GREEN}web_scrape${NC} - Scrape a web page and extract content based on a query"
echo -e "  Parameters:"
echo -e "    - ${BOLD}url${NC}: The URL of the web page to scrape"
echo -e "    - ${BOLD}query${NC}: The query to extract specific content from the web page"
echo -e "    - ${BOLD}css_selector${NC}: (Optional) CSS selector to target specific elements"

echo -e "\n${YELLOW}Press Ctrl+C to stop the server and clean up temporary files.${NC}"

# Function to clean up on exit
cleanup() {
    echo -e "\n${YELLOW}Stopping MCP web scraper server...${NC}"
    kill $SERVER_PID 2>/dev/null
    
    echo -e "${YELLOW}Do you want to keep the temporary files? (y/n)${NC}"
    read -r keep_files
    
    if [[ "$keep_files" != "y" && "$keep_files" != "Y" ]]; then
        echo -e "${YELLOW}Removing temporary files...${NC}"
        rm -rf "$TEMP_DIR"
        echo -e "${GREEN}Temporary files removed.${NC}"
    else
        echo -e "${GREEN}Temporary files kept at: $TEMP_DIR${NC}"
    fi
    
    echo -e "${GREEN}MCP web scraper server stopped.${NC}"
    exit 0
}

# Set up trap to clean up on exit
trap cleanup INT TERM

# Wait for the server to finish (which it won't unless killed)
wait $SERVER_PID 