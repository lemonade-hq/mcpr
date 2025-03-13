//! # Model Context Protocol (MCP) for Rust
//!
//! This crate provides a Rust implementation of Anthropic's Model Context Protocol (MCP),
//! an open standard for connecting AI assistants to data sources and tools.
//!
//! The implementation includes:
//! - Schema definitions for MCP messages
//! - Transport layer for communication
//! - CLI tools for generating server and client stubs
//! - Generator for creating MCP server and client stubs

pub mod cli;
pub mod generator;
pub mod schema;
pub mod transport;

// Re-export commonly used types
pub use schema::common::{Cursor, LoggingLevel, ProgressToken};
pub use schema::json_rpc::{JSONRPCMessage, RequestId};

/// Protocol version constants
pub mod constants {
    /// The latest supported MCP protocol version
    pub const LATEST_PROTOCOL_VERSION: &str = "2024-11-05";
    /// The JSON-RPC version used by MCP
    pub const JSONRPC_VERSION: &str = "2.0";
}

/// Error types for the MCP implementation
pub mod error {
    use thiserror::Error;

    #[derive(Error, Debug)]
    pub enum MCPError {
        #[error("JSON serialization error: {0}")]
        Serialization(#[from] serde_json::Error),

        #[error("Transport error: {0}")]
        Transport(String),

        #[error("Protocol error: {0}")]
        Protocol(String),

        #[error("Unsupported feature: {0}")]
        UnsupportedFeature(String),
    }
}
