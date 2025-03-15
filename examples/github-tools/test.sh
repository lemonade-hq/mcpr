#!/bin/bash

# Simple test script for GitHub Tools project with stdio transport

# Exit on error
set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Build the server and client
echo -e "${BLUE}=== Building Components ===${NC}"
echo -e "${YELLOW}[SERVER]${NC} Building..."
cd server
cargo build
echo -e "${YELLOW}[SERVER]${NC} Build complete"

echo -e "${CYAN}[CLIENT]${NC} Building..."
cd ../client
cargo build
echo -e "${CYAN}[CLIENT]${NC} Build complete"
cd ..

# Start a server in the background
echo -e "\n${BLUE}=== Starting Server ===${NC}"
echo -e "${YELLOW}[SERVER]${NC} Starting server in background..."
./server/target/debug/github-tools-server > server_output.log 2>&1 &
SERVER_PID=$!
echo -e "${YELLOW}[SERVER]${NC} Started with PID: $SERVER_PID"

# Give the server a moment to initialize
sleep 1

# Run client in connect mode with default query
echo -e "\n${BLUE}=== Running Client (README Query) ===${NC}"
echo -e "${CYAN}[CLIENT]${NC} Connecting to server and querying rust-lang/rust repository..."
./client/target/debug/github-tools-client --connect --repo "rust-lang/rust" --query "What is Rust used for?"

# Run client with repository search
echo -e "\n${BLUE}=== Running Client (Repository Search) ===${NC}"
echo -e "${CYAN}[CLIENT]${NC} Connecting to server and searching for repositories..."
./client/target/debug/github-tools-client --connect --query "rust language"

# Stop the server
echo -e "\n${BLUE}=== Cleaning Up ===${NC}"
echo -e "${YELLOW}[SERVER]${NC} Stopping server (PID: $SERVER_PID)..."
kill $SERVER_PID
echo -e "${YELLOW}[SERVER]${NC} Server stopped"

# Clean up log file
rm server_output.log

echo -e "\n${GREEN}=== Test completed successfully! ===${NC}"
echo -e "${BLUE}Here are some ways to use the client:${NC}"
echo -e "  ${CYAN}# Connect to a running server in interactive mode (recommended):${NC}"
echo -e "  ./client/target/debug/github-tools-client --connect"
echo -e ""
echo -e "  ${CYAN}# Start a new server and run in interactive mode:${NC}"
echo -e "  ./client/target/debug/github-tools-client --interactive"
echo -e ""
echo -e "  ${CYAN}# Query a repository README:${NC}"
echo -e "  ./client/target/debug/github-tools-client --connect --repo \"owner/repo\" --query \"your question\""
echo -e ""
echo -e "  ${CYAN}# Search for repositories:${NC}"
echo -e "  ./client/target/debug/github-tools-client --connect --query \"search term\""
echo -e ""
echo -e "  ${CYAN}# Start a server in the background:${NC}"
echo -e "  ./server/target/debug/github-tools-server < /dev/null > server_output.log 2>&1 &"
