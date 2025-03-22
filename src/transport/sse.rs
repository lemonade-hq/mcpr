// cspell:ignore reqwest
use crate::error::MCPError;
use crate::transport::{CloseCallback, ErrorCallback, MessageCallback, Transport};
use async_trait::async_trait;
use log::warn;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio::task::JoinHandle;
use url::Url;
use uuid::Uuid;

/// Server-Sent Events (SSE) transport
pub struct SSETransport {
    /// The base URL for the server
    url: Url,

    /// Connection status
    is_connected: bool,

    /// Message queue for sending
    sender_tx: mpsc::Sender<String>,

    /// Close callback
    on_close: Option<CloseCallback>,

    /// Error callback
    on_error: Option<ErrorCallback>,

    /// Message callback
    on_message: Option<MessageCallback>,

    /// Server mode fields
    server_handle: Option<JoinHandle<()>>,
    message_broadcaster: Option<Arc<broadcast::Sender<String>>>,
    server_shutdown_tx: Option<mpsc::Sender<()>>,
    /// Store for received messages
    received_messages: Option<Arc<Mutex<Vec<String>>>>,
    /// Channel for receiving messages
    message_rx: Option<mpsc::Receiver<String>>,
    /// Sender for the message channel (must be kept to prevent channel closure)
    message_sender: Option<Arc<mpsc::Sender<String>>>,
    /// Active SSE sessions
    active_sessions: Option<Arc<Mutex<HashMap<String, mpsc::Sender<String>>>>>,
}

impl SSETransport {
    /// Create a new SSE transport in server mode
    pub fn new_server(url: &str) -> Result<Self, MCPError> {
        let url = Url::parse(url)
            .map_err(|e| MCPError::Transport(format!("Invalid server URL: {}", e)))?;

        // Create a sender channel for server message sending
        let (sender_tx, _) = mpsc::channel::<String>(32);

        // Create a broadcast channel for SSE events
        let (broadcast_tx, _) = broadcast::channel::<String>(100);
        let broadcaster = Arc::new(broadcast_tx);

        // Create channel for receiving messages
        let (message_tx, message_rx) = mpsc::channel::<String>(100);
        let message_sender = Arc::new(message_tx);
        let received_messages = Arc::new(Mutex::new(Vec::new()));

        // Session tracking
        let active_sessions = Arc::new(Mutex::new(HashMap::new()));

        Ok(Self {
            url,
            is_connected: false,
            sender_tx,
            on_close: None,
            on_error: None,
            on_message: None,
            server_handle: None,
            message_broadcaster: Some(broadcaster),
            server_shutdown_tx: None,
            received_messages: Some(received_messages),
            message_rx: Some(message_rx),
            message_sender: Some(message_sender),
            active_sessions: Some(active_sessions),
        })
    }

    /// Start the SSE server
    async fn start_server(&mut self) -> Result<(), MCPError> {
        if self.is_connected {
            return Ok(());
        }

        // Get the host and port from the URL
        let host = self.url.host_str().unwrap_or("127.0.0.1");
        let port = self.url.port().unwrap_or(8000);
        let addr = format!("{}:{}", host, port)
            .parse::<SocketAddr>()
            .map_err(|e| MCPError::Transport(format!("Invalid address: {}", e)))?;

        // Create a TcpListener
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| MCPError::Transport(format!("Failed to bind to address: {}", e)))?;

        // Create a channel for shutdown signaling
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.server_shutdown_tx = Some(shutdown_tx);

        // Get the broadcaster
        let broadcaster = self.message_broadcaster.clone().ok_or_else(|| {
            MCPError::Transport("Message broadcaster not initialized".to_string())
        })?;

        // Clone for the server task
        let message_broadcaster = broadcaster.clone();

        // Store received messages
        let received_messages = self.received_messages.clone().ok_or_else(|| {
            MCPError::Transport("Received messages store not initialized".to_string())
        })?;

        // Get message sender
        let message_sender = self
            .message_sender
            .clone()
            .ok_or_else(|| MCPError::Transport("Message sender not initialized".to_string()))?;

        // Get active sessions
        let active_sessions = self
            .active_sessions
            .clone()
            .ok_or_else(|| MCPError::Transport("Active sessions not initialized".to_string()))?;

