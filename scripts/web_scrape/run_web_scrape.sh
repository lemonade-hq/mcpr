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

# Create a temporary directory for HTML files and logs
TEMP_DIR=$(mktemp -d)
HTML_DIR="${TEMP_DIR}/html"
LOG_DIR="${TEMP_DIR}/logs"

# Create directories for HTML files and logs
mkdir -p "${HTML_DIR}"
mkdir -p "${LOG_DIR}"

# Create log files
SERVER_LOG="${LOG_DIR}/server.log"
CLIENT_LOG="${LOG_DIR}/client.log"

echo -e "${BLUE}Temporary directory created at: ${TEMP_DIR}${NC}"
echo -e "${BLUE}HTML files will be saved to: ${HTML_DIR}${NC}"
echo -e "${BLUE}Logs will be saved to: ${LOG_DIR}${NC}"

# Cleanup function to remove temporary files and kill background processes
cleanup() {
    echo -e "\n${YELLOW}Cleaning up...${NC}"
    # Kill any background processes
    if [[ -n $SERVER_PID ]]; then
        kill $SERVER_PID 2>/dev/null || true
    fi
    
    # Display log locations before cleanup
    echo -e "${YELLOW}Logs are available at:${NC}"
    echo -e "  Server log: ${SERVER_LOG}"
    echo -e "  Client log: ${CLIENT_LOG}"
    echo -e "${YELLOW}HTML files are available at: ${HTML_DIR}${NC}"
    
    # Ask if user wants to keep the temporary files
    read -p "Do you want to keep the temporary files? (y/n): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        # Remove temporary directory and pipes
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

# Check if ANTHROPIC_API_KEY is set
if [[ -z "${ANTHROPIC_API_KEY}" ]]; then
    echo -e "${YELLOW}Warning: ANTHROPIC_API_KEY environment variable is not set.${NC}"
    echo -e "${YELLOW}The web scraping server requires an Anthropic API key to extract content.${NC}"
    echo -e "${YELLOW}Please set it using: export ANTHROPIC_API_KEY=your_api_key${NC}"
    
    # Ask if the user wants to continue without the API key
    read -p "Do you want to continue anyway? (y/n): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${RED}Exiting...${NC}"
        exit 1
    fi
fi

# Export the HTML directory for the server to use
export HTML_DIR="${HTML_DIR}"

# Build the examples
echo -e "${BLUE}${BOLD}Building web scraping examples...${NC}"
cargo build --example web_scrape_server --example web_scrape_client

if [ $? -ne 0 ]; then
    echo -e "${RED}Error: Failed to build examples.${NC}"
    exit 1
fi

echo -e "${GREEN}Build successful!${NC}"

# Create a simple interactive menu
echo -e "${BLUE}${BOLD}Web Scraping Tool${NC}"
echo -e "${YELLOW}This tool allows you to scrape web content and extract information using Claude.${NC}"
echo

while true; do
    echo -e "${BLUE}${BOLD}Menu:${NC}"
    echo -e "1. ${YELLOW}Scrape a web page${NC}"
    echo -e "2. ${YELLOW}Exit${NC}"
    echo
    read -p "Enter your choice (1-2): " choice
    echo
    
    case $choice in
        1)
            # Get URL to scrape
            read -p "Enter the URL to scrape: " url
            if [[ -z "$url" ]]; then
                echo -e "${RED}URL cannot be empty. Please try again.${NC}"
                continue
            fi
            
            # Get CSS selector (optional)
            read -p "Enter CSS selector (optional, e.g., 'div.content', 'article', '#main'): " css_selector
            
            # Get query
            read -p "Enter your query (what information are you looking for?): " query
            if [[ -z "$query" ]]; then
                echo -e "${RED}Query cannot be empty. Please try again.${NC}"
                continue
            fi
            
            echo -e "${YELLOW}Scraping web page: ${url}${NC}"
            if [[ ! -z "$css_selector" ]]; then
                echo -e "${YELLOW}Using CSS selector: ${css_selector}${NC}"
            fi
            echo -e "${YELLOW}Query: ${query}${NC}"
            echo -e "${YELLOW}This may take a moment...${NC}"
            
            # Start the server in the background
            echo -e "${BLUE}Starting server...${NC}"
            cargo run --example web_scrape_server > "${SERVER_LOG}" 2>&1 &
            SERVER_PID=$!
            
            # Give the server a moment to start
            sleep 2
            
            # Check if server is running
            if ! ps -p $SERVER_PID > /dev/null; then
                echo -e "${RED}Error: Server failed to start. Check the log at ${SERVER_LOG}${NC}"
                continue
            fi
            
            # Run the client with the provided parameters
            echo -e "${BLUE}Sending request to server...${NC}"
            cargo run --example web_scrape_client -- --url "$url" --query "$query" ${css_selector:+--css-selector "$css_selector"} > "${CLIENT_LOG}" 2>&1
            
            # Display the result
            if [ $? -eq 0 ]; then
                echo -e "${GREEN}Scraping completed successfully!${NC}"
                echo -e "${YELLOW}Result:${NC}"
                cat "${CLIENT_LOG}" | grep -v "^warning:" | tail -n +10
            else
                echo -e "${RED}Error: Failed to scrape web page. Check the log at ${CLIENT_LOG}${NC}"
            fi
            
            # Kill the server
            if [[ -n $SERVER_PID ]]; then
                kill $SERVER_PID 2>/dev/null || true
                unset SERVER_PID
            fi
            ;;
        2)
            echo -e "${GREEN}Exiting...${NC}"
            exit 0
            ;;
        *)
            echo -e "${RED}Invalid choice. Please enter 1 or 2.${NC}"
            ;;
    esac
    
    echo
done

# Cleanup will be called automatically by the trap 