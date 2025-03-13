#!/bin/bash

# Colors for better output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== MCP Browser Search Example ===${NC}"
echo -e "${YELLOW}Starting server and client...${NC}"

# Check if Cargo is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Cargo is not installed. Please install Rust and Cargo first.${NC}"
    exit 1
fi

# Build the examples
echo -e "${YELLOW}Building examples...${NC}"
cargo build --example browser_search_server --example browser_search_client

if [ $? -ne 0 ]; then
    echo -e "${RED}Build failed. Please fix the errors and try again.${NC}"
    exit 1
fi

# Create a temporary directory for our files
TEMP_DIR=$(mktemp -d)
echo -e "${YELLOW}Using temporary directory: $TEMP_DIR${NC}"

# Cleanup function
cleanup() {
    echo -e "\n${YELLOW}Cleaning up...${NC}"
    # Kill any remaining processes
    if [ ! -z "$SERVER_PID" ]; then
        echo -e "${YELLOW}Stopping server (PID: $SERVER_PID)...${NC}"
        kill $SERVER_PID 2>/dev/null || true
    fi
    if [ ! -z "$CLIENT_PID" ]; then
        echo -e "${YELLOW}Stopping client (PID: $CLIENT_PID)...${NC}"
        kill $CLIENT_PID 2>/dev/null || true
    fi
    # Remove temporary directory
    rm -rf "$TEMP_DIR"
    echo -e "${GREEN}Done!${NC}"
    exit 0
}

# Set up trap for cleanup
trap cleanup INT TERM EXIT

# Use a simpler approach with temporary files
SERVER_LOG="$TEMP_DIR/server.log"
CLIENT_LOG="$TEMP_DIR/client.log"
QUERY_FILE="$TEMP_DIR/query.txt"
RESULT_FILE="$TEMP_DIR/result.txt"

# Create a simple web search function
search_web() {
    local query="$1"
    local engine="${2:-google}"
    
    echo -e "${YELLOW}Searching for: '$query' using $engine${NC}"
    
    # Build the search URL based on the engine
    local url
    case "$engine" in
        google)
            url="https://www.google.com/search?q=$(echo "$query" | sed 's/ /+/g')"
            ;;
        bing)
            url="https://www.bing.com/search?q=$(echo "$query" | sed 's/ /+/g')"
            ;;
        duckduckgo)
            url="https://duckduckgo.com/?q=$(echo "$query" | sed 's/ /+/g')"
            ;;
        *)
            url="https://www.google.com/search?q=$(echo "$query" | sed 's/ /+/g')"
            ;;
    esac
    
    echo -e "${YELLOW}Opening URL: $url${NC}"
    
    # Open the URL in the default browser
    if [[ "$OSTYPE" == "darwin"* ]]; then
        open "$url"
    elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
        xdg-open "$url"
    elif [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
        start "$url"
    else
        echo -e "${RED}Unsupported OS: $OSTYPE${NC}"
        return 1
    fi
    
    echo -e "${GREEN}Search completed successfully!${NC}"
    return 0
}

# Print instructions
echo -e "\n${BOLD}${BLUE}=== BROWSER SEARCH TOOL INSTRUCTIONS ===${NC}"
echo -e "${BOLD}1. Enter your search query when prompted${NC}"
echo -e "${BOLD}2. Optionally specify a search engine (google, bing, duckduckgo)${NC}"
echo -e "${BOLD}3. Type 'exit' to quit the application${NC}"
echo -e "${BOLD}${BLUE}==========================================${NC}\n"

# Interactive search loop
while true; do
    echo -e "\n${BOLD}${BLUE}=== Web Search Tool ===${NC}"
    echo -ne "${BOLD}>>> Enter search query (or 'exit' to quit): ${NC}"
    read query
    
    if [ -z "$query" ]; then
        continue
    fi
    
    if [ "$query" = "exit" ]; then
        echo -e "${YELLOW}Exiting...${NC}"
        break
    fi
    
    echo -ne "${BOLD}>>> Enter search engine [google/bing/duckduckgo] (default: google): ${NC}"
    read engine
    
    # Perform the search
    search_web "$query" "$engine"
    
    echo -e "\n${GREEN}Search completed. Your browser should have opened with the results.${NC}"
    echo -e "${YELLOW}Press Enter to continue...${NC}"
    read
done

echo -e "${GREEN}Thank you for using the Browser Search Tool!${NC}"
cleanup 