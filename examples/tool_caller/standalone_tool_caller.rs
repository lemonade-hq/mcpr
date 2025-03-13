//! Standalone LLM Tool Caller Example
//!
//! This example demonstrates a simple HTTP server that accepts tool call requests from an LLM
//! and executes them directly, without requiring an MCP server.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::process::Command;
use std::sync::{Arc, Mutex};
use tiny_http::{Header, Method, Request, Response, Server};
use url::Url;

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

#[derive(Serialize, Deserialize, Clone)]
struct Tool {
    name: String,
    description: String,
    parameters: Vec<ToolParameter>,
}

#[derive(Serialize, Deserialize, Clone)]
struct ToolParameter {
    name: String,
    description: String,
    required: bool,
    #[serde(rename = "type")]
    param_type: String,
}

// Available tools
fn get_available_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "web_search".to_string(),
            description: "Search the web using the default browser".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "query".to_string(),
                    description: "The search query to use".to_string(),
                    required: true,
                    param_type: "string".to_string(),
                },
                ToolParameter {
                    name: "engine".to_string(),
                    description: "The search engine to use (google, bing, duckduckgo)".to_string(),
                    required: false,
                    param_type: "string".to_string(),
                },
            ],
        },
        Tool {
            name: "calculator".to_string(),
            description: "Perform a simple calculation".to_string(),
            parameters: vec![ToolParameter {
                name: "expression".to_string(),
                description: "The mathematical expression to evaluate".to_string(),
                required: true,
                param_type: "string".to_string(),
            }],
        },
    ]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}Standalone LLM Tool Caller Example{}", BLUE, NC);
    println!(
        "{}This example demonstrates a simple HTTP server that accepts tool call requests from an LLM.{}",
        BLUE, NC
    );

    // Create a shared state for the tools
    let tools = Arc::new(Mutex::new(get_available_tools()));

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
        handle_request(request, tools.clone(), api_key);
    }

    Ok(())
}

