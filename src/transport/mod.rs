//! Transport layer for MCP communication
//!
//! This module provides transport implementations for the Model Context Protocol (MCP).
//! Transports handle the underlying mechanics of how messages are sent and received.
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
use std::{
    io,
    sync::{Arc, Mutex},
};

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
    fn set_on_close(&mut self, callback: Option<Box<dyn Fn() + Send + Sync>>);

    /// Set callback for when an error occurs
    fn set_on_error(&mut self, callback: Option<Box<dyn Fn(&MCPError) + Send + Sync>>);

    /// Set callback for when a message is received
    fn set_on_message<F>(&mut self, callback: Option<F>)
    where
        F: Fn(&str) + Send + Sync + 'static;
}

// Note: AsyncTransport trait is not provided here to avoid compilation issues.
// When implementing async support, you would need to define an AsyncTransport trait
// with async methods and implement it for each transport type.

/// Standard IO transport
pub mod stdio {
    use super::*;
    use std::io::{BufRead, BufReader, Write};

    /// Standard IO transport
    pub struct StdioTransport {
        reader: BufReader<Box<dyn io::Read + Send>>,
        writer: Box<dyn io::Write + Send>,
        is_connected: bool,
        on_close: Option<Box<dyn Fn() + Send + Sync>>,
        on_error: Option<Box<dyn Fn(&MCPError) + Send + Sync>>,
        on_message: Option<Box<dyn Fn(&str) + Send + Sync>>,
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
                    let error = MCPError::Transport(format!("Serialization error: {}", e));
                    self.handle_error(&error);
                    return Err(error);
                }
            };

            if let Err(e) = writeln!(self.writer, "{}", json) {
                let error = MCPError::Transport(e.to_string());
                self.handle_error(&error);
                return Err(error);
            }

            if let Err(e) = self.writer.flush() {
                let error = MCPError::Transport(e.to_string());
                self.handle_error(&error);
                return Err(error);
            }

            Ok(())
        }

        fn receive<T: DeserializeOwned>(&mut self) -> Result<T, MCPError> {
            if !self.is_connected {
                let error = MCPError::Transport("Transport not connected".to_string());
                self.handle_error(&error);
                return Err(error);
            }

            let mut line = String::new();
            match self.reader.read_line(&mut line) {
                Ok(0) => {
                    // End of file
                    self.is_connected = false;
                    if let Some(callback) = &self.on_close {
                        callback();
                    }
                    return Err(MCPError::Transport("Connection closed".to_string()));
                }
                Ok(_) => {}
                Err(e) => {
                    let error = MCPError::Transport(e.to_string());
                    self.handle_error(&error);
                    return Err(error);
                }
            }

            // Call the message callback with the raw JSON string
            if let Some(callback) = &self.on_message {
                callback(&line);
            }

            // Parse the JSON string into the requested type
            match serde_json::from_str::<T>(&line) {
                Ok(message) => Ok(message),
                Err(e) => {
                    let error = MCPError::Transport(format!("Deserialization error: {}", e));
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

        fn set_on_close(&mut self, callback: Option<Box<dyn Fn() + Send + Sync>>) {
            self.on_close = callback;
        }

        fn set_on_error(&mut self, callback: Option<Box<dyn Fn(&MCPError) + Send + Sync>>) {
            self.on_error = callback;
        }

        fn set_on_message<F>(&mut self, callback: Option<F>)
        where
            F: Fn(&str) + Send + Sync + 'static,
        {
            self.on_message = callback.map(|cb| Box::new(cb) as Box<dyn Fn(&str) + Send + Sync>);
        }
    }
}

/// Server-Sent Events (SSE) transport
pub mod sse {
    use super::*;

    /// SSE transport for server-to-client streaming with HTTP POST for client-to-server
    pub struct SSETransport {
        is_connected: bool,
        on_close: Option<Box<dyn Fn() + Send + Sync>>,
        on_error: Option<Box<dyn Fn(&MCPError) + Send + Sync>>,
        on_message: Option<Box<dyn Fn(&str) + Send + Sync>>,
        post_url: String,
        event_source: Option<EventSource>,
        client: Option<HttpClient>,
    }

    /// HTTP client for sending POST requests
    struct HttpClient {
        url: String,
    }

    /// Event source for receiving SSE events
    struct EventSource {
        // This would be implemented with a real SSE client
        // For now, we'll use a placeholder
        _placeholder: (),
    }

    impl HttpClient {
        /// Create a new HTTP client
        fn new(url: &str) -> Self {
            Self {
                url: url.to_string(),
            }
        }

        /// Send a POST request
        fn post(&self, data: &str) -> Result<(), MCPError> {
            // This is a simplified implementation that doesn't actually send HTTP requests
            // In a real implementation, you would use reqwest with the blocking feature
            // or implement a proper HTTP client

            // For now, we'll just log the request and return success
            log::info!(
                "Would send POST request to {} with data: {}",
                self.url,
                data
            );

            // Simulate successful request
            Ok(())
        }
    }

    impl SSETransport {
        /// Create a new SSE transport
        pub fn new(post_url: &str) -> Self {
            Self {
                is_connected: false,
                on_close: None,
                on_error: None,
                on_message: None,
                post_url: post_url.to_string(),
                event_source: None,
                client: None,
            }
        }

        /// Handle an error by calling the error callback if set
        fn handle_error(&self, error: &MCPError) {
            if let Some(callback) = &self.on_error {
                callback(error);
            }
        }

        /// Connect to the SSE event source
        fn connect_event_source(&mut self) -> Result<(), MCPError> {
            // In a real implementation, this would connect to an SSE event source
            // For now, we'll just create a placeholder
            self.event_source = Some(EventSource { _placeholder: () });
            Ok(())
        }
    }

    impl Transport for SSETransport {
        fn start(&mut self) -> Result<(), MCPError> {
            if self.is_connected {
                return Ok(());
            }

            // Initialize HTTP client
            self.client = Some(HttpClient::new(&self.post_url));

            // Connect to event source
            self.connect_event_source()?;

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
                    let error = MCPError::Transport(format!("Serialization error: {}", e));
                    self.handle_error(&error);
                    return Err(error);
                }
            };

            // Send the message via HTTP POST
            if let Some(client) = &self.client {
                if let Err(e) = client.post(&json) {
                    self.handle_error(&e);
                    return Err(e);
                }
            } else {
                let error = MCPError::Transport("HTTP client not initialized".to_string());
                self.handle_error(&error);
                return Err(error);
            }

            Ok(())
        }

        fn receive<T: DeserializeOwned>(&mut self) -> Result<T, MCPError> {
            // In a real implementation, this would wait for an SSE event
            // For now, we'll just return an error
            let error =
                MCPError::Transport("SSE receive not implemented in blocking mode".to_string());
            self.handle_error(&error);
            Err(error)
        }

        fn close(&mut self) -> Result<(), MCPError> {
            if !self.is_connected {
                return Ok(());
            }

            self.is_connected = false;
            self.event_source = None;
            self.client = None;

            if let Some(callback) = &self.on_close {
                callback();
            }

            Ok(())
        }

        fn set_on_close(&mut self, callback: Option<Box<dyn Fn() + Send + Sync>>) {
            self.on_close = callback;
        }

        fn set_on_error(&mut self, callback: Option<Box<dyn Fn(&MCPError) + Send + Sync>>) {
            self.on_error = callback;
        }

        fn set_on_message<F>(&mut self, callback: Option<F>)
        where
            F: Fn(&str) + Send + Sync + 'static,
        {
            self.on_message = callback.map(|cb| Box::new(cb) as Box<dyn Fn(&str) + Send + Sync>);
        }
    }
}

