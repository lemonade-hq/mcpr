use crate::error::MCPError;
use crate::transport::{CloseCallback, ErrorCallback, MessageCallback, Transport};
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use serde::{de::DeserializeOwned, Serialize};
use std::{collections::VecDeque, sync::Arc, time::Duration};
use tokio::{
    sync::{Mutex as TokioMutex, Notify},
    time::sleep,
};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

/// WebSocket transport implementation for MCP
pub struct WebSocketTransport {
    uri: String,
    is_connected: bool,
    is_server: bool,
    on_close: Option<CloseCallback>,
    on_error: Option<ErrorCallback>,
    on_message: Option<MessageCallback>,

    // Queue for incoming messages
    message_queue: Arc<TokioMutex<VecDeque<String>>>,

    // Signal to stop background tasks
    stop_signal: Arc<Notify>,

    // Background task handle
    message_task: Option<tokio::task::JoinHandle<()>>,
}

// Implement Clone for WebSocketTransport
impl Clone for WebSocketTransport {
    fn clone(&self) -> Self {
        Self {
            uri: self.uri.clone(),
            is_connected: self.is_connected,
            is_server: self.is_server,
            on_close: None, // Callbacks cannot be cloned
            on_error: None,
            on_message: None,
            message_queue: Arc::new(TokioMutex::new(VecDeque::new())),
            stop_signal: Arc::new(Notify::new()),
            message_task: None, // Each clone should create its own task
        }
    }
}

impl WebSocketTransport {
    /// Create a new WebSocket transport in client mode
    pub fn new(uri: &str) -> Self {
        info!("Creating new WebSocket client transport with URI: {}", uri);
        Self {
            uri: uri.to_string(),
            is_connected: false,
            is_server: false,
            on_close: None,
            on_error: None,
            on_message: None,
            message_queue: Arc::new(TokioMutex::new(VecDeque::new())),
            stop_signal: Arc::new(Notify::new()),
            message_task: None,
        }
    }

    /// Create a new WebSocket transport in server mode
    pub fn new_server(uri: &str) -> Self {
        info!("Creating new WebSocket server transport with URI: {}", uri);
        let mut transport = Self::new(uri);
        transport.is_server = true;
        transport
    }

    /// Start client connection to a WebSocket server
    async fn connect_as_client(&mut self) -> Result<(), MCPError> {
        debug!("Connecting to WebSocket server: {}", self.uri);

        // Parse URL
        let url = Url::parse(&self.uri)
            .map_err(|e| MCPError::Transport(format!("Invalid WebSocket URL: {}", e)))?;

        // Connect to server
        let (ws_stream, _) = connect_async(url).await.map_err(|e| {
            MCPError::Transport(format!("Failed to connect to WebSocket server: {}", e))
        })?;

        info!("Connected to WebSocket server: {}", self.uri);

        // Start message processing
        self.start_message_processing(ws_stream).await?;

        Ok(())
    }

    /// Start server and listen for connections
    async fn start_as_server(&mut self) -> Result<(), MCPError> {
        debug!("Starting WebSocket server on: {}", self.uri);

        // We'll use tokio::net::TcpListener directly for simplicity
        let listener = tokio::net::TcpListener::bind(&self.uri)
            .await
            .map_err(|e| MCPError::Transport(format!("Failed to bind to {}: {}", self.uri, e)))?;

        info!("WebSocket server listening on {}", self.uri);

        // Accept a connection
        let (socket, addr) = listener
            .accept()
            .await
            .map_err(|e| MCPError::Transport(format!("Failed to accept connection: {}", e)))?;

        info!("WebSocket connection accepted from {}", addr);

        // Upgrade to WebSocket
        let ws_stream = tokio_tungstenite::accept_async(socket)
            .await
            .map_err(|e| MCPError::Transport(format!("Error during WebSocket handshake: {}", e)))?;

        // Start message processing
        self.start_message_processing(ws_stream).await?;

        Ok(())
    }

    /// Start message processing for the WebSocket stream
    async fn start_message_processing<S>(&mut self, ws_stream: S) -> Result<(), MCPError>
    where
        S: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>
            + Send
            + 'static,
    {
        // Clone resources for the task
        let message_queue = Arc::clone(&self.message_queue);
        let stop_signal = Arc::clone(&self.stop_signal);

        // Start a task to process incoming messages
        self.message_task = Some(tokio::spawn(async move {
            debug!("WebSocket message processing task started");

            tokio::pin!(ws_stream);

            // Process messages until the stream ends or we're signaled to stop
            loop {
                tokio::select! {
                    // Process incoming message
                    msg = ws_stream.next() => match msg {
                        Some(Ok(msg)) => {
                            if let Message::Text(text) = msg {
                                debug!("Received WebSocket text message: {}", text);

                                // Add to message queue
                                let mut queue = message_queue.lock().await;
                                queue.push_back(text);
                            } else if let Message::Binary(data) = msg {
                                debug!("Received WebSocket binary message of {} bytes", data.len());
                                // We don't handle binary messages currently
                            } else if let Message::Close(_) = msg {
                                debug!("Received WebSocket close message");
                                break;
                            }
                        },
                        Some(Err(e)) => {
                            error!("WebSocket error: {}", e);
                            break;
                        },
                        None => {
                            debug!("WebSocket stream ended");
                            break;
                        }
                    },

                    // Check for stop signal
                    _ = stop_signal.notified() => {
                        debug!("Received stop signal, exiting message processing task");
                        break;
                    }
                }
            }

            debug!("WebSocket message processing task ended");
        }));

        Ok(())
    }

