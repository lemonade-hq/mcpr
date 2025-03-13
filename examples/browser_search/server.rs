//! Browser Search MCP Server Example
//!
//! This example demonstrates an MCP server that offers a tool to search the web
//! using the default browser.

use mcpr::constants::LATEST_PROTOCOL_VERSION;
use mcpr::error::MCPError;
use mcpr::schema::{
    CallToolParams, CallToolResult, Implementation, InitializeResult, JSONRPCError, JSONRPCMessage,
    JSONRPCRequest, JSONRPCResponse, RequestId, ServerCapabilities, TextContent, Tool,
    ToolInputSchema, ToolResultContent, ToolsCapability,
};
use mcpr::transport::{stdio::StdioTransport, Transport};

use serde_json::{json, Value};
use std::io::{self, Write};
use std::process::Command;
use url::Url;

fn main() -> Result<(), MCPError> {
    // Create a stdio transport
    let mut transport = StdioTransport::new();
    println!("Browser Search MCP Server started. Waiting for client connection...");

    // Wait for initialize request
    println!("Waiting for initialize request...");
    let message: JSONRPCMessage = match transport.receive() {
        Ok(msg) => {
            println!("Received message: {:?}", msg);
            msg
        }
        Err(e) => {
            eprintln!("Error receiving message: {}", e);
            return Err(e);
        }
    };

    match message {
        JSONRPCMessage::Request(request) => {
            println!("Received request: {}", request.method);
            if request.method == "initialize" {
                println!("Processing initialize request with ID: {:?}", request.id);

                // Define our search tool
                let search_tool = Tool {
                    name: "web_search".to_string(),
                    description: Some("Search the web using the default browser".to_string()),
                    input_schema: ToolInputSchema {
                        r#type: "object".to_string(),
                        properties: Some(
                            [
                                (
                                    "query".to_string(),
                                    json!({
                                        "type": "string",
                                        "description": "The search query to use"
                                    }),
                                ),
                                (
                                    "engine".to_string(),
                                    json!({
                                        "type": "string",
                                        "description": "The search engine to use (google, bing, duckduckgo)",
                                        "enum": ["google", "bing", "duckduckgo"]
                                    }),
                                ),
                            ]
                            .into_iter()
                            .collect(),
                        ),
                        required: Some(vec!["query".to_string()]),
                    },
                };

                // Handle initialize request
                let result = InitializeResult {
                    protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
                    capabilities: ServerCapabilities {
                        experimental: None,
                        logging: Some(Value::Object(serde_json::Map::new())),
                        prompts: None,
                        resources: None,
                        tools: Some(ToolsCapability {
                            list_changed: Some(false),
                        }),
                    },
                    server_info: Implementation {
                        name: "browser-search-server".to_string(),
                        version: "0.1.0".to_string(),
                    },
                    instructions: Some(
                        "This server provides a tool to search the web using your default browser."
                            .to_string(),
                    ),
                };

                // Send initialize response
                let response =
                    JSONRPCResponse::new(request.id, serde_json::to_value(result).unwrap());
                println!("Sending initialize response");
                let response_json = serde_json::to_string(&response).unwrap();
                println!("Response JSON: {}", response_json);
                transport.send(&response)?;

                println!("Server initialized successfully");

                // Main message loop
                loop {
                    println!("Waiting for next message...");
                    let message: JSONRPCMessage = match transport.receive() {
                        Ok(msg) => {
                            println!("Received message: {:?}", msg);
                            msg
                        }
                        Err(e) => {
                            eprintln!("Error receiving message: {}", e);
                            break;
                        }
                    };

                    match message {
                        JSONRPCMessage::Request(request) => {
                            println!(
                                "Received request: {} with ID: {:?}",
                                request.method, request.id
                            );
                            // Handle request
                            match request.method.as_str() {
                                "tools/list" => {
                                    // Return our search tool
                                    let response = JSONRPCResponse::new(
                                        request.id,
                                        json!({
                                            "tools": [search_tool]
                                        }),
                                    );
                                    println!("Sending tools list response");
                                    let response_json = serde_json::to_string(&response).unwrap();
                                    println!("Response JSON: {}", response_json);
                                    transport.send(&response)?;
                                }
                                "tools/call" => {
                                    // Handle tool call
                                    if let Some(params) = request.params {
                                        println!("Tool call params: {}", params);
                                        let params: CallToolParams =
                                            match serde_json::from_value(params) {
                                                Ok(p) => p,
                                                Err(e) => {
                                                    eprintln!(
                                                        "Invalid tool call parameters: {}",
                                                        e
                                                    );
                                                    let error = JSONRPCError::new(
                                                        request.id,
                                                        -32602,
                                                        format!(
                                                            "Invalid tool call parameters: {}",
                                                            e
                                                        ),
                                                        None,
                                                    );
                                                    println!(
                                                    "Sending error response for invalid parameters"
                                                );
                                                    transport.send(&error)?;
                                                    continue;
                                                }
                                            };

                                        if params.name == "web_search" {
                                            println!("Processing web_search tool call");
                                            // Extract search parameters
                                            let query = match params
                                                .arguments
                                                .as_ref()
                                                .and_then(|args| args.get("query"))
                                                .and_then(|q| q.as_str())
                                            {
                                                Some(q) => q,
                                                None => {
                                                    eprintln!("Missing required 'query' parameter");
                                                    let error = JSONRPCError::new(
                                                        request.id,
                                                        -32602,
                                                        "Missing required 'query' parameter"
                                                            .to_string(),
                                                        None,
                                                    );
                                                    println!(
                                                        "Sending error response for missing query"
                                                    );
                                                    transport.send(&error)?;
                                                    continue;
                                                }
                                            };

                                            let engine = params
                                                .arguments
                                                .as_ref()
                                                .and_then(|args| args.get("engine"))
                                                .and_then(|e| e.as_str())
                                                .unwrap_or("google");

                                            println!(
                                                "Performing search: query='{}', engine='{}'",
                                                query, engine
                                            );

                                            // Perform the search
                                            let result = match perform_search(query, engine) {
                                                Ok(r) => r,
                                                Err(e) => {
                                                    eprintln!("Error performing search: {}", e);
                                                    let error = JSONRPCError::new(
                                                        request.id,
                                                        -32603,
                                                        format!("Error performing search: {}", e),
                                                        None,
                                                    );
                                                    println!(
                                                        "Sending error response for search failure"
                                                    );
                                                    transport.send(&error)?;
                                                    continue;
                                                }
                                            };

                                            // Return the result
                                            let response = JSONRPCResponse::new(
                                                request.id,
                                                serde_json::to_value(result).unwrap(),
                                            );
                                            println!("Sending search result response");
                                            let response_json =
                                                serde_json::to_string(&response).unwrap();
                                            println!("Response JSON: {}", response_json);
                                            transport.send(&response)?;
                                        } else {
                                            // Unknown tool
                                            eprintln!("Unknown tool: {}", params.name);
                                            let error = JSONRPCError::new(
                                                request.id,
                                                -32601,
                                                format!("Unknown tool: {}", params.name),
                                                None,
                                            );
                                            println!("Sending error response for unknown tool");
                                            transport.send(&error)?;
                                        }
                                    } else {
                                        // Missing parameters
                                        eprintln!("Missing tool call parameters");
                                        let error = JSONRPCError::new(
                                            request.id,
                                            -32602,
                                            "Missing tool call parameters".to_string(),
                                            None,
                                        );
                                        println!("Sending error response for missing parameters");
                                        transport.send(&error)?;
                                    }
                                }
                                "ping" => {
                                    // Send empty response
                                    let response = JSONRPCResponse::new(
                                        request.id,
                                        Value::Object(serde_json::Map::new()),
                                    );
                                    println!("Sending ping response");
                                    transport.send(&response)?;
                                }
                                _ => {
                                    // Method not found
                                    eprintln!("Method not found: {}", request.method);
                                    let error = JSONRPCError::new(
                                        request.id,
                                        -32601,
                                        format!("Method not found: {}", request.method),
                                        None,
                                    );
                                    println!("Sending error response for unknown method");
                                    transport.send(&error)?;
                                }
                            }
                        }
                        JSONRPCMessage::Notification(notification) => {
                            println!("Received notification: {}", notification.method);
                            // Handle notification
                            match notification.method.as_str() {
                                "notifications/initialized" => {
                                    println!("Client initialized");
                                }
                                _ => {
                                    println!("Received notification: {}", notification.method);
                                }
                            }
                        }
                        _ => {
                            // Ignore other message types
                            println!("Received other message type: {:?}", message);
                        }
                    }
                }
            } else {
                // Not an initialize request
                eprintln!("Expected initialize request, got: {}", request.method);
                let error = JSONRPCError::new(
                    request.id,
                    -32600,
                    "Expected initialize request".to_string(),
                    None,
                );
                println!("Sending error response for invalid initial request");
                transport.send(&error)?;
            }
        }
        _ => {
            // Not a request
            eprintln!(
                "Expected initialize request, got something else: {:?}",
                message
            );
        }
    }

    Ok(())
}

