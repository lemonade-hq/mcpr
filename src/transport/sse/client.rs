// cspell:ignore reqwest
use crate::error::MCPError;
use crate::transport::{CloseCallback, ErrorCallback, MessageCallback, Transport};
use async_trait::async_trait;
use futures::stream::StreamExt;
use log::{debug, warn};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use url::Url;

/// Server-Sent Events (SSE) Client Transport
pub struct SSEClientTransport {
    /// The URL for SSE events
    url: Url,

    /// The URL for sending requests (dynamically received from server)
    send_url: Arc<Mutex<Option<Url>>>,

    /// Authentication token for requests
    auth_token: Option<String>,

    /// Reconnection interval in seconds
    reconnect_interval: Duration,

    /// Maximum number of reconnection attempts
    max_reconnect_attempts: u32,

    /// Connection status
    is_connected: bool,

    /// Close callback
    on_close: Option<CloseCallback>,

    /// Error callback
    on_error: Option<ErrorCallback>,

    /// Message callback
    on_message: Option<MessageCallback>,

    /// Client handle
    client_handle: Option<JoinHandle<()>>,

    /// Store for received messages
    received_messages: Arc<Mutex<Vec<String>>>,

    /// Channel for receiving messages
    message_rx: Option<mpsc::Receiver<String>>,

    /// Sender for the message channel
    message_sender: Arc<mpsc::Sender<String>>,
}

impl SSEClientTransport {
    /// Create a new SSE transport in client mode
    ///
    /// Only requires the event source URL. The send URL will be dynamically
    /// provided by the server via an "endpoint" SSE event according to the MCP protocol.
    pub fn new(event_source_url: &str) -> Result<Self, MCPError> {
        let url = Url::parse(event_source_url)
            .map_err(|e| MCPError::Transport(format!("Invalid event source URL: {}", e)))?;

        // Create a channel for receiving messages
        let (message_tx, message_rx) = mpsc::channel::<String>(100);
        let message_sender = Arc::new(message_tx);
        let received_messages = Arc::new(Mutex::new(Vec::new()));

        Ok(Self {
            url,
            send_url: Arc::new(Mutex::new(None)),
            auth_token: None,
            reconnect_interval: Duration::from_secs(3), // Default 3 seconds
            max_reconnect_attempts: 5,                  // Default 5 attempts
            is_connected: false,
            on_close: None,
            on_error: None,
            on_message: None,
            client_handle: None,
            received_messages,
            message_rx: Some(message_rx),
            message_sender,
        })
    }

    /// Set authentication token for requests
    pub fn with_auth_token(mut self, token: &str) -> Self {
        self.auth_token = Some(token.to_string());
        self
    }

    /// Set reconnection parameters
    pub fn with_reconnect_params(mut self, interval_secs: u64, max_attempts: u32) -> Self {
        self.reconnect_interval = Duration::from_secs(interval_secs);
        self.max_reconnect_attempts = max_attempts;
        self
    }

