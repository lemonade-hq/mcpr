#!/bin/bash

# ANSI color codes for better terminal output
GREEN="\033[0;32m"
YELLOW="\033[1;33m"
BLUE="\033[0;34m"
RED="\033[0;31m"
BOLD="\033[1m"
NC="\033[0m" # No Color

# Check if Cargo is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Cargo is not installed. Please install Rust and Cargo first.${NC}"
    exit 1
fi

# Check if the server is running
echo -e "${BLUE}Checking if MCP web scraper socket server is running...${NC}"
if ! nc -z 127.0.0.1 8081 &> /dev/null; then
    echo -e "${YELLOW}Warning: MCP web scraper socket server does not appear to be running.${NC}"
    echo -e "${YELLOW}Please start the server first using:${NC}"
    echo -e "${YELLOW}    ./run_mcp_web_scraper_socket.sh${NC}"
    
    # Ask if the user wants to continue anyway
    read -p "Do you want to continue anyway? (y/n): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${RED}Exiting...${NC}"
        exit 1
    fi
else
    echo -e "${GREEN}MCP web scraper socket server is running.${NC}"
fi

# Build the example
echo -e "${BLUE}${BOLD}Building MCP web scraper client...${NC}"
cargo build --example mcp_web_scraper_client

if [ $? -ne 0 ]; then
    echo -e "${RED}Error: Failed to build example.${NC}"
    exit 1
fi

echo -e "${GREEN}Build successful!${NC}"

# Run the MCP web scraper client
echo -e "${BLUE}${BOLD}Running MCP web scraper client...${NC}"
echo -e "${YELLOW}This client will connect to the MCP web scraper socket server.${NC}"
cargo run --example mcp_web_scraper_client 