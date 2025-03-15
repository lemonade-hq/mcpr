use crate::tools::GitHubTool;
use anyhow::{anyhow, Result};
use base64::decode;
use log::{debug, info, warn};
use mcpr::schema::common::{Tool, ToolInputSchema};
use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;

pub struct ReadmeQueryTool {
    http_client: Client,
    anthropic_api_key: Option<String>,
    use_fallback: bool,
}

impl ReadmeQueryTool {
    pub fn new() -> Self {
        // Load environment variables from .env file if it exists
        let _ = dotenv::dotenv();

        // Initialize HTTP client
        let http_client = Client::builder()
            .user_agent("github-tools-server")
            .build()
            .expect("Failed to create HTTP client");

        // Get Anthropic API key if available
        let anthropic_api_key = env::var("ANTHROPIC_API_KEY").ok();
        let use_fallback = anthropic_api_key.is_none();

        if use_fallback {
            warn!("ANTHROPIC_API_KEY not found, using fallback keyword-based approach");
        } else {
            info!("Using Anthropic API for README queries");
        }

        Self {
            http_client,
            anthropic_api_key,
            use_fallback,
        }
    }

    fn fetch_github_readme(&self, repo: &str) -> Result<String> {
        info!("Fetching README for repository: {}", repo);

        // GitHub API URL for the README
        let url = format!("https://api.github.com/repos/{}/readme", repo);

        // Send the request
        let response = self.http_client.get(&url).send()?;

        // Check if the request was successful
        if !response.status().is_success() {
            return Err(anyhow!(
                "GitHub API returned status code: {}",
                response.status()
            ));
        }

        // Parse the response
        let response_json: Value = response.json()?;

        // Extract the content (base64 encoded)
        let content = response_json["content"]
            .as_str()
            .ok_or_else(|| anyhow!("Content field not found in GitHub API response"))?;

        // Decode the content
        let content = content.replace("\n", "");
        let decoded_bytes = decode(&content)?;
        let decoded_content = String::from_utf8(decoded_bytes)?;

        Ok(decoded_content)
    }

    fn query_anthropic(&self, readme: &str, query: &str) -> Result<String> {
        info!("Querying Anthropic API with question: {}", query);

        let api_key = self
            .anthropic_api_key
            .as_ref()
            .ok_or_else(|| anyhow!("Anthropic API key not available"))?;

        let system_prompt = "You are a helpful assistant that answers questions about GitHub repositories based on their README content. \
                            Provide accurate, concise answers based only on the information in the README. \
                            If the README doesn't contain information to answer the question, say so clearly.";

        let user_prompt = format!(
            "README CONTENT:\n\n{}\n\nQUESTION: {}\n\nPlease answer the question based on the README content.",
            readme, query
        );

        // Call Anthropic API directly using reqwest
        let response = self
            .http_client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&json!({
                "model": "claude-3-sonnet-20240229",
                "max_tokens": 1000,
                "system": system_prompt,
                "messages": [
                    {
                        "role": "user",
                        "content": user_prompt
                    }
                ]
            }))
            .send()?;

        // Check if the request was successful
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text()?;
            return Err(anyhow!(
                "Anthropic API returned error: {} - {}",
                status,
                error_text
            ));
        }

        let response_json: Value = response.json()?;
        let content = response_json["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|item| item["text"].as_str())
            .ok_or_else(|| anyhow!("Failed to extract answer from Anthropic API response"))?;

        Ok(content.to_string())
    }

    /// Generate an answer based on the README content and query using a simple keyword-based approach
    fn generate_answer_with_keywords(&self, readme: &str, query: &str) -> String {
        info!(
            "Generating answer for query using keyword-based approach: {}",
            query
        );

        // Convert to lowercase for case-insensitive matching
        let query_lower = query.to_lowercase();

        // Extract keywords from the query (simple approach)
        let keywords: Vec<&str> = query_lower
            .split_whitespace()
            .filter(|word| {
                word.len() > 3
                    && ![
                        "what", "where", "when", "how", "does", "this", "that", "with", "about",
                    ]
                    .contains(word)
            })
            .collect();

        // Find paragraphs containing keywords
        let paragraphs: Vec<&str> = readme.split("\n\n").collect();
        let mut relevant_paragraphs = Vec::new();

        for paragraph in paragraphs {
            let paragraph_lower = paragraph.to_lowercase();
            let mut keyword_matches = 0;

            for keyword in &keywords {
                if paragraph_lower.contains(keyword) {
                    keyword_matches += 1;
                }
            }

            if keyword_matches > 0 {
                relevant_paragraphs.push((paragraph, keyword_matches));
            }
        }

        // Sort by number of keyword matches (descending)
        relevant_paragraphs.sort_by(|a, b| b.1.cmp(&a.1));

        // Generate the answer
        if relevant_paragraphs.is_empty() {
            return format!(
                "I couldn't find information about '{}' in the README.",
                query
            );
        } else {
            let top_paragraphs: Vec<&str> = relevant_paragraphs
                .iter()
                .take(2) // Take top 2 most relevant paragraphs
                .map(|(p, _)| *p)
                .collect();

            return format!(
                "Based on the README, here's what I found about '{}':\n\n{}",
                query,
                top_paragraphs.join("\n\n")
            );
        }
    }
}

impl GitHubTool for ReadmeQueryTool {
    fn get_tool_definition(&self) -> Tool {
        // Create properties for the input schema
        let mut properties = HashMap::new();

        // Add repo property
        properties.insert(
            "repo".to_string(),
            json!({
                "type": "string",
                "description": "Repository in format owner/repo"
            }),
        );

        // Add query property
        properties.insert(
            "query".to_string(),
            json!({
                "type": "string",
                "description": "Your question about the project"
            }),
        );

        // Create required fields
        let required = vec!["repo".to_string(), "query".to_string()];

        Tool {
            name: "readme_query".to_string(),
            description: Some(
                "Ask questions about a GitHub project based on its README content".to_string(),
            ),
            input_schema: ToolInputSchema {
                r#type: "object".to_string(),
                properties: Some(properties),
                required: Some(required),
            },
        }
    }

    fn handle(&self, params: Value) -> Result<Value> {
        // Extract parameters
        let repo = params["repo"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing 'repo' parameter"))?;
        let query = params["query"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing 'query' parameter"))?;

        // Fetch README from GitHub
        let readme = self.fetch_github_readme(repo)?;
        debug!("README fetched successfully ({} characters)", readme.len());

        // Generate answer
        let answer = if self.use_fallback {
            // Use keyword-based approach if Anthropic API key is not available
            self.generate_answer_with_keywords(&readme, query)
        } else {
            // Use Anthropic API
            match self.query_anthropic(&readme, query) {
                Ok(response) => response,
                Err(e) => {
                    warn!(
                        "Error calling Anthropic API: {}, falling back to keyword-based approach",
                        e
                    );
                    self.generate_answer_with_keywords(&readme, query)
                }
            }
        };

        info!("Generated answer for query");

        // Return the result
        Ok(json!({
            "answer": answer,
            "repository": repo,
            "query": query
        }))
    }
}
