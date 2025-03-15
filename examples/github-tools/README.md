# GitHub Tools Example

This example demonstrates how to build a simple MCP (Machine Comprehension Protocol) client-server application that interacts with GitHub repositories. The example includes a tool that can query a GitHub repository's README file and answer questions about it.

## Features

- **README Query Tool**: Ask questions about a GitHub repository's README file and get relevant answers.
- **Repository Search Tool**: Search for GitHub repositories based on keywords.
- **Interactive Mode**: Choose tools and provide parameters interactively with a user-friendly interface.
- **Command-Line Mode**: Run queries directly from the command line.

## Project Structure

- `client/`: The MCP client implementation
- `server/`: The MCP server implementation with GitHub tools

## Building the Example

To build both the client and server:

```bash
# Build the server
cd server
cargo build

# Build the client
cd ../client
cargo build
```

## Environment Setup

The server uses environment variables for configuration. You can set these in two ways:

1. Create a `.env` file in the server directory (see `.env.example` for required variables)
2. Set environment variables directly in your shell

The server will first check for a `.env` file, and if not found, it will use environment variables.

Required environment variables:
- `ANTHROPIC_API_KEY`: Your API key for Anthropic's Claude 3.7 Sonnet model
- `GITHUB_TOKEN` (optional): GitHub API token to avoid rate limits

## Running the Example

### Starting the Server

There are two ways to run the server:

#### Option 1: Start the server as a standalone process

```bash
cd server
cargo run
```

#### Option 2: Start the server in the background

```bash
cd server
./target/debug/github-tools-server < /dev/null > server_output.log 2>&1 &
```

This will start the server in the background, redirecting its output to `server_output.log`.

### Using the Client

#### Interactive Mode (Recommended)

To run the client in interactive mode:

```bash
cd client
cargo run -- --interactive
```

Or, to connect to an already running server:

```bash
cd client
cargo run -- --connect
```

This will display available tools and prompt you to select one. For the README query tool, you'll be asked to provide a GitHub repository (in the format `owner/repo`) and your question about the repository.

#### Command-Line Mode

To directly query a repository's README:

```bash
cd client
cargo run -- --repo "rust-lang/rust" --query "What is Rust used for?"
```

To search for repositories:

```bash
cd client
cargo run -- --query "machine learning"
```

## How It Works

1. The client establishes a connection with the server using stdio transport.
2. The server provides a list of available tools during initialization.
3. The client sends tool call requests to the server.
4. For README queries, the server:
   - Fetches the README from the GitHub API
   - Analyzes the content to find relevant information
   - Returns an answer based on the README content using Claude 3.7 Sonnet

## Generating Your Own MCP Template

This example can serve as a template for your own MCP-based applications. Here's how to create your own project based on this template:

1. **Copy the basic structure**:
   ```bash
   mkdir -p my-mcp-project/{client,server}/src
   cp examples/github-tools/client/Cargo.toml my-mcp-project/client/
   cp examples/github-tools/server/Cargo.toml my-mcp-project/server/
   ```

2. **Modify the client**:
   - Update the client's `Cargo.toml` to reflect your project's name and dependencies
   - Create a new `main.rs` in `my-mcp-project/client/src/` based on the GitHub tools client
   - Customize the client to support your specific tools and UI requirements

3. **Implement your server and tools**:
   - Update the server's `Cargo.toml` to reflect your project's name and dependencies
   - Create a new `main.rs` in `my-mcp-project/server/src/` based on the GitHub tools server
   - Implement your own tools by creating new structs that implement the `Tool` trait

4. **Test your implementation**:
   - Build and run your server
   - Connect to it with your client
   - Test your tools to ensure they work as expected

## Extending the Example

You can extend this example by:

1. Adding more GitHub-related tools (e.g., issue search, repository statistics)
2. Improving the answer generation with more sophisticated text analysis or AI
3. Adding authentication to access private repositories
4. Creating a web-based client instead of a terminal client

## License

This example is part of the MCPR project and is licensed under the same terms as the main project.

# GitHub Tools Server

This is a server implementation that provides tools for interacting with GitHub repositories. It uses the MCPR (Model-Client-Protocol) library to provide a standardized interface for tool calls.

## Available Tools

The server provides the following tools:

1. **readme_query**: Ask questions about a GitHub project based on its README content.
   - Parameters:
     - `repo`: Repository in format owner/repo (e.g., "rust-lang/rust")
     - `query`: Your question about the project (e.g., "What is Rust used for?")

2. **repo_search**: Search for GitHub repositories based on keywords.
   - Parameters:
     - `query`: Search query for repositories (e.g., "machine learning")
     - `limit`: Maximum number of repositories to return (default: 5)

## Running the Server

The server can be run using:

```bash
cd server
cargo run
```

The server uses stdio transport by default, which means it reads JSON-RPC messages from stdin and writes responses to stdout.

## Client-Server Architecture

The GitHub Tools example demonstrates a client-server architecture where:

1. The server implements the tools and their functionality
2. The client provides a user interface to interact with these tools
3. Communication happens via the MCP protocol over stdio transport

This separation allows:
- Multiple clients to connect to the same server
- The server to run independently of any client
- Different types of clients (CLI, GUI, web) to use the same server

To demonstrate this architecture:

1. Start the server in the background:
   ```bash
   ./server/target/debug/github-tools-server < /dev/null > server_output.log 2>&1 &
   ```

2. Connect to it with the client:
   ```bash
   ./client/target/debug/github-tools-client --connect
   ```

3. Run multiple clients simultaneously, all connecting to the same server.

## JSON-RPC Protocol

The server uses the JSON-RPC 2.0 protocol. Here are some example requests:

### Initialization

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {}
}
```

### Tool Call (readme_query)

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tool_call",
  "params": {
    "name": "readme_query",
    "parameters": {
      "repo": "rust-lang/rust",
      "query": "What is Rust used for?"
    }
  }
}
```

### Tool Call (repo_search)

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tool_call",
  "params": {
    "name": "repo_search",
    "parameters": {
      "query": "machine learning",
      "limit": 3
    }
  }
}
```

### Shutdown

```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "shutdown",
  "params": {}
}
```

## Implementation Details

The server is implemented in Rust and uses the following dependencies:

- `mcpr`: For the JSON-RPC protocol and tool handling
- `reqwest`: For making HTTP requests to the GitHub API
- `serde_json`: For JSON serialization/deserialization
- `base64`: For encoding/decoding base64 data
- `anyhow`: For error handling
- `dotenv`: For loading environment variables

The `ReadmeQueryTool` can use the Anthropic API for generating answers if the `ANTHROPIC_API_KEY` environment variable is set. Otherwise, it falls back to a simpler keyword-based method.

## Environment Variables

- `ANTHROPIC_API_KEY`: API key for Anthropic (optional, used by the `ReadmeQueryTool`)
- `GITHUB_TOKEN`: GitHub API token (optional, used to increase rate limits for GitHub API calls)
