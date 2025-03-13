//! MCP Web Scraper Server with TCP Socket
//!
//! This example demonstrates an MCP server that provides a web scraping tool
//! using Claude to extract information from web pages based on user queries.
//! It uses a TCP socket for communication.

use dotenv::dotenv;
use mcpr::error::MCPError;
use mcpr::schema::{
    CallToolParams, CallToolResult, Implementation, InitializeResult, JSONRPCError, JSONRPCMessage,
    JSONRPCResponse, ServerCapabilities, TextContent, Tool, ToolInputSchema, ToolResultContent,
    ToolsCapability,
};
use reqwest::Client as HttpClient;
use scraper::{Html, Selector};
use serde_json::{json, Value};
use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;

// ANSI color codes for better terminal output
const GREEN: &str = "\x1b[0;32m";
const YELLOW: &str = "\x1b[1;33m";
const BLUE: &str = "\x1b[0;34m";
const RED: &str = "\x1b[0;31m";
const NC: &str = "\x1b[0m"; // No Color

// Anthropic API constants
const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_MODEL: &str = "claude-3-opus-20240229";
const MAX_TOKENS: u32 = 4096;

// Custom error type
#[derive(Debug)]
struct WebScraperError(String);

impl fmt::Display for WebScraperError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for WebScraperError {}

impl From<String> for WebScraperError {
    fn from(error: String) -> Self {
        WebScraperError(error)
    }
}

impl From<&str> for WebScraperError {
    fn from(error: &str) -> Self {
        WebScraperError(error.to_string())
    }
}

impl From<reqwest::Error> for WebScraperError {
    fn from(error: reqwest::Error) -> Self {
        WebScraperError(error.to_string())
    }
}

impl From<std::io::Error> for WebScraperError {
    fn from(error: std::io::Error) -> Self {
        WebScraperError(error.to_string())
    }
}

impl From<env::VarError> for WebScraperError {
    fn from(error: env::VarError) -> Self {
        WebScraperError(error.to_string())
    }
}

struct WebScraper {
    http_client: HttpClient,
    anthropic_key: String,
}

impl WebScraper {
    fn new(anthropic_key: String) -> Self {
        Self {
            http_client: HttpClient::new(),
            anthropic_key,
        }
    }

    async fn scrape_url(&self, url: &str) -> Result<String, WebScraperError> {
        println!("{}Scraping URL: {}{}", YELLOW, url, NC);

        // Fetch the HTML content
        let response = self.http_client.get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Failed to fetch URL: HTTP {}", response.status()).into());
        }

        let html = response.text().await?;
        println!(
            "{}Successfully fetched HTML content ({} bytes){}",
            GREEN,
            html.len(),
            NC
        );

        // Save HTML to the specified directory or /tmp as fallback
        let html_dir = env::var("HTML_DIR").unwrap_or_else(|_| "/tmp".to_string());
        let file_name = format!("scraped_{}.html", chrono::Utc::now().timestamp());
        let file_path = Path::new(&html_dir).join(&file_name);

        fs::write(&file_path, &html)?;
        println!("{}HTML saved to {}{}", GREEN, file_path.display(), NC);