        // Spawn the server task
        let handle = tokio::spawn(async move {
            println!("SSE server listening on http://{}", addr);
            println!("Endpoints:");
            println!("  - GET  http://{}/events   (SSE events stream)", addr);
            println!("  - POST http://{}/messages (Message endpoint)", addr);

            // Accept connections until shutdown
            loop {
                tokio::select! {
                    result = listener.accept() => {
                        match result {
                            Ok((stream, _)) => {
                                let broadcaster = message_broadcaster.clone();
                                let messages = received_messages.clone();
                                let task_message_tx = message_sender.clone();
                                let sessions = active_sessions.clone();

                                tokio::spawn(async move {
                                    // Peek to determine request type
                                    let mut stream = stream;
                                    let mut peek_buffer = [0; 128];
                                    let n = match stream.peek(&mut peek_buffer).await {
                                        Ok(n) => n,
                                        Err(_) => return,
                                    };

                                    let peek_str = String::from_utf8_lossy(&peek_buffer[..n]);

                                    // Handle based on request type
                                    if peek_str.starts_with("GET") {
                                        // Handle SSE connection
                                        let _ = handle_sse_connection(stream, broadcaster, sessions).await;
                                    } else if peek_str.starts_with("POST") {
                                        // Handle POST request
                                        let _ = handle_post_request(stream, broadcaster, messages, task_message_tx, sessions).await;
                                    } else {
                                        // Unknown method
                                        let response = "HTTP/1.1 405 Method Not Allowed\r\nContent-Length: 18\r\n\r\nMethod Not Allowed";
                                        let _ = tokio::io::AsyncWriteExt::write_all(&mut stream, response.as_bytes()).await;
                                    }
                                });
                            }
                            Err(e) => eprintln!("Error accepting connection: {}", e),
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        println!("SSE server shutting down");
                        break;
                    }
                }
            }
        });

        self.server_handle = Some(handle);
        self.is_connected = true;

        Ok(())
    }

    /// Handle an error by calling the error callback if set
    fn handle_error(&self, error: &MCPError) {
        if let Some(callback) = &self.on_error {
            callback(error);
        }
    }

    /// Broadcast a message to all SSE clients
    pub async fn broadcast<T: Serialize + Send + Sync>(&self, message: &T) -> Result<(), MCPError> {
        if !self.is_connected {
            let error = MCPError::Transport("Transport not connected".to_string());
            self.handle_error(&error);
            return Err(error);
        }

        let broadcaster = self.message_broadcaster.as_ref().ok_or_else(|| {
            MCPError::Transport("Message broadcaster not initialized".to_string())
        })?;

        let json = serde_json::to_string(message).map_err(|e| {
            let error = MCPError::Serialization(e.to_string());
            self.handle_error(&error);
            error
        })?;

        // Broadcast the message
        if broadcaster.send(json).is_err() {
            let error = MCPError::Transport("Failed to broadcast message".to_string());
            self.handle_error(&error);
            return Err(error);
        }

        Ok(())
    }

    /// Send a response to a specific client session
    pub async fn send_to_session<T: Serialize + Send + Sync>(
        &self,
        session_id: &str,
        message: &T,
    ) -> Result<(), MCPError> {
        if !self.is_connected {
            let error = MCPError::Transport("Transport not connected".to_string());
            self.handle_error(&error);
            return Err(error);
        }

        let active_sessions = self
            .active_sessions
            .as_ref()
            .ok_or_else(|| MCPError::Transport("Active sessions not initialized".to_string()))?;

        let json = serde_json::to_string(message).map_err(|e| {
            let error = MCPError::Serialization(e.to_string());
            self.handle_error(&error);
            error
        })?;

        let sessions = active_sessions.lock().await;
        if let Some(tx) = sessions.get(session_id) {
            if tx.send(json).await.is_err() {
                let error =
                    MCPError::Transport(format!("Failed to send to session {}", session_id));
                self.handle_error(&error);
                return Err(error);
            }
            Ok(())
        } else {
            let error = MCPError::Transport(format!("Session {} not found", session_id));
            self.handle_error(&error);
            Err(error)
        }
    }
}

impl Clone for SSETransport {
    fn clone(&self) -> Self {
        Self {
            url: self.url.clone(),
            is_connected: self.is_connected,
            sender_tx: self.sender_tx.clone(),
            on_close: None, // Callbacks cannot be cloned
            on_error: None,
            on_message: None,
            server_handle: None, // Server handle cannot be cloned
            message_broadcaster: self.message_broadcaster.clone(),
            server_shutdown_tx: self.server_shutdown_tx.clone(),
            received_messages: self.received_messages.clone(),
            message_rx: None, // Receivers cannot be cloned
            message_sender: self.message_sender.clone(),
            active_sessions: self.active_sessions.clone(),
        }
    }
}

#[async_trait]
impl Transport for SSETransport {
    async fn start(&mut self) -> Result<(), MCPError> {
        if self.is_connected {
            return Ok(());
        }

        self.start_server().await
    }