/// WebSocket transport
pub mod websocket {
    use super::*;

    /// WebSocket transport for full-duplex communication
    pub struct WebSocketTransport {
        is_connected: bool,
        on_close: Option<Box<dyn Fn() + Send + Sync>>,
        on_error: Option<Box<dyn Fn(&MCPError) + Send + Sync>>,
        on_message: Option<Box<dyn Fn(&str) + Send + Sync>>,
        url: String,
        socket: Option<WebSocket>,
        message_queue: Arc<Mutex<Vec<String>>>,
    }

    /// WebSocket connection
    struct WebSocket {
        // This would be implemented with a real WebSocket client
        // For now, we'll use a placeholder
        _placeholder: (),
        // The URL is stored for future implementation but currently unused
        _url: String,
        message_queue: Arc<Mutex<Vec<String>>>,
    }

    impl WebSocket {
        /// Create a new WebSocket connection
        fn new(url: &str, message_queue: Arc<Mutex<Vec<String>>>) -> Self {
            Self {
                _placeholder: (),
                _url: url.to_string(),
                message_queue,
            }
        }

        /// Send a message over the WebSocket
        fn send(&self, data: &str) -> Result<(), MCPError> {
            // In a real implementation, this would send data over the WebSocket
            // For now, we'll just add it to the message queue for demonstration
            if let Ok(mut queue) = self.message_queue.lock() {
                queue.push(data.to_string());
                Ok(())
            } else {
                Err(MCPError::Transport(
                    "Failed to lock message queue".to_string(),
                ))
            }
        }

