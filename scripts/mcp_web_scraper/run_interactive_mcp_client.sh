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

# Check if ANTHROPIC_API_KEY is set
if [[ -z "${ANTHROPIC_API_KEY}" ]]; then
    echo -e "${YELLOW}Warning: ANTHROPIC_API_KEY environment variable is not set.${NC}"
    echo -e "${YELLOW}The web scraper requires an Anthropic API key to extract content.${NC}"
    echo -e "${YELLOW}Please set it using: export ANTHROPIC_API_KEY=your_api_key${NC}"
    
    # Ask if the user wants to continue without the API key
    read -p "Do you want to continue anyway? (y/n): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${RED}Exiting...${NC}"
        exit 1
    fi
    
    # Set a dummy key for testing
    export ANTHROPIC_API_KEY="dummy-key"
fi

# Create a temporary directory for HTML files
TEMP_DIR=$(mktemp -d)
HTML_DIR="${TEMP_DIR}/html"

# Create directory for HTML files
mkdir -p "${HTML_DIR}"

echo -e "${BLUE}Temporary directory created at: ${TEMP_DIR}${NC}"
echo -e "${BLUE}HTML files will be saved to: ${HTML_DIR}${NC}"

# Export the HTML directory for the scraper to use
export HTML_DIR="${HTML_DIR}"

# Cleanup function to remove temporary files
cleanup() {
    echo -e "\n${YELLOW}Cleaning up...${NC}"
    
    # Display HTML file location before cleanup
    echo -e "${YELLOW}HTML files are available at: ${HTML_DIR}${NC}"
    
    # Ask if user wants to keep the temporary files
    read -p "Do you want to keep the temporary files? (y/n): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        # Remove temporary directory
        rm -rf "${TEMP_DIR}"
        echo -e "${GREEN}Temporary files removed.${NC}"
    else
        echo -e "${GREEN}Temporary files kept at: ${TEMP_DIR}${NC}"
    fi
    
    echo -e "${GREEN}Cleanup complete.${NC}"
    exit 0
}

# Set trap to call cleanup function on exit
trap cleanup EXIT INT TERM

# Build the example
echo -e "${BLUE}${BOLD}Building interactive MCP client...${NC}"
cargo build --example interactive_mcp_client

if [ $? -ne 0 ]; then
    echo -e "${RED}Error: Failed to build example.${NC}"
    exit 1
fi

echo -e "${GREEN}Build successful!${NC}"

# Run the interactive MCP client
echo -e "${BLUE}${BOLD}Running interactive MCP client...${NC}"
echo -e "${YELLOW}This client will start and connect to the MCP web scraper server.${NC}"
cargo run --example interactive_mcp_client 