        Ok(html)
    }

    async fn extract_content(
        &self,
        html: &str,
        query: &str,
        css_selector: Option<&str>,
    ) -> Result<String, WebScraperError> {
        println!(
            "{}Parsing HTML and extracting content for query: {}{}",
            YELLOW, query, NC
        );

        // Parse the HTML
        let document = Html::parse_document(html);

        // Extract the title
        let title_selector = Selector::parse("title").unwrap();
        let title = document
            .select(&title_selector)
            .next()
            .map(|element| element.inner_html())
            .unwrap_or_else(|| "No title found".to_string());

        // Extract specific content if CSS selector is provided
        let specific_content = if let Some(selector_str) = css_selector {
            match Selector::parse(selector_str) {
                Ok(selector) => {
                    println!("{}Using CSS selector: {}{}", GREEN, selector_str, NC);
                    let selected_content: Vec<String> = document
                        .select(&selector)
                        .map(|element| element.html())
                        .collect();

                    if selected_content.is_empty() {
                        println!(
                            "{}No elements found with selector: {}{}",
                            YELLOW, selector_str, NC
                        );
                        None
                    } else {
                        println!(
                            "{}Found {} elements with selector{}",
                            GREEN,
                            selected_content.len(),
                            NC
                        );
                        Some(selected_content.join("\n"))
                    }
                }
                Err(e) => {
                    println!(
                        "{}Invalid CSS selector: {} - {}{}",
                        RED, selector_str, e, NC
                    );
                    None
                }
            }
        } else {
            None
        };

        // Extract the main content if no specific content was selected
        let body_text = if specific_content.is_none() {
            let body_selector = Selector::parse("body").unwrap();
            document
                .select(&body_selector)
                .next()
                .map(|element| element.text().collect::<Vec<_>>().join(" "))
                .unwrap_or_else(|| "No body content found".to_string())
        } else {
            "".to_string()
        };

        // Determine which content to use for extraction
        let content_to_analyze = if let Some(content) = specific_content {
            println!("{}Using selected content for analysis{}", GREEN, NC);
            content
        } else {
            println!("{}Using full body content for analysis{}", GREEN, NC);
            body_text
        };

        // Truncate the content if it's too long
        let truncated_content = if content_to_analyze.len() > 100000 {
            format!("{}... (truncated)", &content_to_analyze[0..100000])
        } else {
            content_to_analyze
        };

        // Use Anthropic's Claude to extract relevant information
        let extracted_content = self
            .query_anthropic(&title, &truncated_content, query)
            .await?;

        Ok(extracted_content)
    }

    async fn query_anthropic(
        &self,
        title: &str,
        content: &str,
        query: &str,
    ) -> Result<String, WebScraperError> {
        println!(
            "{}Querying Anthropic API to extract relevant information{}",
            YELLOW, NC
        );

        let prompt = format!(
            "I need you to extract specific information from this web page based on the user's query.\n\n\
            Page Title: {}\n\n\
            User Query: {}\n\n\
            Web Page Content:\n{}\n\n\
            Please extract and summarize only the information that is relevant to the user's query. \
            Focus on providing accurate, concise, and helpful information. \
            If the requested information is not found in the content, please state that clearly.",
            title, query, content
        );

        let request_body = json!({
            "model": ANTHROPIC_MODEL,
            "max_tokens": MAX_TOKENS,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        });

        // Send the request to Anthropic API
        let response = self
            .http_client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.anthropic_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        // Check the status code first
        let status = response.status();
        if !status.is_success() {
            // Get the error text from the response
            let error_text = response.text().await?;
            return Err(format!("Anthropic API error: {} - {}", status, error_text).into());
        }

        // Parse the successful response
        let response_json: Value = response.json().await?;
        let extracted_content = response_json["content"][0]["text"]
            .as_str()
            .ok_or_else(|| WebScraperError("Failed to parse Anthropic API response".to_string()))?
            .to_string();

        println!(
            "{}Successfully extracted relevant content using Anthropic API{}",
            GREEN, NC
        );

        Ok(extracted_content)
    }
}

// TCP transport for MCP communication
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

