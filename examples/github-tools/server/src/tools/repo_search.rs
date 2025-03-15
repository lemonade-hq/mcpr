use crate::tools::GitHubTool;
use anyhow::{anyhow, Result};
use log::{debug, info};
use mcpr::schema::common::{Tool, ToolInputSchema};
use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::collections::HashMap;

pub struct RepoSearchTool {
    http_client: Client,
}

impl RepoSearchTool {
    pub fn new() -> Self {
        // Initialize HTTP client
        let http_client = Client::builder()
            .user_agent("github-tools-server")
            .build()
            .expect("Failed to create HTTP client");

        Self { http_client }
    }

    fn search_repositories(&self, query: &str, limit: usize) -> Result<Vec<Value>> {
        info!("Searching GitHub repositories with query: {}", query);

        // GitHub API URL for repository search
        let url = format!(
            "https://api.github.com/search/repositories?q={}&sort=stars&order=desc&per_page={}",
            query, limit
        );

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

        // Extract the items
        let items = response_json["items"]
            .as_array()
            .ok_or_else(|| anyhow!("Items field not found in GitHub API response"))?;

        // Convert items to a Vec<Value>
        let repositories: Vec<Value> = items
            .iter()
            .map(|item| {
                json!({
                    "name": item["full_name"].as_str().unwrap_or(""),
                    "description": item["description"].as_str().unwrap_or("No description"),
                    "url": item["html_url"].as_str().unwrap_or(""),
                    "stars": item["stargazers_count"].as_u64().unwrap_or(0),
                    "forks": item["forks_count"].as_u64().unwrap_or(0),
                    "language": item["language"].as_str().unwrap_or("Unknown"),
                    "updated_at": item["updated_at"].as_str().unwrap_or("")
                })
            })
            .collect();

        Ok(repositories)
    }
}

impl GitHubTool for RepoSearchTool {
    fn get_tool_definition(&self) -> Tool {
        // Create properties for the input schema
        let mut properties = HashMap::new();

        // Add query property
        properties.insert(
            "query".to_string(),
            json!({
                "type": "string",
                "description": "Search query for repositories"
            }),
        );

        // Add limit property
        properties.insert(
            "limit".to_string(),
            json!({
                "type": "integer",
                "description": "Maximum number of repositories to return",
                "default": 5
            }),
        );

        // Create required fields
        let required = vec!["query".to_string()];

        Tool {
            name: "repo_search".to_string(),
            description: Some("Search for GitHub repositories based on keywords".to_string()),
            input_schema: ToolInputSchema {
                r#type: "object".to_string(),
                properties: Some(properties),
                required: Some(required),
            },
        }
    }

    fn handle(&self, params: Value) -> Result<Value> {
        // Extract parameters
        let query = params["query"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing 'query' parameter"))?;

        let limit = params["limit"].as_u64().unwrap_or(5) as usize;
        debug!(
            "Searching for repositories with query: {}, limit: {}",
            query, limit
        );

        // Search repositories
        let repositories = self.search_repositories(query, limit)?;
        info!(
            "Found {} repositories matching query: {}",
            repositories.len(),
            query
        );

        // Return the result
        Ok(json!({
            "repositories": repositories,
            "query": query,
            "count": repositories.len()
        }))
    }
}
