#!/bin/bash

# Colors for better output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Standalone LLM Tool Caller Example ===${NC}"

# Check if Cargo is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Cargo is not installed. Please install Rust and Cargo first.${NC}"
    exit 1
fi

# Build the example
echo -e "${YELLOW}Building example...${NC}"
cargo build --example standalone_tool_caller

if [ $? -ne 0 ]; then
    echo -e "${RED}Build failed. Please fix the errors and try again.${NC}"
    exit 1
fi

# Run the standalone tool caller
echo -e "${YELLOW}Starting standalone tool caller...${NC}"
RUST_LOG=debug cargo run --example standalone_tool_caller &
SERVER_PID=$!

# Give the server a moment to initialize
sleep 2

# Check if server is running
if ! kill -0 $SERVER_PID 2>/dev/null; then
    echo -e "${RED}Server failed to start.${NC}"
    exit 1
fi

echo -e "${GREEN}Standalone tool caller started with PID $SERVER_PID${NC}"

# Print instructions
echo -e "\n${BOLD}${BLUE}=== TOOL CALLER INSTRUCTIONS ===${NC}"
echo -e "${BOLD}The API endpoint is now running at http://127.0.0.1:8080${NC}"
echo -e "${BOLD}Available endpoints:${NC}"
echo -e "${BOLD}  - GET /tools - List available tools${NC}"
echo -e "${BOLD}  - POST /tool-call - Call a tool${NC}"
echo -e "${BOLD}Example curl commands:${NC}"

echo -e "\n${YELLOW}# List available tools${NC}"
echo -e "${YELLOW}curl -X GET http://127.0.0.1:8080/tools${NC}"

echo -e "\n${YELLOW}# Call the web_search tool${NC}"
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

echo -e "\n${YELLOW}# Call the calculator tool${NC}"
echo -e "${YELLOW}curl -X POST http://127.0.0.1:8080/tool-call \\
  -H \"Content-Type: application/json\" \\
  -d '{
    \"tool_name\": \"calculator\",
    \"parameters\": {
      \"expression\": \"42 * 2\"
    },
    \"api_key\": \"test-api-key\"
  }'${NC}"

echo -e "\n${BOLD}Press Ctrl+C to stop the server${NC}"

# Wait for user to press Ctrl+C
wait $SERVER_PID

echo -e "${GREEN}Server stopped.${NC}" 