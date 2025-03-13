//! Simple MCP server example

use mcpr::constants::LATEST_PROTOCOL_VERSION;
use mcpr::error::MCPError;
use mcpr::schema::{
    Implementation, InitializeResult, JSONRPCError, JSONRPCMessage, JSONRPCResponse,
    ServerCapabilities,
};
use mcpr::transport::{stdio::StdioTransport, Transport};

use serde_json::Value;

fn main() -> Result<(), MCPError> {
    // Create a stdio transport
    let mut transport = StdioTransport::new();

    // Wait for initialize request
    let message: JSONRPCMessage = transport.receive()?;

    match message {
        JSONRPCMessage::Request(request) => {
            if request.method == "initialize" {
                // Handle initialize request
                let result = InitializeResult {
                    protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
                    capabilities: ServerCapabilities {
                        experimental: None,
                        logging: Some(Value::Object(serde_json::Map::new())),
                        prompts: None,
                        resources: None,
                        tools: None,
                    },
                    server_info: Implementation {
                        name: "simple-server".to_string(),
                        version: "0.1.0".to_string(),
                    },
                    instructions: Some("This is a simple MCP server example.".to_string()),
                };

                // Send initialize response
                let response =
                    JSONRPCResponse::new(request.id, serde_json::to_value(result).unwrap());
                transport.send(&response)?;

                println!("Server initialized");

                // Main message loop
                loop {
                    let message: JSONRPCMessage = match transport.receive() {
                        Ok(msg) => msg,
                        Err(e) => {
                            eprintln!("Error receiving message: {}", e);
                            break;
                        }
                    };

                    match message {
                        JSONRPCMessage::Request(request) => {
                            // Handle request
                            match request.method.as_str() {
                                "ping" => {
                                    // Send empty response
                                    let response = JSONRPCResponse::new(
                                        request.id,
                                        Value::Object(serde_json::Map::new()),
                                    );
                                    transport.send(&response)?;
                                }
                                _ => {
                                    // Method not found
                                    let error = JSONRPCError::new(
                                        request.id,
                                        -32601,
                                        format!("Method not found: {}", request.method),
                                        None,
                                    );
                                    transport.send(&error)?;
                                }
                            }
                        }
                        JSONRPCMessage::Notification(notification) => {
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
                        }
                    }
                }
            } else {
                // Not an initialize request
                let error = JSONRPCError::new(
                    request.id,
                    -32600,
                    "Expected initialize request".to_string(),
                    None,
                );
                transport.send(&error)?;
            }
        }
        _ => {
            // Not a request
            eprintln!("Expected initialize request, got something else");
        }
    }

    Ok(())
}
