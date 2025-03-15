use anyhow::Result;
use mcpr::schema::common::Tool;
use serde_json::Value;

pub mod readme_query;
pub mod repo_search;

pub use readme_query::ReadmeQueryTool;
pub use repo_search::RepoSearchTool;

/// Trait for GitHub tools
pub trait GitHubTool: Send + Sync {
    /// Get the tool definition
    fn get_tool_definition(&self) -> Tool;

    /// Handle a tool call
    fn handle(&self, params: Value) -> Result<Value>;
}
