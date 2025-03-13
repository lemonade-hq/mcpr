#!/bin/bash

# Colors for better output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== MCP LLM Tool Caller Example ===${NC}"
echo -e "${YELLOW}Starting server and API endpoint...${NC}"

# Check if Cargo is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Cargo is not installed. Please install Rust and Cargo first.${NC}"
    exit 1
fi

# Build the examples
echo -e "${YELLOW}Building examples...${NC}"
cargo build --example browser_search_server --example llm_tool_caller

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
    if [ ! -z "$API_PID" ]; then
        echo -e "${YELLOW}Stopping API endpoint (PID: $API_PID)...${NC}"
        kill $API_PID 2>/dev/null || true
    fi
    # Remove temporary directory
    rm -rf "$TEMP_DIR"
    echo -e "${GREEN}Done!${NC}"
    exit 0
}

# Set up trap for cleanup
trap cleanup INT TERM EXIT

# Create named pipes for communication
echo -e "${YELLOW}Setting up communication channels...${NC}"
SERVER_TO_CLIENT="$TEMP_DIR/server_to_client"
CLIENT_TO_SERVER="$TEMP_DIR/client_to_server"

mkfifo "$SERVER_TO_CLIENT"
mkfifo "$CLIENT_TO_SERVER"

# Start the server in the background
echo -e "${YELLOW}Starting MCP server...${NC}"
RUST_LOG=debug cargo run --example browser_search_server < "$CLIENT_TO_SERVER" > "$SERVER_TO_CLIENT" 2> "$TEMP_DIR/server.log" &
SERVER_PID=$!

# Give the server a moment to initialize
sleep 2

# Check if server is running
if ! kill -0 $SERVER_PID 2>/dev/null; then
    echo -e "${RED}Server failed to start. Check for errors in $TEMP_DIR/server.log.${NC}"
    cat "$TEMP_DIR/server.log"
    cleanup
    exit 1
fi

echo -e "${GREEN}MCP server started with PID $SERVER_PID${NC}"

# Start the LLM tool caller API endpoint
echo -e "${YELLOW}Starting LLM tool caller API endpoint...${NC}"
RUST_LOG=debug cargo run --example llm_tool_caller < "$SERVER_TO_CLIENT" > "$CLIENT_TO_SERVER" 2> "$TEMP_DIR/api.log" &
API_PID=$!

# Give the API endpoint a moment to initialize
sleep 2

# Check if API endpoint is running
if ! kill -0 $API_PID 2>/dev/null; then
    echo -e "${RED}API endpoint failed to start. Check for errors in $TEMP_DIR/api.log.${NC}"
    cat "$TEMP_DIR/api.log"
    cleanup
    exit 1
fi

echo -e "${GREEN}LLM tool caller API endpoint started with PID $API_PID${NC}"

# Print instructions
echo -e "\n${BOLD}${BLUE}=== LLM TOOL CALLER INSTRUCTIONS ===${NC}"
echo -e "${BOLD}The API endpoint is now running at http://127.0.0.1:8080${NC}"
echo -e "${BOLD}Available endpoints:${NC}"
echo -e "${BOLD}  - GET /tools - List available tools${NC}"
echo -e "${BOLD}  - POST /tool-call - Call a tool${NC}"
echo -e "${BOLD}Example curl command to call the web_search tool:${NC}"
echo -e "${YELLOW}curl -X POST http://127.0.0.1:8080/tool-call \\
  -H \"Content-Type: application/json\" \\
  -d '{
    \"tool_name\": \"web_search\",
    \"parameters\": {
      \"query\": \"Rust programming language\",
      \"engine\": \"google\"
    },
    \"api_key\": \"test-api-key\"
  }'${NC}"

echo -e "\n${BOLD}Press Ctrl+C to stop the server and API endpoint${NC}"

# Wait for user to press Ctrl+C
wait $API_PID

echo -e "${GREEN}API endpoint stopped. Shutting down...${NC}"
cleanup 