    async fn send<T: Serialize + Send + Sync>(&mut self, message: &T) -> Result<(), MCPError> {
        if !self.is_connected {
            let error = MCPError::Transport("Transport not connected".to_string());
            self.handle_error(&error);
            return Err(error);
        }

        // In server mode, send means broadcast to all clients
        self.broadcast(message).await
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

        // Shutdown the server
        if let Some(tx) = &self.server_shutdown_tx {
            let _ = tx.send(()).await;
        }

        // Wait for the server to shutdown
        if let Some(handle) = self.server_handle.take() {
            let _ = handle.await;
        }

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

// Helper function to handle SSE connections (server-side)
async fn handle_sse_connection(
    mut stream: TcpStream,
    broadcaster: Arc<broadcast::Sender<String>>,
    active_sessions: Arc<Mutex<HashMap<String, mpsc::Sender<String>>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Parse HTTP request to extract path and headers
    let mut buffer = [0; 4096];
    let n = stream.read(&mut buffer).await?;

    if n == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buffer[..n]);
    let lines: Vec<&str> = request.lines().collect();

    if lines.is_empty() {
        return Ok(());
    }

    // Extract the request path
    let request_line = lines[0];
    let parts: Vec<&str> = request_line.split_whitespace().collect();

    if parts.len() < 2 {
        return Ok(());
    }

    let path = parts[1];

    // Extract host header for constructing the message endpoint URL
    let mut host = "localhost";
    for line in &lines[1..] {
        if line.to_lowercase().starts_with("host:") {
            // Host header format can be either "Host: example.com" or "Host: example.com:8080"
            // We want to preserve any port information
            let header_value = line.splitn(2, ':').nth(1).unwrap_or("").trim();
            if !header_value.is_empty() {
                host = header_value;
                break;
            }
        }
    }

    // Make sure the path is correct
    if path != "/events" {
        let response = "HTTP/1.1 404 Not Found\r\nContent-Length: 14\r\n\r\nPath not found";
        stream.write_all(response.as_bytes()).await?;
        println!("Rejected connection to invalid path: {}", path);
        return Ok(());
    }

    // Generate a unique session ID for this connection
    let session_id = Uuid::new_v4().to_string();

    // Create a channel for this session
    let (session_tx, mut session_rx) = mpsc::channel::<String>(100);

    // Register the session
    {
        let mut sessions = active_sessions.lock().await;
        sessions.insert(session_id.clone(), session_tx);
    }

    // Send SSE headers
    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\nAccess-Control-Allow-Origin: *\r\n\r\n";
    stream.write_all(response.as_bytes()).await?;

    // Determine the message endpoint URL based on the request
    let scheme = "http"; // Default to HTTP
    let messages_endpoint = format!("{}://{}/messages?sessionId={}", scheme, host, session_id);

    // Send the endpoint event with session ID
    let endpoint_event = format!("event: endpoint\ndata: {}\n\n", messages_endpoint);
    stream.write_all(endpoint_event.as_bytes()).await?;
    stream.flush().await?;

    // Subscribe to broadcast channel
    let mut broadcast_rx = broadcaster.subscribe();

    // Send welcome message
    let welcome = serde_json::json!({
        "id": 0,
        "jsonrpc": "2.0",
        "method": "welcome",
        "params": {"message": "Connected to SSE stream", "session": session_id}
    });

    if let Ok(json) = serde_json::to_string(&welcome) {
        let sse_event = format!("event: message\ndata: {}\n\n", json);
        stream.write_all(sse_event.as_bytes()).await?;
        stream.flush().await?;
    }

    println!(
        "Client connected to SSE stream with session ID: {}",
        session_id
    );

    // Keep the connection alive until it's closed
    let mut closed = false;
    while !closed {
        tokio::select! {
            // Check for session-specific messages
            msg = session_rx.recv() => {
                match msg {
                    Some(msg) => {
                        let sse_event = format!("event: message\ndata: {}\n\n", msg);
                        if let Err(_) = stream.write_all(sse_event.as_bytes()).await {
                            closed = true;
                            break;
                        }
                        if let Err(_) = stream.flush().await {
                            closed = true;
                            break;
                        }
                    },
                    None => {
                        // Channel closed
                        closed = true;
                        break;
                    }
                }
            },
            // Check for broadcast messages
            result = broadcast_rx.recv() => {
                match result {
                    Ok(msg) => {
                        // Only forward broadcast messages that don't have a session
                        // or match this session's ID
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&msg) {
                            if value.get("session").is_none() ||
                               value.get("session") == Some(&serde_json::Value::String(session_id.clone())) {
                                let sse_event = format!("event: message\ndata: {}\n\n", msg);
                                if let Err(_) = stream.write_all(sse_event.as_bytes()).await {
                                    closed = true;
                                    break;
                                }
                                if let Err(_) = stream.flush().await {
                                    closed = true;
                                    break;
                                }
                            }
                        } else {
                            // Non-JSON messages are broadcast to everyone
                            let sse_event = format!("event: message\ndata: {}\n\n", msg);
                            if let Err(_) = stream.write_all(sse_event.as_bytes()).await {
                                closed = true;
                                break;
                            }
                            if let Err(_) = stream.flush().await {
                                closed = true;
                                break;
                            }
                        }
                    },
                    Err(_) => {
                        // Channel closed
                        closed = true;
                        break;
                    }
                }
            }
        }
    }

