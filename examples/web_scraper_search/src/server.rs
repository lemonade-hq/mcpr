mod common;

use anthropic::client::Client as AnthropicClient;
use anyhow::{Context, Result};
use clap::Parser;
use common::{ScrapeAndSearchRequest, ScrapeAndSearchResponse, TOOL_NAME};
use log::{error, info};
use mcpr::{
    error::MCPError,
    server::{Server, ServerConfig},
    transport::stdio::StdioTransport,
    Tool,
};
use reqwest::blocking::Client as HttpClient;
use scraper::{Html, Selector};
use serde_json::Value;
use std::{env, fs::File, io::Write};
use tempfile::TempDir;

#[derive(Parser, Debug)]
#[clap(author, version, about = "MCP Web Scraper Search Server")]
struct Args {
    /// Anthropic API key
    #[clap(long, env = "ANTHROPIC_API_KEY")]
    anthropic_api_key: Option<String>,
}

struct WebScraperSearch {
    http_client: HttpClient,
    anthropic_api_key: String,
    temp_dir: TempDir,
}

impl WebScraperSearch {
    fn new(anthropic_api_key: String) -> Result<Self> {
        let http_client = HttpClient::new();
        let temp_dir = TempDir::new().context("Failed to create temporary directory")?;

        Ok(Self {
            http_client,
            anthropic_api_key,
            temp_dir,
        })
    }

    fn scrape_webpage(&self, uri: &str) -> Result<String> {
        info!("Scraping webpage: {}", uri);

        // Download the webpage
        let response = self
            .http_client
            .get(uri)
            .send()
            .context("Failed to fetch webpage")?;
        let html = response.text().context("Failed to get response text")?;

        // Parse the HTML
        let document = Html::parse_document(&html);

        // Extract the text content
        let body_selector = Selector::parse("body").unwrap();
        let mut content = String::new();

        if let Some(body) = document.select(&body_selector).next() {
            content = body.text().collect::<Vec<_>>().join(" ");
        }

        // Save the content to a file in the temp directory
        let file_path = self.temp_dir.path().join("webpage.html");
        let mut file = File::create(&file_path).context("Failed to create temp file")?;
        file.write_all(html.as_bytes())
            .context("Failed to write to temp file")?;

        info!("Webpage scraped and saved to: {:?}", file_path);

        Ok(content)
    }

    fn search_with_llm(&self, content: &str, search_term: &str) -> Result<String> {
        info!("Searching for '{}' in the scraped content", search_term);

        // Truncate content if it's too long
        let max_content_length = 20000; // Adjust based on Claude's context window
        let truncated_content = if content.len() > max_content_length {
            &content[0..max_content_length]
        } else {
            content
        };

        // Create a prompt for Claude
        let prompt = format!(
            "I have scraped the content of a webpage. Please search for information about '{}' in the following content and provide a concise summary of what you find. If the information is not present, please state that clearly.\n\nWebpage content:\n{}",
            search_term, truncated_content
        );

        // Call Claude API using the anthropic crate
        let client = AnthropicClient::new(self.anthropic_api_key.clone());
        let completion = client
            .completions()
            .create("claude-2.0", &prompt, 100000)
            .context("Failed to get response from Claude")?;

        let result = completion.completion;

        info!("LLM search completed");

        Ok(result)
    }

    fn handle_scrape_and_search(
        &self,
        request: ScrapeAndSearchRequest,
    ) -> Result<ScrapeAndSearchResponse, MCPError> {
        info!(
            "Handling scrape and search request for URI: {}, search term: {}",
            request.uri, request.search_term
        );

        let result = match self.scrape_webpage(&request.uri) {
            Ok(content) => match self.search_with_llm(&content, &request.search_term) {
                Ok(search_result) => ScrapeAndSearchResponse {
                    uri: request.uri.clone(),
                    search_term: request.search_term.clone(),
                    result: search_result,
                    error: None,
                },
                Err(e) => ScrapeAndSearchResponse {
                    uri: request.uri.clone(),
                    search_term: request.search_term.clone(),
                    result: String::new(),
                    error: Some(format!("LLM search error: {}", e)),
                },
            },
            Err(e) => ScrapeAndSearchResponse {
                uri: request.uri.clone(),
                search_term: request.search_term.clone(),
                result: String::new(),
                error: Some(format!("Web scraping error: {}", e)),
            },
        };

        Ok(result)
    }
}

fn main() -> Result<()> {
    // Initialize logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    // Parse command line arguments
    let args = Args::parse();

    // Get Anthropic API key
    let anthropic_api_key = args.anthropic_api_key
        .or_else(|| env::var("ANTHROPIC_API_KEY").ok())
        .context("Anthropic API key not provided. Set ANTHROPIC_API_KEY environment variable or use --anthropic-api-key flag")?;

    // Create the web scraper search service
    let web_scraper = WebScraperSearch::new(anthropic_api_key)?;

    // Define the tool
    let tool = Tool {
        name: TOOL_NAME.to_string(),
        description: "Scrapes a webpage and searches for specific information using an LLM"
            .to_string(),
        parameters_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "uri": {
                    "type": "string",
                    "description": "The URI of the webpage to scrape"
                },
                "search_term": {
                    "type": "string",
                    "description": "The term to search for in the webpage content"
                }
            },
            "required": ["uri", "search_term"]
        }),
    };

    // Configure the server
    let server_config = ServerConfig::new()
        .with_name("Web Scraper Search Server")
        .with_version("1.0.0")
        .with_tool(tool);

    // Create the server
    let mut server = Server::new(server_config);

    // Register the tool handler
    let web_scraper_clone = web_scraper;
    server.register_tool_handler(TOOL_NAME, move |params: Value| {
        // Parse the parameters
        let request: ScrapeAndSearchRequest = serde_json::from_value(params)
            .map_err(|e| MCPError::Protocol(format!("Invalid parameters: {}", e)))?;

        // Handle the request
        let response = web_scraper_clone.handle_scrape_and_search(request)?;

        // Return the response
        Ok(serde_json::to_value(response).unwrap())
    })?;

    // Start the server
    info!("Starting MCP server");
    let transport = StdioTransport::new();
    server.start(transport)?;

    Ok(())
}
