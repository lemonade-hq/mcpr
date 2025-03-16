//! Transport layer for MCP communication
//!
//! This module provides transport implementations for the Model Context Protocol (MCP).
//! Transports handle the underlying mechanics of how messages are sent and received.
//!
//! The following transport types are supported:
//! - Stdio: Standard input/output for local processes
//! - SSE: Server-Sent Events for server-to-client messages with HTTP POST for client-to-server
//!
//! The following transport types are planned but not yet implemented:
//! - WebSocket: Bidirectional communication over WebSockets (TBD)
//!
//! Note: There are some linter errors related to async/await in this file.
//! These errors occur because the async implementations require proper async
//! HTTP and WebSocket clients. To fix these errors, you would need to:
//! 1. Add proper async dependencies to your Cargo.toml
//! 2. Implement the async methods using those dependencies
//! 3. Use proper async/await syntax throughout the implementation
//!
//! For now, the synchronous implementations are provided and work correctly.

use crate::error::MCPError;
use serde::{de::DeserializeOwned, Serialize};
use std::io;

/// Type alias for a closure that is called when an error occurs
pub type ErrorCallback = Box<dyn Fn(&MCPError) + Send + Sync>;

/// Type alias for a closure that is called when a message is received
pub type MessageCallback = Box<dyn Fn(&str) + Send + Sync>;

/// Type alias for a closure that is called when the connection is closed
pub type CloseCallback = Box<dyn Fn() + Send + Sync>;

/// Transport trait for MCP communication
pub trait Transport {
    /// Start processing messages
    fn start(&mut self) -> Result<(), MCPError>;

    /// Send a message
    fn send<T: Serialize>(&mut self, message: &T) -> Result<(), MCPError>;

    /// Receive a message
    fn receive<T: DeserializeOwned>(&mut self) -> Result<T, MCPError>;

    /// Close the connection
    fn close(&mut self) -> Result<(), MCPError>;

    /// Set callback for when the connection is closed
    fn set_on_close(&mut self, callback: Option<CloseCallback>);

    /// Set callback for when an error occurs
    fn set_on_error(&mut self, callback: Option<ErrorCallback>);

    /// Set callback for when a message is received
    fn set_on_message<F>(&mut self, callback: Option<F>)
    where
        F: Fn(&str) + Send + Sync + 'static;
}

/// Standard IO transport
pub mod stdio {
    use super::*;
    use std::io::{BufRead, BufReader, Write};

    /// Standard IO transport
    pub struct StdioTransport {
        reader: BufReader<Box<dyn io::Read + Send>>,
        writer: Box<dyn io::Write + Send>,
        is_connected: bool,
        on_close: Option<CloseCallback>,
        on_error: Option<ErrorCallback>,
        on_message: Option<MessageCallback>,
    }

    impl Default for StdioTransport {
        fn default() -> Self {
            Self::new()
        }
    }

    impl StdioTransport {
        /// Create a new stdio transport using stdin and stdout
        pub fn new() -> Self {
            Self {
                reader: BufReader::new(Box::new(io::stdin())),
                writer: Box::new(io::stdout()),
                is_connected: false,
                on_close: None,
                on_error: None,
                on_message: None,
            }
        }

        /// Create a new stdio transport with custom reader and writer
        pub fn with_reader_writer(
            reader: Box<dyn io::Read + Send>,
            writer: Box<dyn io::Write + Send>,
        ) -> Self {
            Self {
                reader: BufReader::new(reader),
                writer,
                is_connected: false,
                on_close: None,
                on_error: None,
                on_message: None,
            }
        }

        /// Handle an error by calling the error callback if set
        fn handle_error(&self, error: &MCPError) {
            if let Some(callback) = &self.on_error {
                callback(error);
            }
        }
    }

    impl Transport for StdioTransport {
        fn start(&mut self) -> Result<(), MCPError> {
            if self.is_connected {
                return Ok(());
            }

            self.is_connected = true;
            Ok(())
        }

        fn send<T: Serialize>(&mut self, message: &T) -> Result<(), MCPError> {
            if !self.is_connected {
                let error = MCPError::Transport("Transport not connected".to_string());
                self.handle_error(&error);
                return Err(error);
            }

            let json = match serde_json::to_string(message) {
                Ok(json) => json,
                Err(e) => {
                    let error = MCPError::Serialization(e);
                    self.handle_error(&error);
                    return Err(error);
                }
            };

            match writeln!(self.writer, "{}", json) {
                Ok(_) => match self.writer.flush() {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        let error = MCPError::Transport(format!("Failed to flush: {}", e));
                        self.handle_error(&error);
                        Err(error)
                    }
                },
                Err(e) => {
                    let error = MCPError::Transport(format!("Failed to write: {}", e));
                    self.handle_error(&error);
                    Err(error)
                }
            }
        }

        fn receive<T: DeserializeOwned>(&mut self) -> Result<T, MCPError> {
            if !self.is_connected {
                let error = MCPError::Transport("Transport not connected".to_string());
                self.handle_error(&error);
                return Err(error);
            }

            let mut line = String::new();
            match self.reader.read_line(&mut line) {
                Ok(_) => {
                    if let Some(callback) = &self.on_message {
                        callback(&line);
                    }

                    match serde_json::from_str(&line) {
                        Ok(parsed) => Ok(parsed),
                        Err(e) => {
                            let error = MCPError::Serialization(e);
                            self.handle_error(&error);
                            Err(error)
                        }
                    }
                }
                Err(e) => {
                    let error = MCPError::Transport(format!("Failed to read: {}", e));
                    self.handle_error(&error);
                    Err(error)
                }
            }
        }

        fn close(&mut self) -> Result<(), MCPError> {
            if !self.is_connected {
                return Ok(());
            }

            self.is_connected = false;

            if let Some(callback) = &self.on_close {
                callback();
            }

            Ok(())
        }

        fn set_on_close(&mut self, callback: Option<CloseCallback>) {
            self.on_close = callback;
        }

        fn set_on_error(&mut self, callback: Option<ErrorCallback>) {
            self.on_error = callback;
        }

        fn set_on_message<F>(&mut self, callback: Option<F>)
        where
            F: Fn(&str) + Send + Sync + 'static,
        {
            self.on_message = callback.map(|f| Box::new(f) as Box<dyn Fn(&str) + Send + Sync>);
        }
    }
}

/// Server-Sent Events (SSE) transport
pub mod sse;

// Note: WebSocket transport is planned but not yet implemented
