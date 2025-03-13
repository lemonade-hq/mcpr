//! MCP Web Scraper Client with TCP Socket
//!
//! This example demonstrates a client for the MCP web scraper server
//! that uses a TCP socket for communication.

use mcpr::error::MCPError;
use mcpr::schema::{
    CallToolParams, CallToolResult, InitializeParams, JSONRPCMessage, JSONRPCRequest, RequestId,
    TextContent, ToolResultContent,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpStream;

// ANSI color codes for better terminal output
const GREEN: &str = "\x1b[0;32m";
const YELLOW: &str = "\x1b[1;33m";
const BLUE: &str = "\x1b[0;34m";
const RED: &str = "\x1b[0;31m";
const CYAN: &str = "\x1b[0;36m";
const BOLD: &str = "\x1b[1m";
const NC: &str = "\x1b[0m"; // No Color

struct TCPTransport {
    stream: TcpStream,
    reader: BufReader<TcpStream>,
}

impl TCPTransport {
    fn new(stream: TcpStream) -> Self {
        let reader_stream = stream.try_clone().expect("Failed to clone TCP stream");
        let reader = BufReader::new(reader_stream);
        Self { stream, reader }
    }

    fn send<T: serde::Serialize>(&mut self, message: &T) -> Result<(), MCPError> {
        let json = serde_json::to_string(message)?;
        writeln!(self.stream, "{}", json).map_err(|e| MCPError::Transport(e.to_string()))?;
        self.stream
            .flush()
            .map_err(|e| MCPError::Transport(e.to_string()))?;
        Ok(())
    }

    fn receive<T: serde::de::DeserializeOwned>(&mut self) -> Result<T, MCPError> {
        let mut line = String::new();
        self.reader
            .read_line(&mut line)
            .map_err(|e| MCPError::Transport(e.to_string()))?;
        let message = serde_json::from_str(&line)?;
        Ok(message)
    }
}

struct MCPClient {
    transport: TCPTransport,
    initialized: bool,
    next_id: i64,
}

impl MCPClient {
    fn connect(address: &str) -> Result<Self, MCPError> {
        println!("{}Connecting to MCP server at {}...{}", BLUE, address, NC);
        let stream = TcpStream::connect(address)
            .map_err(|e| MCPError::Transport(format!("Failed to connect: {}", e)))?;

        let transport = TCPTransport::new(stream);

        Ok(Self {
            transport,
            initialized: false,
            next_id: 1,
        })
    }

    fn initialize(&mut self) -> Result<(), MCPError> {
        if self.initialized {
            return Ok(());
        }

        println!("{}Initializing connection to MCP server...{}", BLUE, NC);

        // Create initialize request
        let initialize_params = InitializeParams {
            protocol_version: mcpr::constants::LATEST_PROTOCOL_VERSION.to_string(),
            client_info: mcpr::schema::Implementation {
                name: "mcp-web-scraper-client".to_string(),
                version: "0.1.0".to_string(),
            },
            capabilities: mcpr::schema::ClientCapabilities {
                experimental: None,
                roots: None,
                sampling: None,
            },
        };

        let id = self.next_id();
        let initialize_request = JSONRPCRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(id),
            method: "initialize".to_string(),
            params: Some(serde_json::to_value(initialize_params).unwrap()),
        };

        println!("{}Sending initialize request...{}", YELLOW, NC);
        self.transport
            .send(&JSONRPCMessage::Request(initialize_request))?;

        // Receive initialize response
        println!("{}Waiting for initialize response...{}", YELLOW, NC);
        let response: JSONRPCMessage = self.transport.receive()?;

        match response {
            JSONRPCMessage::Response(_) => {
                println!("{}Server initialized successfully{}", GREEN, NC);

                // Send initialized notification
                let initialized_notification = mcpr::schema::JSONRPCNotification {
                    jsonrpc: "2.0".to_string(),
                    method: "notifications/initialized".to_string(),
                    params: None,
                };

                self.transport
                    .send(&JSONRPCMessage::Notification(initialized_notification))?;
                self.initialized = true;
                Ok(())
            }
            JSONRPCMessage::Error(error) => Err(MCPError::Protocol(format!(
                "Initialize failed: {:?}",
                error
            ))),
            _ => Err(MCPError::Protocol(
                "Unexpected response to initialize request".to_string(),
            )),
        }
    }

    fn list_tools(&mut self) -> Result<Vec<String>, MCPError> {
        if !self.initialized {
            return Err(MCPError::Protocol("Client not initialized".to_string()));
        }

        println!("{}Requesting tools list...{}", BLUE, NC);

        // Create tools list request
        let id = self.next_id();
        let tools_list_request = JSONRPCRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(id),
            method: "tools/list".to_string(),
            params: None,
        };

        self.transport
            .send(&JSONRPCMessage::Request(tools_list_request))?;

        // Receive tools list response
        let response: JSONRPCMessage = self.transport.receive()?;

        match response {
            JSONRPCMessage::Response(response) => {
                let tools_list: Value = response.result;

                if let Some(tools) = tools_list.get("tools").and_then(|t| t.as_array()) {
                    let tool_names: Vec<String> = tools
                        .iter()
                        .filter_map(|tool| {
                            tool.get("name").and_then(|n| n.as_str()).map(String::from)
                        })
                        .collect();

                    println!("{}Available tools: {}{}", GREEN, tool_names.join(", "), NC);
                    Ok(tool_names)
                } else {
                    println!("{}No tools available{}", YELLOW, NC);
                    Ok(vec![])
                }
            }
            JSONRPCMessage::Error(error) => Err(MCPError::Protocol(format!(
                "List tools failed: {:?}",
                error
            ))),
            _ => Err(MCPError::Protocol(
                "Unexpected response to tools list request".to_string(),
            )),
        }
    }

    fn call_web_scrape_tool(
        &mut self,
        url: &str,
        query: &str,
        css_selector: Option<&str>,
    ) -> Result<String, MCPError> {
        if !self.initialized {
            return Err(MCPError::Protocol("Client not initialized".to_string()));
        }

        println!("{}Calling web_scrape tool...{}", BLUE, NC);
        println!("{}URL: {}{}", CYAN, url, NC);
        println!("{}Query: {}{}", CYAN, query, NC);
        if let Some(selector) = css_selector {
            println!("{}CSS Selector: {}{}", CYAN, selector, NC);
        }

        // Create tool call request
        let mut arguments = HashMap::new();
        arguments.insert("url".to_string(), json!(url));
        arguments.insert("query".to_string(), json!(query));
        if let Some(selector) = css_selector {
            arguments.insert("css_selector".to_string(), json!(selector));
        }

        let tool_call_params = CallToolParams {
            name: "web_scrape".to_string(),
            arguments: Some(arguments),
        };

        let id = self.next_id();
        let tool_call_request = JSONRPCRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(id),
            method: "tools/call".to_string(),
            params: Some(serde_json::to_value(tool_call_params).unwrap()),
        };

        self.transport
            .send(&JSONRPCMessage::Request(tool_call_request))?;

        // Receive tool call response
        println!("{}Waiting for web scrape results...{}", YELLOW, NC);
        let response: JSONRPCMessage = self.transport.receive()?;

        match response {
            JSONRPCMessage::Response(response) => {
                let result: CallToolResult =
                    serde_json::from_value(response.result).map_err(|e| {
                        MCPError::Protocol(format!("Failed to parse tool call result: {}", e))
                    })?;

                let mut content_text = String::new();
                for content in result.content {
                    match content {
                        ToolResultContent::Text(TextContent { text, .. }) => {
                            content_text.push_str(&text);
                        }
                        _ => {
                            println!("{}Received non-text content{}", YELLOW, NC);
                        }
                    }
                }

                Ok(content_text)
            }
            JSONRPCMessage::Error(error) => {
                Err(MCPError::Protocol(format!("Tool call failed: {:?}", error)))
            }
            _ => Err(MCPError::Protocol(
                "Unexpected response to tool call request".to_string(),
            )),
        }
    }

    fn next_id(&mut self) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

