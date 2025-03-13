//! Transport layer for MCP communication

use crate::error::MCPError;
use serde::{de::DeserializeOwned, Serialize};
use std::io;

/// Transport trait for MCP communication
pub trait Transport {
    /// Send a message
    fn send<T: Serialize>(&mut self, message: &T) -> Result<(), MCPError>;

    /// Receive a message
    fn receive<T: DeserializeOwned>(&mut self) -> Result<T, MCPError>;
}

/// Async transport trait for MCP communication
#[cfg(feature = "async")]
pub trait AsyncTransport {
    /// Send a message asynchronously
    async fn send<T: Serialize>(&mut self, message: &T) -> Result<(), MCPError>;

    /// Receive a message asynchronously
    async fn receive<T: DeserializeOwned>(&mut self) -> Result<T, MCPError>;
}

/// Standard IO transport
pub mod stdio {
    use super::*;
    use std::io::{BufRead, BufReader, Write};

    /// Standard IO transport
    pub struct StdioTransport {
        reader: BufReader<Box<dyn io::Read>>,
        writer: Box<dyn io::Write>,
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
            }
        }

        /// Create a new stdio transport with custom reader and writer
        pub fn with_reader_writer(reader: Box<dyn io::Read>, writer: Box<dyn io::Write>) -> Self {
            Self {
                reader: BufReader::new(reader),
                writer,
            }
        }
    }

    impl Transport for StdioTransport {
        fn send<T: Serialize>(&mut self, message: &T) -> Result<(), MCPError> {
            let json = serde_json::to_string(message)?;
            writeln!(self.writer, "{}", json).map_err(|e| MCPError::Transport(e.to_string()))?;
            self.writer
                .flush()
                .map_err(|e| MCPError::Transport(e.to_string()))?;
            Ok(())
        }

        fn receive<T: DeserializeOwned>(&mut self) -> Result<T, MCPError> {
            let mut line = String::new();
            self.reader
                .read_line(&mut line)
                .map_err(|e| MCPError::Transport(e.to_string()))?;
            let message = serde_json::from_str(&line)?;
            Ok(message)
        }
    }
}

/// WebSocket transport
pub mod websocket {
    // To be implemented
}
