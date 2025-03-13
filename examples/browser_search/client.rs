//! Browser Search MCP Client Example
//!
//! This example demonstrates an MCP client that connects to the browser search server
//! and initiates web searches.

use mcpr::constants::LATEST_PROTOCOL_VERSION;
use mcpr::error::MCPError;
use mcpr::schema::{
    CallToolParams, ClientCapabilities, Implementation, InitializeParams, JSONRPCMessage,
    JSONRPCNotification, JSONRPCRequest, RequestId, Tool,
};
use mcpr::transport::{stdio::StdioTransport, Transport};

use serde_json::json;
use std::collections::HashMap;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

// ANSI color codes for better terminal output
const GREEN: &str = "\x1b[0;32m";
const YELLOW: &str = "\x1b[1;33m";
const BLUE: &str = "\x1b[0;34m";
const BOLD: &str = "\x1b[1m";
const NC: &str = "\x1b[0m"; // No Color

fn main() -> Result<(), MCPError> {
    // Create a stdio transport
    let mut transport = StdioTransport::new();
    println!(
        "{}Browser Search MCP Client started. Connecting to server...{}",
        BLUE, NC
    );

    // Send initialize request
    let request_id = RequestId::Number(1);
    let initialize_params = InitializeParams {
        protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
        capabilities: ClientCapabilities {
            experimental: None,
            roots: None,
            sampling: None,
        },
        client_info: Implementation {
            name: "browser-search-client".to_string(),
            version: "0.1.0".to_string(),
        },
    };

    let initialize_request = JSONRPCRequest::new(
        request_id.clone(),
        "initialize".to_string(),
        Some(serde_json::to_value(initialize_params).unwrap()),
    );

    println!("{}Sending initialize request{}", YELLOW, NC);
    let request_json = serde_json::to_string(&initialize_request).unwrap();
    println!("Request JSON: {}", request_json);
    transport.send(&initialize_request)?;

    // Wait for initialize response
    println!("{}Waiting for initialize response...{}", YELLOW, NC);
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
        JSONRPCMessage::Response(response) => {
            println!(
                "{}Received initialize response{}: {:?}",
                GREEN, NC, response
            );

            // Send initialized notification
            let initialized_notification =
                JSONRPCNotification::new("notifications/initialized".to_string(), None);

            println!("{}Sending initialized notification{}", YELLOW, NC);
            transport.send(&initialized_notification)?;
            println!("{}Sent initialized notification{}", GREEN, NC);

            // Wait a moment to ensure the server processes the notification
            thread::sleep(Duration::from_millis(500));

            // Request available tools
            let tools_request =
                JSONRPCRequest::new(RequestId::Number(2), "tools/list".to_string(), None);

            println!("{}Requesting available tools{}", YELLOW, NC);
            transport.send(&tools_request)?;

            // Wait for tools response
            println!("{}Waiting for tools response...{}", YELLOW, NC);
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

            let tools = match message {
                JSONRPCMessage::Response(response) => {
                    if let Some(tools_list) = response.result.get("tools") {
                        println!("{}Received tools list{}: {:?}", GREEN, NC, tools_list);
                        let tools: Vec<Tool> =
                            serde_json::from_value(tools_list.clone()).map_err(|e| {
                                MCPError::Protocol(format!("Invalid tools list: {}", e))
                            })?;
                        tools
                    } else {
                        println!("{}No tools available in response{}", YELLOW, NC);
                        Vec::new()
                    }
                }
                JSONRPCMessage::Error(error) => {
                    println!(
                        "Received error: {} ({})",
                        error.error.message, error.error.code
                    );
                    Vec::new()
                }
                _ => {
                    println!("Received unexpected message type");
                    Vec::new()
                }
            };

            // Check if web_search tool is available
            if let Some(search_tool) = tools.iter().find(|t| t.name == "web_search") {
                println!(
                    "{}Found web_search tool{}: {}",
                    GREEN,
                    NC,
                    search_tool
                        .description
                        .as_ref()
                        .unwrap_or(&"No description".to_string())
                );

                println!("\n{}=== BROWSER SEARCH TOOL INSTRUCTIONS ==={}", BLUE, NC);
                println!("{}1. Enter your search query when prompted", NC);
                println!(
                    "{}2. Optionally specify a search engine (google, bing, duckduckgo)",
                    NC
                );
                println!("{}3. Type 'exit' to quit the application{}\n", NC, NC);

                // Interactive search loop
                loop {
                    println!("\n{}=== Web Search Tool ==={}", BOLD, NC);
                    print_with_prompt(format!(
                        "{}>>> Enter search query (or 'exit' to quit): {}",
                        BOLD, NC
                    ));

                    let mut query = String::new();
                    io::stdin().read_line(&mut query).unwrap();
                    let query = query.trim();

                    if query.is_empty() {
                        continue;
                    }

                    if query == "exit" {
                        println!("{}Exiting...{}", YELLOW, NC);
                        break;
                    }

                    print_with_prompt(format!(
                        "{}>>> Enter search engine [google/bing/duckduckgo] (default: google): {}",
                        BOLD, NC
                    ));

                    let mut engine = String::new();
                    io::stdin().read_line(&mut engine).unwrap();
                    let engine = engine.trim();

                    // Call the search tool
                    let call_params = CallToolParams {
                        name: "web_search".to_string(),
                        arguments: Some(if engine.is_empty() {
                            let mut args = HashMap::new();
                            args.insert("query".to_string(), json!(query));
                            args
                        } else {
                            let mut args = HashMap::new();
                            args.insert("query".to_string(), json!(query));
                            args.insert("engine".to_string(), json!(engine));
                            args
                        }),
                    };

                    let call_request = JSONRPCRequest::new(
                        RequestId::Number(3),
                        "tools/call".to_string(),
                        Some(serde_json::to_value(call_params).unwrap()),
                    );

                    println!("{}Calling web_search tool{}", YELLOW, NC);
                    let request_json = serde_json::to_string(&call_request).unwrap();
                    println!("Request JSON: {}", request_json);
                    transport.send(&call_request)?;

                    // Wait for tool call response
                    println!("{}Waiting for tool call response...{}", YELLOW, NC);
                    let message: JSONRPCMessage = match transport.receive() {
                        Ok(msg) => {
                            println!("Received message: {:?}", msg);
                            msg
                        }
                        Err(e) => {
                            eprintln!("Error receiving message: {}", e);
                            println!(
                                "{}Failed to get response from server. Try again.{}",
                                YELLOW, NC
                            );
                            continue;
                        }
                    };

                    match message {
                        JSONRPCMessage::Response(response) => {
                            println!("{}Response result{}: {:?}", GREEN, NC, response.result);
                            if let Some(content) = response.result.get("content") {
                                if let Some(content_array) = content.as_array() {
                                    for item in content_array {
                                        if let Some(text) = item.get("text") {
                                            println!("\n{}=== Search Result ==={}", BLUE, NC);
                                            println!("{}", text.as_str().unwrap_or(""));
                                            println!("{}===================={}", BLUE, NC);
                                        }
                                    }
                                } else {
                                    println!("Content is not an array: {:?}", content);
                                }
                            } else {
                                println!("{}No content in response{}", YELLOW, NC);
                            }
                        }
                        JSONRPCMessage::Error(error) => {
                            println!(
                                "{}Received error: {} ({}){}",
                                YELLOW, error.error.message, error.error.code, NC
                            );
                        }
                        _ => {
                            println!("{}Received unexpected message type{}", YELLOW, NC);
                        }
                    }
                }
            } else {
                println!("{}Web search tool not available{}", YELLOW, NC);
            }
        }
        JSONRPCMessage::Error(error) => {
            println!(
                "{}Received error: {} ({}){}",
                YELLOW, error.error.message, error.error.code, NC
            );
        }
        _ => {
            println!(
                "{}Received unexpected message type{}: {:?}",
                YELLOW, NC, message
            );
        }
    }

    Ok(())
}

/// Helper function to print a prompt and flush stdout
fn print_with_prompt(prompt: String) {
    print!("{}", prompt);
    io::stdout().flush().unwrap();
}