fn handle_request(mut request: Request, tools: Arc<Mutex<Vec<Tool>>>, api_key: &str) {
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

            // Get the available tools
            let available_tools = tools.lock().unwrap().clone();

            // Check if the tool exists
            let tool = match available_tools
                .iter()
                .find(|t| t.name == tool_call_request.tool_name)
            {
                Some(t) => t,
                None => {
                    send_error_response(
                        request,
                        404,
                        &format!("Tool not found: {}", tool_call_request.tool_name),
                    );
                    return;
                }
            };

            // Check required parameters
            for param in &tool.parameters {
                if param.required && !tool_call_request.parameters.contains_key(&param.name) {
                    send_error_response(
                        request,
                        400,
                        &format!("Missing required parameter: {}", param.name),
                    );
                    return;
                }
            }

            // Call the tool
            match tool.name.as_str() {
                "web_search" => {
                    // Get parameters
                    let query = match tool_call_request.parameters.get("query") {
                        Some(q) => match q.as_str() {
                            Some(s) => s,
                            None => {
                                send_error_response(request, 400, "Query must be a string");
                                return;
                            }
                        },
                        None => {
                            send_error_response(request, 400, "Missing required parameter: query");
                            return;
                        }
                    };

                    let engine = match tool_call_request.parameters.get("engine") {
                        Some(e) => match e.as_str() {
                            Some(s) => s,
                            None => "google",
                        },
                        None => "google",
                    };

                    // Perform the search
                    match perform_web_search(query, engine) {
                        Ok(result) => {
                            let response = ToolCallResponse {
                                success: true,
                                result: Some(serde_json::to_value(result).unwrap()),
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
                            send_error_response(
                                request,
                                500,
                                &format!("Failed to perform search: {}", e),
                            );
                        }
                    }
                }
                "calculator" => {
                    // Get parameters
                    let expression = match tool_call_request.parameters.get("expression") {
                        Some(e) => match e.as_str() {
                            Some(s) => s,
                            None => {
                                send_error_response(request, 400, "Expression must be a string");
                                return;
                            }
                        },
                        None => {
                            send_error_response(
                                request,
                                400,
                                "Missing required parameter: expression",
                            );
                            return;
                        }
                    };

                    // Perform the calculation
                    match evaluate_expression(expression) {
                        Ok(result) => {
                            let response = ToolCallResponse {
                                success: true,
                                result: Some(serde_json::json!({
                                    "expression": expression,
                                    "result": result
                                })),
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
                            send_error_response(
                                request,
                                400,
                                &format!("Failed to evaluate expression: {}", e),
                            );
                        }
                    }
                }
                _ => {
                    send_error_response(
                        request,
                        501,
                        &format!("Tool not implemented: {}", tool.name),
                    );
                }
            }
        }
        (Method::Get, "/tools") => {
            // Return the list of available tools
            let available_tools = tools.lock().unwrap().clone();
            let json = serde_json::to_string(&available_tools).unwrap();
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

// Tool implementations

fn perform_web_search(query: &str, engine: &str) -> Result<String, String> {
    // Build the search URL based on the engine
    let search_url = match engine {
        "google" => format!("https://www.google.com/search?q={}", url_encode(query)),
        "bing" => format!("https://www.bing.com/search?q={}", url_encode(query)),
        "duckduckgo" => format!("https://duckduckgo.com/?q={}", url_encode(query)),
        _ => format!("https://www.google.com/search?q={}", url_encode(query)),
    };

    println!(
        "{}Opening browser with search URL: {}{}",
        YELLOW, search_url, NC
    );

    // Open the default browser with the search URL
    #[cfg(target_os = "windows")]
    {
        println!("{}Using Windows 'cmd' to open browser{}", YELLOW, NC);
        Command::new("cmd")
            .args(["/c", "start", &search_url])
            .spawn()
            .map_err(|e| format!("Failed to open browser: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        println!("{}Using macOS 'open' command to open browser{}", YELLOW, NC);
        Command::new("open")
            .arg(&search_url)
            .spawn()
            .map_err(|e| format!("Failed to open browser: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        println!(
            "{}Using Linux 'xdg-open' command to open browser{}",
            YELLOW, NC
        );
        Command::new("xdg-open")
            .arg(&search_url)
            .spawn()
            .map_err(|e| format!("Failed to open browser: {}", e))?;
    }

    Ok(format!(
        "Search initiated for '{}' using {} engine. Browser opened with URL: {}",
        query, engine, search_url
    ))
}

fn evaluate_expression(expression: &str) -> Result<f64, String> {
    // This is a very simple expression evaluator
    // In a real application, you would use a proper expression parser
    let expression = expression.replace(" ", "");

    // Simple addition
    if expression.contains('+') {
        let parts: Vec<&str> = expression.split('+').collect();
        if parts.len() != 2 {
            return Err("Only simple expressions with two operands are supported".to_string());
        }

        let a = parts[0]
            .parse::<f64>()
            .map_err(|e| format!("Invalid number: {}", e))?;
        let b = parts[1]
            .parse::<f64>()
            .map_err(|e| format!("Invalid number: {}", e))?;

        return Ok(a + b);
    }

    // Simple subtraction
    if expression.contains('-') {
        let parts: Vec<&str> = expression.split('-').collect();
        if parts.len() != 2 {
            return Err("Only simple expressions with two operands are supported".to_string());
        }

        let a = parts[0]
            .parse::<f64>()
            .map_err(|e| format!("Invalid number: {}", e))?;
        let b = parts[1]
            .parse::<f64>()
            .map_err(|e| format!("Invalid number: {}", e))?;

        return Ok(a - b);
    }

    // Simple multiplication
    if expression.contains('*') {
        let parts: Vec<&str> = expression.split('*').collect();
        if parts.len() != 2 {
            return Err("Only simple expressions with two operands are supported".to_string());
        }

        let a = parts[0]
            .parse::<f64>()
            .map_err(|e| format!("Invalid number: {}", e))?;
        let b = parts[1]
            .parse::<f64>()
            .map_err(|e| format!("Invalid number: {}", e))?;

        return Ok(a * b);
    }

    // Simple division
    if expression.contains('/') {
        let parts: Vec<&str> = expression.split('/').collect();
        if parts.len() != 2 {
            return Err("Only simple expressions with two operands are supported".to_string());
        }

        let a = parts[0]
            .parse::<f64>()
            .map_err(|e| format!("Invalid number: {}", e))?;
        let b = parts[1]
            .parse::<f64>()
            .map_err(|e| format!("Invalid number: {}", e))?;

        if b == 0.0 {
            return Err("Division by zero".to_string());
        }

        return Ok(a / b);
    }

    // Just a number
    expression
        .parse::<f64>()
        .map_err(|e| format!("Invalid expression: {}", e))
}

/// Simple URL encoding function
fn url_encode(s: &str) -> String {
    let url = Url::parse(&format!("https://example.com/?q={}", s)).unwrap();
    url.query().unwrap().strip_prefix("q=").unwrap().to_string()
}