    /// Create a new connection for sending messages
    async fn create_send_connection(&self) -> Result<impl SinkExt<Message>, MCPError> {
        let url = Url::parse(&self.uri)
            .map_err(|e| MCPError::Transport(format!("Invalid WebSocket URL: {}", e)))?;

        let (stream, _) = connect_async(url)
            .await
            .map_err(|e| MCPError::Transport(format!("Failed to create send connection: {}", e)))?;

        Ok(stream)
    }
}

#[async_trait]
impl Transport for WebSocketTransport {
    async fn start(&mut self) -> Result<(), MCPError> {
        if self.is_connected {
            debug!("WebSocket transport already connected");
            return Ok(());
        }

        info!("Starting WebSocket transport: {}", self.uri);

        // Connect or start server based on mode
        if self.is_server {
            self.start_as_server().await?;
        } else {
            self.connect_as_client().await?;
        }

        self.is_connected = true;
        info!("WebSocket transport started successfully");
        Ok(())
    }

    async fn send<T: Serialize + Send + Sync>(&mut self, message: &T) -> Result<(), MCPError> {
        if !self.is_connected {
            return Err(MCPError::Transport(
                "WebSocket transport not connected".to_string(),
            ));
        }

        // Serialize the message
        let serialized_message = serde_json::to_string(message).map_err(|e| {
            let msg = format!("Failed to serialize message: {}", e);
            error!("{}", msg);
            MCPError::Serialization(e)
        })?;

        debug!("Sending WebSocket message: {}", serialized_message);

        // Create a new connection for sending if none exists
        let mut send_stream = self.create_send_connection().await?;

        // Send the message
        send_stream
            .send(Message::Text(serialized_message))
            .await
            .map_err(|_| MCPError::Transport("Error sending WebSocket message".to_string()))?;

        debug!("WebSocket message sent successfully");
        Ok(())
    }

    async fn receive<T: DeserializeOwned + Send + Sync>(&mut self) -> Result<T, MCPError> {
        if !self.is_connected {
            return Err(MCPError::Transport(
                "WebSocket transport not connected".to_string(),
            ));
        }

        // Use a timeout of 30 seconds
        let timeout_duration = Duration::from_secs(30);
        let start = std::time::Instant::now();

        // Wait for a message
        let message = loop {
            // Check for timeout
            if start.elapsed() >= timeout_duration {
                return Err(MCPError::Transport(
                    "Timeout waiting for message".to_string(),
                ));
            }

            // Try to get a message from the queue
            let queue_msg = {
                let mut queue = self.message_queue.lock().await;
                queue.pop_front()
            };

            if let Some(message) = queue_msg {
                debug!("Received message from queue: {}", message);

                // Execute callback if set
                if let Some(callback) = &self.on_message {
                    callback(&message);
                }

                break message;
            }

            // Wait before checking again
            sleep(Duration::from_millis(100)).await;
        };

        // Parse the message
        match serde_json::from_str::<T>(&message) {
            Ok(parsed) => {
                debug!("Successfully parsed WebSocket message");
                Ok(parsed)
            }
            Err(e) => {
                let error_msg = format!(
                    "Failed to deserialize WebSocket message: {} - Content: {}",
                    e, message
                );
                error!("{}", error_msg);
                Err(MCPError::Serialization(e))
            }
        }
    }

    async fn close(&mut self) -> Result<(), MCPError> {
        if !self.is_connected {
            debug!("WebSocket transport already closed");
            return Ok(());
        }

        info!("Closing WebSocket transport: {}", self.uri);

        // Create a send connection to send the close frame
        let mut send_stream = self.create_send_connection().await?;

        // Send close frame
        debug!("Sending WebSocket close frame");
        if let Err(_) = send_stream.send(Message::Close(None)).await {
            warn!("Error sending WebSocket close frame");
        }

        // Signal tasks to stop
        self.stop_signal.notify_waiters();

        // Wait for tasks to finish
        if let Some(task) = self.message_task.take() {
            debug!("Waiting for WebSocket message task to finish");
            let _ = tokio::time::timeout(Duration::from_secs(5), task).await;
        }

        // Update state
        self.is_connected = false;

        // Call close callback
        if let Some(callback) = &self.on_close {
            callback();
        }

        info!("WebSocket transport closed successfully");
        Ok(())
    }

    fn set_on_close(&mut self, callback: Option<CloseCallback>) {
        debug!("Setting on_close callback for WebSocket transport");
        self.on_close = callback;
    }

    fn set_on_error(&mut self, callback: Option<ErrorCallback>) {
        debug!("Setting on_error callback for WebSocket transport");
        self.on_error = callback;
    }

    fn set_on_message<F>(&mut self, callback: Option<F>)
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        debug!("Setting on_message callback for WebSocket transport");
        self.on_message = callback.map(|f| Box::new(f) as MessageCallback);
    }
}

impl Drop for WebSocketTransport {
    fn drop(&mut self) {
        if self.is_connected {
            debug!("WebSocketTransport dropped while still connected, attempting to close");
            self.stop_signal.notify_waiters();
        }
        debug!("WebSocketTransport dropped");
    }
}