    // Cleanup when the connection is closed
    {
        let mut sessions = active_sessions.lock().await;
        sessions.remove(&session_id);
    }

    Ok(())
}

// Helper function to handle POST requests (server-side)
async fn handle_post_request(
    mut stream: TcpStream,
    broadcaster: Arc<broadcast::Sender<String>>,
    message_store: Arc<Mutex<Vec<String>>>,
    message_tx: Arc<mpsc::Sender<String>>,
    active_sessions: Arc<Mutex<HashMap<String, mpsc::Sender<String>>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Parse HTTP request to extract path and headers
    let mut buffer = [0; 4096];
    let n = stream.read(&mut buffer).await?;

    if n == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buffer[..n]);
    let lines: Vec<&str> = request.lines().collect();

    if lines.is_empty() {
        return Ok(());
    }

    // Extract the request path with query parameters
    let request_line = lines[0];
    let parts: Vec<&str> = request_line.split_whitespace().collect();

    if parts.len() < 2 {
        return Ok(());
    }

    // Extract path and query parameters
    let full_path = parts[1];
    let path_parts: Vec<&str> = full_path.split('?').collect();
    let path = path_parts[0];

    // Extract session ID from query parameters
    let mut session_id: Option<String> = None;
    if path_parts.len() > 1 {
        let query_string = path_parts[1];
        for param in query_string.split('&') {
            let param_parts: Vec<&str> = param.split('=').collect();
            if param_parts.len() == 2 && param_parts[0] == "sessionId" {
                session_id = Some(param_parts[1].to_string());
                break;
            }
        }
    }

    // Make sure the path is correct
    if path != "/messages" {
        let response = "HTTP/1.1 404 Not Found\r\nContent-Length: 14\r\n\r\nPath not found";
        stream.write_all(response.as_bytes()).await?;
        warn!("Rejected POST to invalid path: {}", path);
        return Ok(());
    }

    // Find Content-Length header
    let mut content_length = 0;
    for line in &lines[1..] {
        if line.to_lowercase().starts_with("content-length:") {
            if let Some(len_str) = line.split(':').nth(1) {
                if let Ok(len) = len_str.trim().parse::<usize>() {
                    content_length = len;
                }
            }
        }
    }

    // Find the body (after the empty line)
    let mut body_start = 0;
    for (i, line) in lines.iter().enumerate() {
        if line.is_empty() {
            body_start = i + 1;
            break;
        }
    }

    // Extract the body
    let body = if body_start < lines.len() {
        lines[body_start..].join("\n")
    } else {
        // If we couldn't find the body, try to find the end of headers
        let headers_end = request.find("\r\n\r\n").map(|pos| pos + 4).unwrap_or(0);

        if headers_end > 0 && headers_end < request.len() {
            request[headers_end..].to_string()
        } else {
            // If still no body found, read more data
            let mut body = vec![0; content_length];
            stream.read_exact(&mut body).await?;
            String::from_utf8_lossy(&body).to_string()
        }
    };

    // Process the message body
    let is_request = if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body) {
        // Check if it's a request (has method field)
        value.get("method").is_some()
    } else {
        false
    };

    if is_request {
        // It's a request, store it and send to the message channel for processing
        let mut messages = message_store.lock().await;
        messages.push(body.clone());

        // Send to the message channel for receive() method
        let _ = message_tx.send(body.clone()).await;

        // Don't send the request back to clients - the server will generate responses
    } else {
        // It's not a request (probably a response); just forward it to the right client
        if let Some(session) = &session_id {
            let sessions = active_sessions.lock().await;
            if let Some(tx) = sessions.get(session) {
                // This is a direct response to a specific client
                let _ = tx.send(body.clone()).await;
            }
        }
    }

    // Send HTTP response
    let response = serde_json::json!({
        "success": true,
        "message": "Message received and processed"
    });
    let json = serde_json::to_string(&response)?;
    let http_response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        json.len(),
        json
    );
    stream.write_all(http_response.as_bytes()).await?;

    Ok(())
}

/// Helper method to send a response message to a specific client
async fn send_response_to_client(
    active_sessions: &Arc<Mutex<HashMap<String, mpsc::Sender<String>>>>,
    session_id: &str,
    response: &str,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let sessions = active_sessions.lock().await;
    if let Some(tx) = sessions.get(session_id) {
        tx.send(response.to_string()).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}