fn handle_client(mut transport: TCPTransport) -> Result<(), MCPError> {
    // Load environment variables from .env file
    dotenv().ok();

    // Get Anthropic API key from environment
    let anthropic_key = match env::var("ANTHROPIC_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!(
                "{}Error: ANTHROPIC_API_KEY environment variable not set{}",
                RED, NC
            );
            return Err(MCPError::Protocol(
                "ANTHROPIC_API_KEY environment variable not set".to_string(),
            ));
        }
    };

    // Create a tokio runtime
    let runtime = Runtime::new().unwrap();

    // Create the web scraper
    let scraper = Arc::new(Mutex::new(WebScraper::new(anthropic_key)));

    println!(
        "{}MCP Web Scraper Server started. Client connected.{}",
        BLUE, NC
    );

    // Wait for initialize request
    println!("{}Waiting for initialize request...{}", YELLOW, NC);
    let message: JSONRPCMessage = match transport.receive() {
        Ok(msg) => {
            println!("{}Received message: {:?}{}", GREEN, msg, NC);
            msg
        }
        Err(e) => {
            eprintln!("{}Error receiving message: {}{}", RED, e, NC);
            return Err(e);
        }
    };

    match message {
        JSONRPCMessage::Request(request) => {
            println!("{}Received request: {}{}", GREEN, request.method, NC);
            if request.method == "initialize" {
                // Define our web scrape tool
                let web_scrape_tool = Tool {
                    name: "web_scrape".to_string(),
                    description: Some("Scrape a web page and extract content based on a query".to_string()),
                    input_schema: ToolInputSchema {
                        r#type: "object".to_string(),
                        properties: Some(
                            [
                                (
                                    "url".to_string(),
                                    json!({
                                        "type": "string",
                                        "description": "The URL of the web page to scrape"
                                    }),
                                ),
                                (
                                    "query".to_string(),
                                    json!({
                                        "type": "string",
                                        "description": "The query to extract specific content from the web page"
                                    }),
                                ),
                                (
                                    "css_selector".to_string(),
                                    json!({
                                        "type": "string",
                                        "description": "Optional CSS selector to target specific elements (e.g., 'div.content', 'article', '#main')"
                                    }),
                                ),
                            ]
                            .into_iter()
                            .collect(),
                        ),
                        required: Some(vec!["url".to_string(), "query".to_string()]),
                    },
                };

                // Handle initialize request
                let result = InitializeResult {
                    protocol_version: mcpr::constants::LATEST_PROTOCOL_VERSION.to_string(),
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
                        name: "mcp-web-scraper-server".to_string(),
                        version: "0.1.0".to_string(),
                    },
                    instructions: Some(
                        "This server provides a tool to scrape web pages and extract content based on queries."
                            .to_string(),
                    ),
                };

                // Send initialize response
                let response =
                    JSONRPCResponse::new(request.id, serde_json::to_value(result).unwrap());
                println!("{}Sending initialize response{}", YELLOW, NC);
                transport.send(&JSONRPCMessage::Response(response))?;

                println!("{}Server initialized{}", GREEN, NC);

                // Main message loop
                loop {
                    println!("{}Waiting for next message...{}", YELLOW, NC);
                    let message: JSONRPCMessage = match transport.receive() {
                        Ok(msg) => {
                            println!("{}Received message: {:?}{}", GREEN, msg, NC);
                            msg
                        }
                        Err(e) => {
                            eprintln!("{}Error receiving message: {}{}", RED, e, NC);
                            break;
                        }
                    };

                    match message {
                        JSONRPCMessage::Request(request) => {
                            println!("{}Received request: {}{}", GREEN, request.method, NC);
                            // Handle request
                            match request.method.as_str() {
                                "tools/list" => {
                                    // Return our web scrape tool
                                    let response = JSONRPCResponse::new(
                                        request.id,
                                        json!({
                                            "tools": [web_scrape_tool]
                                        }),
                                    );
                                    println!("{}Sending tools list response{}", YELLOW, NC);
                                    transport.send(&JSONRPCMessage::Response(response))?;
                                }
                                "tools/call" => {
                                    // Handle tool call
                                    if let Some(params) = request.params {
                                        println!("{}Tool call params: {}{}", GREEN, params, NC);
                                        let params: CallToolParams =
                                            match serde_json::from_value(params) {
                                                Ok(p) => p,
                                                Err(e) => {
                                                    eprintln!(
                                                        "{}Invalid tool call parameters: {}{}",
                                                        RED, e, NC
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
                                                    transport
                                                        .send(&JSONRPCMessage::Error(error))?;
                                                    continue;
                                                }
                                            };

                                        if params.name == "web_scrape" {
                                            // Extract parameters
                                            let url = match params
                                                .arguments
                                                .as_ref()
                                                .and_then(|args| args.get("url"))
                                                .and_then(|u| u.as_str())
                                            {
                                                Some(u) => u,
                                                None => {
                                                    eprintln!(
                                                        "{}Missing required 'url' parameter{}",
                                                        RED, NC
                                                    );
                                                    let error = JSONRPCError::new(
                                                        request.id,
                                                        -32602,
                                                        "Missing required 'url' parameter"
                                                            .to_string(),
                                                        None,
                                                    );
                                                    transport
                                                        .send(&JSONRPCMessage::Error(error))?;
                                                    continue;
                                                }
                                            };

                                            let query = match params
                                                .arguments
                                                .as_ref()
                                                .and_then(|args| args.get("query"))
                                                .and_then(|q| q.as_str())
                                            {
                                                Some(q) => q,
                                                None => {
                                                    eprintln!(
                                                        "{}Missing required 'query' parameter{}",
                                                        RED, NC
                                                    );
                                                    let error = JSONRPCError::new(
                                                        request.id,
                                                        -32602,
                                                        "Missing required 'query' parameter"
                                                            .to_string(),
                                                        None,
                                                    );
                                                    transport
                                                        .send(&JSONRPCMessage::Error(error))?;
                                                    continue;
                                                }
                                            };

                                            // Extract optional CSS selector
                                            let css_selector = params
                                                .arguments
                                                .as_ref()
                                                .and_then(|args| args.get("css_selector"))
                                                .and_then(|s| s.as_str());

                                            println!(
                                                "{}Performing web scrape: url='{}', query='{}', css_selector='{:?}'{}",
                                                YELLOW, url, query, css_selector, NC
                                            );

                                            // Perform the web scrape
                                            let scraper_clone = scraper.clone();
                                            let result = runtime.block_on(async move {
                                                let scraper = scraper_clone.lock().await;

                                                // Step 1: Scrape the URL
                                                let html = match scraper.scrape_url(url).await {
                                                    Ok(html) => html,
                                                    Err(e) => {
                                                        return Err(format!(
                                                            "Error scraping URL: {}",
                                                            e
                                                        ))
                                                    }
                                                };

                                                // Step 2: Extract content based on query and optional CSS selector
                                                match scraper
                                                    .extract_content(&html, query, css_selector)
                                                    .await
                                                {
                                                    Ok(content) => Ok(content),
                                                    Err(e) => Err(format!(
                                                        "Error extracting content: {}",
                                                        e
                                                    )),
                                                }
                                            });

                                            match result {
                                                Ok(content) => {
                                                    // Return the result
                                                    let result = CallToolResult {
                                                        content: vec![ToolResultContent::Text(
                                                            TextContent {
                                                                r#type: "text".to_string(),
                                                                text: content,
                                                                annotations: None,
                                                            },
                                                        )],
                                                        is_error: None,
                                                    };

                                                    let response = JSONRPCResponse::new(
                                                        request.id,
                                                        serde_json::to_value(result).unwrap(),
                                                    );
                                                    println!(
                                                        "{}Sending web scrape result response{}",
                                                        YELLOW, NC
                                                    );
                                                    transport.send(&JSONRPCMessage::Response(
                                                        response,
                                                    ))?;
                                                }
                                                Err(e) => {
                                                    eprintln!(
                                                        "{}Error performing web scrape: {}{}",
                                                        RED, e, NC
                                                    );
                                                    let error = JSONRPCError::new(
                                                        request.id,
                                                        -32603,
                                                        format!(
                                                            "Error performing web scrape: {}",
                                                            e
                                                        ),
                                                        None,
                                                    );
                                                    transport
                                                        .send(&JSONRPCMessage::Error(error))?;
                                                }
                                            }
                                        } else {
                                            // Unknown tool
                                            eprintln!("{}Unknown tool: {}{}", RED, params.name, NC);
                                            let error = JSONRPCError::new(
                                                request.id,
                                                -32601,
                                                format!("Unknown tool: {}", params.name),
                                                None,
                                            );
                                            transport.send(&JSONRPCMessage::Error(error))?;
                                        }
                                    } else {
                                        // Missing parameters
                                        eprintln!("{}Missing tool call parameters{}", RED, NC);
                                        let error = JSONRPCError::new(
                                            request.id,
                                            -32602,
                                            "Missing tool call parameters".to_string(),
                                            None,
                                        );
                                        transport.send(&JSONRPCMessage::Error(error))?;
                                    }
                                }
                                "ping" => {
                                    // Send empty response
                                    let response = JSONRPCResponse::new(
                                        request.id,
                                        Value::Object(serde_json::Map::new()),
                                    );
                                    println!("{}Sending ping response{}", YELLOW, NC);
                                    transport.send(&JSONRPCMessage::Response(response))?;
                                }
                                _ => {
                                    // Method not found
                                    eprintln!("{}Method not found: {}{}", RED, request.method, NC);
                                    let error = JSONRPCError::new(
                                        request.id,
                                        -32601,
                                        format!("Method not found: {}", request.method),
                                        None,
                                    );
                                    transport.send(&JSONRPCMessage::Error(error))?;
                                }
                            }
                        }
                        JSONRPCMessage::Notification(notification) => {
                            println!(
                                "{}Received notification: {}{}",
                                GREEN, notification.method, NC
                            );
                            // Handle notification
                            match notification.method.as_str() {
                                "notifications/initialized" => {
                                    println!("{}Client initialized{}", GREEN, NC);
                                }
                                _ => {
                                    println!(
                                        "{}Received notification: {}{}",
                                        GREEN, notification.method, NC
                                    );
                                }
                            }
                        }
                        _ => {
                            // Ignore other message types
                            println!("{}Received other message type{}", YELLOW, NC);
                        }
                    }
                }
            } else {
                // Not an initialize request
                eprintln!(
                    "{}Expected initialize request, got: {}{}",
                    RED, request.method, NC
                );
                let error = JSONRPCError::new(
                    request.id,
                    -32600,
                    "Expected initialize request".to_string(),
                    None,
                );
                transport.send(&JSONRPCMessage::Error(error))?;
            }
        }
        _ => {
            // Not a request
            eprintln!(
                "{}Expected initialize request, got something else: {:?}{}",
                RED, message, NC
            );
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    // Load environment variables from .env file
    dotenv().ok();

    // Check if ANTHROPIC_API_KEY is set
    if env::var("ANTHROPIC_API_KEY").is_err() {
        eprintln!(
            "{}Error: ANTHROPIC_API_KEY environment variable not set{}",
            RED, NC
        );
        return Err("ANTHROPIC_API_KEY environment variable not set".into());
    }

    // Start TCP server on port 8081 instead of 8080
    let listener = TcpListener::bind("127.0.0.1:8081")?;
    println!(
        "{}MCP Web Scraper Server started on 127.0.0.1:8081{}",
        BLUE, NC
    );

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("{}New client connected{}", GREEN, NC);
                let transport = TCPTransport::new(stream);
                if let Err(e) = handle_client(transport) {
                    eprintln!("{}Error handling client: {}{}", RED, e, NC);
                }
            }
            Err(e) => {
                eprintln!("{}Error accepting connection: {}{}", RED, e, NC);
            }
        }
    }

    Ok(())
}
