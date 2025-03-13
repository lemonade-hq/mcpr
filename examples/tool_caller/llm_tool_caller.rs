//! LLM Tool Caller Example
//!
//! This example demonstrates how an LLM can invoke tools through the MCP protocol.
//! It creates a simple HTTP server that accepts tool call requests from an LLM
//! and forwards them to an MCP server.

use mcpr::constants::LATEST_PROTOCOL_VERSION;
use mcpr::error::MCPError;
use mcpr::schema::{
    CallToolParams, ClientCapabilities, Implementation, InitializeParams, JSONRPCMessage,
    JSONRPCNotification, JSONRPCRequest, RequestId, Tool,
};
use mcpr::transport::{stdio::StdioTransport, Transport};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::io;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tiny_http::{Header, Method, Request, Response, Server};

// ANSI color codes for better terminal output
const GREEN: &str = "\x1b[0;32m";
const YELLOW: &str = "\x1b[1;33m";
const BLUE: &str = "\x1b[0;34m";
const RED: &str = "\x1b[0;31m";
const BOLD: &str = "\x1b[1m";
const NC: &str = "\x1b[0m"; // No Color

// Structs for API requests and responses
#[derive(Serialize, Deserialize)]
struct ToolCallRequest {
    tool_name: String,
    parameters: HashMap<String, Value>,
    api_key: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct ToolCallResponse {
    success: bool,
    result: Option<Value>,
    error: Option<String>,
}

// Main MCP client that will communicate with the MCP server
struct MCPClient {
    transport: StdioTransport,
    initialized: bool,
    tools: Vec<Tool>,
}

impl MCPClient {
    fn new() -> Self {
        Self {
            transport: StdioTransport::new(),
            initialized: false,
            tools: Vec::new(),
        }
    }

    fn initialize(&mut self) -> Result<(), MCPError> {
        println!("{}Initializing MCP client...{}", BLUE, NC);

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
                name: "llm-tool-caller".to_string(),
                version: "0.1.0".to_string(),
            },
        };

        let initialize_request = JSONRPCRequest::new(
            request_id.clone(),
            "initialize".to_string(),
            Some(serde_json::to_value(initialize_params).unwrap()),
        );

        println!("{}Sending initialize request{}", YELLOW, NC);
        self.transport.send(&initialize_request)?;

        // Wait for initialize response
        println!("{}Waiting for initialize response...{}", YELLOW, NC);
        let message: JSONRPCMessage = self.transport.receive()?;

        match message {
            JSONRPCMessage::Response(_) => {
                println!("{}Received initialize response{}", GREEN, NC);

                // Send initialized notification
                let initialized_notification =
                    JSONRPCNotification::new("notifications/initialized".to_string(), None);

                println!("{}Sending initialized notification{}", YELLOW, NC);
                self.transport.send(&initialized_notification)?;
                println!("{}Sent initialized notification{}", GREEN, NC);

                // Wait a moment to ensure the server processes the notification
                thread::sleep(Duration::from_millis(500));

                // Request available tools
                let tools_request =
                    JSONRPCRequest::new(RequestId::Number(2), "tools/list".to_string(), None);

                println!("{}Requesting available tools{}", YELLOW, NC);
                self.transport.send(&tools_request)?;

                // Wait for tools response
                println!("{}Waiting for tools response...{}", YELLOW, NC);
                let message: JSONRPCMessage = self.transport.receive()?;

                match message {
                    JSONRPCMessage::Response(response) => {
                        if let Some(tools_list) = response.result.get("tools") {
                            println!("{}Received tools list{}", GREEN, NC);
                            let tools: Vec<Tool> = serde_json::from_value(tools_list.clone())
                                .map_err(|e| {
                                    MCPError::Protocol(format!("Invalid tools list: {}", e))
                                })?;
                            self.tools = tools;
                            self.initialized = true;
                            println!("{}MCP client initialized successfully{}", GREEN, NC);
                            Ok(())
                        } else {
                            println!("{}No tools available in response{}", YELLOW, NC);
                            Err(MCPError::Protocol("No tools available".to_string()))
                        }
                    }
                    _ => Err(MCPError::Protocol(
                        "Unexpected response to tools/list".to_string(),
                    )),
                }
            }
            _ => Err(MCPError::Protocol(
                "Unexpected response to initialize".to_string(),
            )),
        }
    }

    fn call_tool(
        &mut self,
        tool_name: &str,
        parameters: HashMap<String, Value>,
    ) -> Result<Value, MCPError> {
        if !self.initialized {
            return Err(MCPError::Protocol("Client not initialized".to_string()));
        }

        // Check if the tool exists
        if !self.tools.iter().any(|t| t.name == tool_name) {
            return Err(MCPError::Protocol(format!("Tool not found: {}", tool_name)));
        }

        // Call the tool
        let call_params = CallToolParams {
            name: tool_name.to_string(),
            arguments: Some(parameters),
        };

        let call_request = JSONRPCRequest::new(
            RequestId::Number(3),
            "tools/call".to_string(),
            Some(serde_json::to_value(call_params).unwrap()),
        );

        println!("{}Calling tool: {}{}", YELLOW, tool_name, NC);
        self.transport.send(&call_request)?;

        // Wait for tool call response
        println!("{}Waiting for tool call response...{}", YELLOW, NC);
        let message: JSONRPCMessage = self.transport.receive()?;

        match message {
            JSONRPCMessage::Response(response) => {
                println!("{}Received tool call response{}", GREEN, NC);
                Ok(response.result)
            }
            JSONRPCMessage::Error(error) => Err(MCPError::Protocol(format!(
                "Tool call error: {} ({})",
                error.error.message, error.error.code
            ))),
            _ => Err(MCPError::Protocol(
                "Unexpected response to tool call".to_string(),
            )),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}LLM Tool Caller Example{}", BLUE, NC);
    println!(
        "{}This example demonstrates how an LLM can invoke tools through the MCP protocol.{}",
        BLUE, NC
    );

    // Create and initialize the MCP client
    let mcp_client = Arc::new(Mutex::new(MCPClient::new()));

    // Initialize the client
    {
        let mut client = mcp_client.lock().unwrap();
        client.initialize()?;
    }

    // Start the HTTP server
    let server = Server::http("127.0.0.1:8080").unwrap();
    println!(
        "{}HTTP server listening on http://127.0.0.1:8080{}",
        GREEN, NC
    );
    println!(
        "{}Send POST requests to /tool-call to invoke tools{}",
        GREEN, NC
    );

    // API key for simple authentication (in a real app, use a more secure approach)
    let api_key = "test-api-key";
    println!("{}Using API key: {}{}", YELLOW, api_key, NC);

    for request in server.incoming_requests() {
        handle_request(request, mcp_client.clone(), api_key);
    }

    Ok(())
}