fn read_input(prompt: &str) -> String {
    print!("{}{}{}", YELLOW, prompt, NC);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().lock().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn main() -> Result<(), MCPError> {
    println!("{}{}MCP Web Scraper Client{}{}", BOLD, BLUE, NC, NC);
    println!(
        "{}This client connects to the MCP web scraper server.{}",
        CYAN, NC
    );

    // Check if ANTHROPIC_API_KEY is set
    if env::var("ANTHROPIC_API_KEY").is_err() {
        println!(
            "{}Warning: Make sure the server has ANTHROPIC_API_KEY set.{}",
            YELLOW, NC
        );
    }

    // Connect to the server on port 8081 instead of 8080
    let mut client = MCPClient::connect("127.0.0.1:8081")?;

    // Initialize the client
    client.initialize()?;

    // List available tools
    let tools = client.list_tools()?;

    if !tools.contains(&"web_scrape".to_string()) {
        return Err(MCPError::Protocol(
            "Web scrape tool not available".to_string(),
        ));
    }

    // Main loop
    loop {
        println!("\n{}{}Web Scraping Options:{}{}", BOLD, BLUE, NC, NC);
        println!("{}1. Scrape a web page{}", CYAN, NC);
        println!("{}2. Exit{}", CYAN, NC);

        let choice = read_input("Enter your choice (1-2): ");

        match choice.as_str() {
            "1" => {
                // Get URL
                let url = read_input("Enter the URL to scrape: ");
                if url.is_empty() {
                    println!("{}URL cannot be empty{}", RED, NC);
                    continue;
                }

                // Get CSS selector (optional)
                let css_selector = read_input(
                    "Enter CSS selector (optional, e.g., 'div.content', 'article', '#main'): ",
                );
                let css_selector = if css_selector.is_empty() {
                    None
                } else {
                    Some(css_selector.as_str())
                };

                // Get query
                let query =
                    read_input("Enter your query (what information are you looking for?): ");
                if query.is_empty() {
                    println!("{}Query cannot be empty{}", RED, NC);
                    continue;
                }

                // Call the web scrape tool
                match client.call_web_scrape_tool(&url, &query, css_selector) {
                    Ok(result) => {
                        println!("\n{}{}Web Scrape Result:{}{}", BOLD, GREEN, NC, NC);
                        println!("{}", result);
                    }
                    Err(e) => {
                        println!("{}Error calling web scrape tool: {}{}", RED, e, NC);
                    }
                }
            }
            "2" => {
                println!("{}Exiting...{}", BLUE, NC);
                break;
            }
            _ => {
                println!("{}Invalid choice. Please enter 1 or 2.{}", RED, NC);
            }
        }
    }

    Ok(())
}