    /// Start the SSE client
    async fn start_client(&mut self) -> Result<(), MCPError> {
        if self.is_connected {
            return Ok(());
        }

        // Clone necessary data for the client task
        let url = self.url.clone();
        let send_url_mutex = self.send_url.clone();
        let message_sender = self.message_sender.clone();
        let received_messages = self.received_messages.clone();
        let auth_token = self.auth_token.clone();
        let reconnect_interval = self.reconnect_interval;
        let max_reconnect_attempts = self.max_reconnect_attempts;

        // Create and spawn the client task
        let client_task = tokio::spawn(async move {
            let mut attempts = 0;

            loop {
                if attempts >= max_reconnect_attempts {
                    eprintln!("Maximum reconnection attempts reached, giving up");
                    break;
                }

                // Create a client with timeout for connection
                let client = reqwest::Client::builder()
                    .timeout(Duration::from_secs(30))
                    .build()
                    .unwrap_or_default();

                // Create the request
                let mut request = client.get(url.clone());

                // Add headers
                request = request.header("Accept", "text/event-stream");

                // Add authorization if available
                if let Some(token) = &auth_token {
                    request = request.header("Authorization", format!("Bearer {}", token));
                }

                // Send the request
                let response = match request.send().await {
                    Ok(resp) => {
                        if !resp.status().is_success() {
                            eprintln!("Server returned error status: {}", resp.status());
                            attempts += 1;
                            tokio::time::sleep(reconnect_interval).await;
                            continue;
                        }
                        resp
                    }
                    Err(e) => {
                        eprintln!("Failed to connect to SSE endpoint: {}", e);
                        attempts += 1;
                        tokio::time::sleep(reconnect_interval).await;
                        continue;
                    }
                };

                // Reset attempts counter on successful connection
                attempts = 0;

                // Process the SSE stream
                let mut stream = response.bytes_stream();
                let mut buffer = String::new();

                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(chunk) => {
                            // Convert bytes to string and append to buffer
                            if let Ok(text) = String::from_utf8(chunk.to_vec()) {
                                buffer.push_str(&text);

                                // Process complete SSE events
                                while let Some(pos) = buffer.find("\n\n") {
                                    let event = buffer[..pos + 2].to_string();
                                    buffer = buffer[pos + 2..].to_string();

                                    // Extract event type and data
                                    let mut event_type = "message"; // Default event type
                                    let mut event_data = String::new();

                                    for line in event.lines() {
                                        if line.starts_with("event:") {
                                            event_type = line[6..].trim();
                                        } else if line.starts_with("data:") {
                                            event_data = line[5..].trim().to_string();
                                        }
                                    }

                                    // Handle different event types
                                    match event_type {
                                        "endpoint" => {
                                            // Update the send URL from the endpoint event
                                            if let Ok(endpoint_url) = Url::parse(&event_data) {
                                                let mut send_url = send_url_mutex.lock().await;
                                                *send_url = Some(endpoint_url);
                                                debug!("Received endpoint URL: {}", event_data);
                                            } else {
                                                eprintln!(
                                                    "Received invalid endpoint URL: {}",
                                                    event_data
                                                );
                                            }
                                        }
                                        "message" => {
                                            // Process message event
                                            // Store the message
                                            {
                                                let mut messages = received_messages.lock().await;
                                                messages.push(event_data.clone());
                                            }

                                            // Send the message to the channel
                                            let _ = message_sender.send(event_data.clone()).await;
                                        }
                                        _ => {
                                            // Ignore unknown event types
                                            debug!("Received unknown event type: {}", event_type);
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Error reading SSE stream: {}", e);
                            break;
                        }
                    }
                }

                // If we reach here, the connection was lost
                eprintln!("SSE connection lost, attempting to reconnect...");
                tokio::time::sleep(reconnect_interval).await;
            }
        });

        // Store the handle
        self.client_handle = Some(client_task);
        self.is_connected = true;

        Ok(())
    }

    /// Handle an error by calling the error callback if set
    fn handle_error(&self, error: &MCPError) {
        if let Some(callback) = &self.on_error {
            callback(error);
        }
    }
}

impl Clone for SSEClientTransport {
    fn clone(&self) -> Self {
        Self {
            url: self.url.clone(),
            send_url: self.send_url.clone(),
            auth_token: self.auth_token.clone(),
            reconnect_interval: self.reconnect_interval,
            max_reconnect_attempts: self.max_reconnect_attempts,
            is_connected: self.is_connected,
            on_close: None, // Callbacks cannot be cloned
            on_error: None,
            on_message: None,
            client_handle: None, // Client handle cannot be cloned
            received_messages: self.received_messages.clone(),
            message_rx: None, // Receivers cannot be cloned
            message_sender: self.message_sender.clone(),
        }
    }
}

#[async_trait]
impl Transport for SSEClientTransport {
    async fn start(&mut self) -> Result<(), MCPError> {
        if self.is_connected {
            return Ok(());
        }

        self.start_client().await
    }

    async fn send<T: Serialize + Send + Sync>(&mut self, message: &T) -> Result<(), MCPError> {
        if !self.is_connected {
            let error = MCPError::Transport("Transport not connected".to_string());
            self.handle_error(&error);
            return Err(error);
        }

        // Get the send URL from the mutex
        let send_url = {
            let send_url_guard = self.send_url.lock().await;
            match &*send_url_guard {
                Some(url) => url.clone(),
                None => {
                    let error = MCPError::Transport(
                        "No send URL available. Waiting for endpoint event from server".to_string(),
                    );
                    self.handle_error(&error);
                    return Err(error);
                }
            }
        };

        // Serialize message to JSON
        let json = serde_json::to_string(message).map_err(|e| {
            let error = MCPError::Serialization(e.to_string());
            self.handle_error(&error);
            error
        })?;

        // Create a reqwest client
        let client = reqwest::Client::new();
        let mut request = client.post(send_url);

        // Add authorization header if auth token is set
        if let Some(token) = &self.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        // Send the request
        let response = request
            .header("Content-Type", "application/json")
            .body(json)
            .send()
            .await
            .map_err(|e| {
                let error = MCPError::Transport(format!("Failed to send message: {}", e));
                self.handle_error(&error);
                error
            })?;

        // Check response status
        if !response.status().is_success() {
            let error = MCPError::Transport(format!(
                "Server returned error status: {}",
                response.status()
            ));
            self.handle_error(&error);
            return Err(error);
        }

        Ok(())
    }

    async fn receive<T: DeserializeOwned + Send + Sync>(&mut self) -> Result<T, MCPError> {
        if !self.is_connected {
            let error = MCPError::Transport("Transport not connected".to_string());
            self.handle_error(&error);
            return Err(error);
        }

        // If we have a receiver, try to get a message
        if let Some(rx) = &mut self.message_rx {
            match rx.recv().await {
                Some(json) => {
                    // Call the message callback if set
                    if let Some(callback) = &self.on_message {
                        callback(&json);
                    }

                    // Parse the JSON message
                    serde_json::from_str(&json).map_err(|e| {
                        let error = MCPError::Deserialization(e.to_string());
                        self.handle_error(&error);
                        error
                    })
                }
                None => {
                    let error = MCPError::Transport("Message channel closed".to_string());
                    self.handle_error(&error);
                    Err(error)
                }
            }
        } else {
            let error = MCPError::Transport("Message receiver not initialized".to_string());
            self.handle_error(&error);
            Err(error)
        }
    }

    async fn close(&mut self) -> Result<(), MCPError> {
        if !self.is_connected {
            return Ok(());
        }

        self.is_connected = false;

        // Wait for the client to shutdown
        if let Some(handle) = self.client_handle.take() {
            let _ = handle.abort();
        }

        // Call the close callback if set
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
        self.on_message = callback.map(|f| Box::new(f) as MessageCallback);
    }
}

impl SSEClientTransport {
    pub fn has_auth_token(&self) -> bool {
        self.auth_token.is_some()
    }

    pub fn get_auth_token(&self) -> Option<&str> {
        self.auth_token.as_deref()
    }

    pub fn get_reconnect_interval(&self) -> Duration {
        self.reconnect_interval
    }

    pub fn get_max_reconnect_attempts(&self) -> u32 {
        self.max_reconnect_attempts
    }
}