/// Perform a web search by opening the default browser
fn perform_search(query: &str, engine: &str) -> Result<CallToolResult, MCPError> {
    // Build the search URL based on the engine
    let search_url = match engine {
        "google" => format!("https://www.google.com/search?q={}", url_encode(query)),
        "bing" => format!("https://www.bing.com/search?q={}", url_encode(query)),
        "duckduckgo" => format!("https://duckduckgo.com/?q={}", url_encode(query)),
        _ => format!("https://www.google.com/search?q={}", url_encode(query)),
    };

    println!("Opening browser with search URL: {}", search_url);

    // Open the default browser with the search URL
    #[cfg(target_os = "windows")]
    {
        println!("Using Windows 'cmd' to open browser");
        Command::new("cmd")
            .args(["/c", "start", &search_url])
            .spawn()
            .map_err(|e| MCPError::Transport(format!("Failed to open browser: {}", e)))?;
    }

    #[cfg(target_os = "macos")]
    {
        println!("Using macOS 'open' command to open browser");
        Command::new("open")
            .arg(&search_url)
            .spawn()
            .map_err(|e| MCPError::Transport(format!("Failed to open browser: {}", e)))?;
    }

    #[cfg(target_os = "linux")]
    {
        println!("Using Linux 'xdg-open' command to open browser");
        Command::new("xdg-open")
            .arg(&search_url)
            .spawn()
            .map_err(|e| MCPError::Transport(format!("Failed to open browser: {}", e)))?;
    }

    // Create the result with more detailed information
    let result = CallToolResult {
        content: vec![ToolResultContent::Text(TextContent {
            r#type: "text".to_string(),
            text: format!(
                "Search initiated for '{}' using {} engine.\nBrowser opened with URL: {}\n\nThe search results should now be visible in your browser.",
                query, engine, search_url
            ),
            annotations: None,
        })],
        is_error: None,
    };

    println!("Search completed successfully");
    Ok(result)
}

/// Simple URL encoding function
fn url_encode(s: &str) -> String {
    let url = Url::parse(&format!("https://example.com/?q={}", s)).unwrap();
    url.query().unwrap().strip_prefix("q=").unwrap().to_string()
}