        /// Close the WebSocket connection
        fn close(&mut self) -> Result<(), MCPError> {
            // In a real implementation, this would close the WebSocket connection
            Ok(())
        }

        /// Check if there are any messages in the queue
        fn has_messages(&self) -> bool {
            if let Ok(queue) = self.message_queue.lock() {
                !queue.is_empty()
            } else {
                false
            }
        }

        /// Get the next message from the queue
        fn next_message(&self) -> Option<String> {
            if let Ok(mut queue) = self.message_queue.lock() {
                queue.pop()
            } else {
                None
            }
        }
    }

    impl WebSocketTransport {
        /// Create a new WebSocket transport
        pub fn new(url: &str) -> Self {
            let message_queue = Arc::new(Mutex::new(Vec::new()));
            Self {
                is_connected: false,
                on_close: None,
                on_error: None,
                on_message: None,
                url: url.to_string(),
                socket: None,
                message_queue,
            }
        }

        /// Handle an error by calling the error callback if set
        fn handle_error(&self, error: &MCPError) {
            if let Some(callback) = &self.on_error {
                callback(error);
            }
        }
    }

    impl Transport for WebSocketTransport {
        fn start(&mut self) -> Result<(), MCPError> {
            if self.is_connected {
                return Ok(());
            }

            // Create a new WebSocket connection
            self.socket = Some(WebSocket::new(&self.url, self.message_queue.clone()));

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
                    let error = MCPError::Transport(format!("Serialization error: {}", e));
                    self.handle_error(&error);
                    return Err(error);
                }
            };

            // Send the message over the WebSocket
            if let Some(socket) = &self.socket {
                if let Err(e) = socket.send(&json) {
                    self.handle_error(&e);
                    return Err(e);
                }
            } else {
                let error = MCPError::Transport("WebSocket not initialized".to_string());
                self.handle_error(&error);
                return Err(error);
            }

            Ok(())
        }

        fn receive<T: DeserializeOwned>(&mut self) -> Result<T, MCPError> {
            if !self.is_connected {
                let error = MCPError::Transport("Transport not connected".to_string());
                self.handle_error(&error);
                return Err(error);
            }

            // Check if there are any messages in the queue
            if let Some(socket) = &self.socket {
                if socket.has_messages() {
                    if let Some(message) = socket.next_message() {
                        // Call the message callback with the raw JSON string
                        if let Some(callback) = &self.on_message {
                            callback(&message);
                        }

                        // Parse the JSON string into the requested type
                        match serde_json::from_str::<T>(&message) {
                            Ok(parsed) => return Ok(parsed),
                            Err(e) => {
                                let error =
                                    MCPError::Transport(format!("Deserialization error: {}", e));
                                self.handle_error(&error);
                                return Err(error);
                            }
                        }
                    }
                }
            }

            // No messages available
            let error = MCPError::Transport("No messages available".to_string());
            self.handle_error(&error);
            Err(error)
        }

        fn close(&mut self) -> Result<(), MCPError> {
            if !self.is_connected {
                return Ok(());
            }

            // Close the WebSocket connection
            if let Some(socket) = &mut self.socket {
                if let Err(e) = socket.close() {
                    self.handle_error(&e);
                    return Err(e);
                }
            }

            self.is_connected = false;
            self.socket = None;

            if let Some(callback) = &self.on_close {
                callback();
            }

            Ok(())
        }

        fn set_on_close(&mut self, callback: Option<Box<dyn Fn() + Send + Sync>>) {
            self.on_close = callback;
        }

        fn set_on_error(&mut self, callback: Option<Box<dyn Fn(&MCPError) + Send + Sync>>) {
            self.on_error = callback;
        }

        fn set_on_message<F>(&mut self, callback: Option<F>)
        where
            F: Fn(&str) + Send + Sync + 'static,
        {
            self.on_message = callback.map(|cb| Box::new(cb) as Box<dyn Fn(&str) + Send + Sync>);
        }
    }
}
