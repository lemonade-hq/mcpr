//! Direct Web Scraper Example
//!
//! This example demonstrates a direct web scraper that uses Claude to extract
//! information from web pages based on user queries.

use dotenv::dotenv;
use reqwest::Client as HttpClient;
use scraper::{Html, Selector};
use serde_json::{json, Value};
use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;
use tokio::runtime::Runtime;

// ANSI color codes for better terminal output
const GREEN: &str = "\x1b[0;32m";
const YELLOW: &str = "\x1b[1;33m";
const BLUE: &str = "\x1b[0;34m";
const RED: &str = "\x1b[0;31m";
const CYAN: &str = "\x1b[0;36m";
const BOLD: &str = "\x1b[1m";
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

impl From<io::Error> for WebScraperError {
    fn from(error: io::Error) -> Self {
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Load environment variables from .env file
    dotenv().ok();

    println!("{}{}Direct Web Scraper{}{}", BOLD, BLUE, NC, NC);
    println!(
        "{}This tool allows you to scrape web content and extract information using Claude.{}",
        CYAN, NC
    );

    // Get Anthropic API key from environment
    let anthropic_key = match env::var("ANTHROPIC_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!(
                "{}Error: ANTHROPIC_API_KEY environment variable not set{}",
                RED, NC
            );
            eprintln!(
                "{}Please set it using: export ANTHROPIC_API_KEY=your_api_key{}",
                YELLOW, NC
            );
            return Err(Box::<dyn Error>::from(WebScraperError(
                "ANTHROPIC_API_KEY environment variable not set".into(),
            )));
        }
    };

    // Create the web scraper
    let scraper = WebScraper::new(anthropic_key);

    // Parse command-line arguments
    let args: Vec<String> = env::args().collect();
    let mut url = String::new();
    let mut query = String::new();
    let mut css_selector = String::new();

    // Check for command-line arguments
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--url" => {
                if i + 1 < args.len() {
                    url = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("{}Error: Missing value for --url{}", RED, NC);
                    return Err(Box::<dyn Error>::from(WebScraperError(
                        "Missing value for --url".into(),
                    )));
                }
            }
            "--query" => {
                if i + 1 < args.len() {
                    query = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("{}Error: Missing value for --query{}", RED, NC);
                    return Err(Box::<dyn Error>::from(WebScraperError(
                        "Missing value for --query".into(),
                    )));
                }
            }
            "--css-selector" => {
                if i + 1 < args.len() {
                    css_selector = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("{}Error: Missing value for --css-selector{}", RED, NC);
                    return Err(Box::<dyn Error>::from(WebScraperError(
                        "Missing value for --css-selector".into(),
                    )));
                }
            }
            _ => {
                i += 1;
            }
        }
    }

    // If no URL or query is provided via command line, prompt the user
    if url.is_empty() || query.is_empty() {
        let stdin = io::stdin();
        let mut stdin_lock = stdin.lock();
        let mut input = String::new();

        if url.is_empty() {
            print!("{}Enter the URL to scrape: {}", YELLOW, NC);
            io::stdout().flush().unwrap();
            input.clear();
            stdin_lock.read_line(&mut input).unwrap();
            url = input.trim().to_string();

            if url.is_empty() {
                println!("{}URL cannot be empty. Exiting.{}", RED, NC);
                return Err(Box::<dyn Error>::from(WebScraperError(
                    "URL cannot be empty".into(),
                )));
            }
        }

        if css_selector.is_empty() {
            print!(
                "{}Enter CSS selector (optional, e.g., 'div.content', 'article', '#main'): {}",
                YELLOW, NC
            );
            io::stdout().flush().unwrap();
            input.clear();
            stdin_lock.read_line(&mut input).unwrap();
            css_selector = input.trim().to_string();
        }

        if query.is_empty() {
            print!(
                "{}Enter your query (what information are you looking for?): {}",
                YELLOW, NC
            );
            io::stdout().flush().unwrap();
            input.clear();
            stdin_lock.read_line(&mut input).unwrap();
            query = input.trim().to_string();

            if query.is_empty() {
                println!("{}Query cannot be empty. Exiting.{}", RED, NC);
                return Err(Box::<dyn Error>::from(WebScraperError(
                    "Query cannot be empty".into(),
                )));
            }
        }
    }

    // Scrape the URL
    let html = match scraper.scrape_url(&url).await {
        Ok(html) => html,
        Err(e) => {
            eprintln!("{}Error scraping URL: {}{}", RED, e, NC);
            return Err(Box::<dyn Error>::from(e));
        }
    };

    // Extract content based on query and optional CSS selector
    let css_selector_option = if css_selector.is_empty() {
        None
    } else {
        Some(css_selector.as_str())
    };
    let extracted_content = match scraper
        .extract_content(&html, &query, css_selector_option)
        .await
    {
        Ok(content) => content,
        Err(e) => {
            eprintln!("{}Error extracting content: {}{}", RED, e, NC);
            return Err(Box::<dyn Error>::from(e));
        }
    };

    // Display the result
    println!("\n{}{}Extracted Content:{}{}", BOLD, GREEN, NC, NC);
    println!("{}", extracted_content);

    Ok(())
}
