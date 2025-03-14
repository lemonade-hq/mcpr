use serde::{Deserialize, Serialize};

// MCP message types
pub const TOOL_NAME: &str = "web_scraper_search";

// Request to scrape a webpage and search for information
#[derive(Debug, Serialize, Deserialize)]
pub struct ScrapeAndSearchRequest {
    pub uri: String,
    pub search_term: String,
}

// Response with the search results
#[derive(Debug, Serialize, Deserialize)]
pub struct ScrapeAndSearchResponse {
    pub uri: String,
    pub search_term: String,
    pub result: String,
    pub error: Option<String>,
}

// Tool definition for the MCP protocol
#[derive(Debug, Serialize, Deserialize)]
pub struct WebScraperSearchTool {
    pub name: String,
    pub description: String,
}

impl Default for WebScraperSearchTool {
    fn default() -> Self {
        Self {
            name: TOOL_NAME.to_string(),
            description: "Scrapes a webpage and searches for specific information using an LLM"
                .to_string(),
        }
    }
}