fn handle_request(mut request: Request, mcp_client: Arc<Mutex<MCPClient>>, api_key: &str) {
    match (request.method(), request.url()) {
        (Method::Post, "/tool-call") => {
            // Read the request body
            let mut content = String::new();
            if let Err(e) = request.as_reader().read_to_string(&mut content) {
                send_error_response(request, 400, &format!("Failed to read request body: {}", e));
                return;
            }

            // Parse the request
            let tool_call_request: ToolCallRequest = match serde_json::from_str(&content) {
                Ok(req) => req,
                Err(e) => {
                    send_error_response(request, 400, &format!("Invalid JSON: {}", e));
                    return;
                }
            };

            // Check API key
            if let Some(provided_key) = &tool_call_request.api_key {
                if provided_key != api_key {
                    send_error_response(request, 401, "Invalid API key");
                    return;
                }
            } else {
                send_error_response(request, 401, "API key required");
                return;
            }

            // Call the tool
            let mut client = mcp_client.lock().unwrap();
            match client.call_tool(&tool_call_request.tool_name, tool_call_request.parameters) {
                Ok(result) => {
                    let response = ToolCallResponse {
                        success: true,
                        result: Some(result),
                        error: None,
                    };
                    let json = serde_json::to_string(&response).unwrap();
                    let content_type =
                        Header::from_bytes("Content-Type", "application/json").unwrap();
                    request
                        .respond(Response::from_string(json).with_header(content_type))
                        .unwrap();
                }
                Err(e) => {
                    let response = ToolCallResponse {
                        success: false,
                        result: None,
                        error: Some(format!("Tool call failed: {}", e)),
                    };
                    let json = serde_json::to_string(&response).unwrap();
                    let content_type =
                        Header::from_bytes("Content-Type", "application/json").unwrap();
                    request
                        .respond(Response::from_string(json).with_header(content_type))
                        .unwrap();
                }
            }
        }
        (Method::Get, "/tools") => {
            // Return the list of available tools
            let tools = {
                let client = mcp_client.lock().unwrap();
                client.tools.clone()
            };
            let json = serde_json::to_string(&tools).unwrap();
            let content_type = Header::from_bytes("Content-Type", "application/json").unwrap();
            request
                .respond(Response::from_string(json).with_header(content_type))
                .unwrap();
        }
        _ => {
            send_error_response(request, 404, "Not found");
        }
    }
}

fn send_error_response(request: Request, status_code: u16, message: &str) {
    let response = ToolCallResponse {
        success: false,
        result: None,
        error: Some(message.to_string()),
    };
    let json = serde_json::to_string(&response).unwrap();
    let content_type = Header::from_bytes("Content-Type", "application/json").unwrap();
    request
        .respond(
            Response::from_string(json)
                .with_status_code(status_code)
                .with_header(content_type),
        )
        .unwrap();
}
