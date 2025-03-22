use crate::error::MCPError;
use crate::transport::sse::session::SessionManager;
use crate::transport::{CloseCallback, ErrorCallback, MessageCallback, Transport};
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio::task::JoinHandle;
use url::Url;

/// Server-Sent Events (SSE) Server Transport
pub struct SSEServerTransport {
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

    /// Server handle
    server_handle: Option<JoinHandle<()>>,

    /// Session manager
    session_manager: SessionManager,

    /// Shutdown channel
    server_shutdown_tx: Option<mpsc::Sender<()>>,

    /// Store for received messages
    received_messages: Arc<Mutex<Vec<String>>>,

    /// Channel for receiving messages
    message_rx: Option<mpsc::Receiver<String>>,

    /// Sender for the message channel
    message_sender: Arc<mpsc::Sender<String>>,
}

impl SSEServerTransport {
    /// Create a new SSE transport in server mode
    pub fn new(url: &str) -> Result<Self, MCPError> {
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

        // Create session manager
        let session_manager = SessionManager::new(broadcaster);

        Ok(Self {
            url,
            is_connected: false,
            sender_tx,
            on_close: None,
            on_error: None,
            on_message: None,
            server_handle: None,
            session_manager,
            server_shutdown_tx: None,
            received_messages,
            message_rx: Some(message_rx),
            message_sender,
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

        // Clone for the server task
        let session_manager = self.session_manager.clone();
        let received_messages = self.received_messages.clone();
        let message_sender = self.message_sender.clone();

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
                                let session_mgr = session_manager.clone();
                                let messages = received_messages.clone();
                                let task_message_tx = message_sender.clone();

                                tokio::spawn(async move {
                                    // Peek to determine request type
                                    let mut stream = stream;
                                    let mut peek_buffer = [0; 128];
                                    let n = match stream.peek(&mut peek_buffer).await {
                                        Ok(n) => n,
                                        Err(_) => return,
                                    };

                                    let peek_str = String::from_utf8_lossy(&peek_buffer[..n]);

                                    // Extract host header for constructing the message endpoint URL
                                    let mut host = "localhost";
                                    if let Some(host_pos) = peek_str.to_lowercase().find("\r\nhost:") {
                                        let host_line = &peek_str[host_pos + 7..];
                                        if let Some(end_pos) = host_line.find("\r\n") {
                                            host = host_line[..end_pos].trim();
                                        }
                                    }

                                    // Extract session ID from query parameters
                                    let mut session_id = None;
                                    if peek_str.to_lowercase().contains("sessionid=") {
                                        if let Some(session_pos) = peek_str.find("sessionId=") {
                                            let session_part = &peek_str[session_pos + 10..];
                                            if let Some(end_pos) = session_part.find(|c: char| c == '&' || c == ' ' || c == '\r') {
                                                session_id = Some(session_part[..end_pos].to_string());
                                            }
                                        }
                                    }

                                    // Handle based on request type
                                    if peek_str.starts_with("GET") {
                                        // Handle SSE connection
                                        let _ = session_mgr.handle_sse_connection(stream, host).await;
                                    } else if peek_str.starts_with("POST") {
                                        // Handle POST request
                                        let _ = session_mgr.handle_post_request(stream, messages, task_message_tx, session_id).await;
                                    } else {
                                        // Unknown method - 405 Method Not Allowed
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

        let json = serde_json::to_string(message).map_err(|e| {
            let error = MCPError::Serialization(e.to_string());
            self.handle_error(&error);
            error
        })?;

        // Broadcast the message
        if let Err(e) = self.session_manager.broadcast(&json) {
            let error = MCPError::Transport(e);
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

        let json = serde_json::to_string(message).map_err(|e| {
            let error = MCPError::Serialization(e.to_string());
            self.handle_error(&error);
            error
        })?;

        // Send to the specific session
        match self
            .session_manager
            .send_to_session(session_id, &json)
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                let error = MCPError::Transport(e);
                self.handle_error(&error);
                Err(error)
            }
        }
    }
}

impl Clone for SSEServerTransport {
    fn clone(&self) -> Self {
        Self {
            url: self.url.clone(),
            is_connected: self.is_connected,
            sender_tx: self.sender_tx.clone(),
            on_close: None, // Callbacks cannot be cloned
            on_error: None,
            on_message: None,
            server_handle: None, // Server handle cannot be cloned
            session_manager: SessionManager::new(self.session_manager.broadcaster()),
            server_shutdown_tx: self.server_shutdown_tx.clone(),
            received_messages: self.received_messages.clone(),
            message_rx: None, // Receivers cannot be cloned
            message_sender: self.message_sender.clone(),
        }
    }
}

#[async_trait]
impl Transport for SSEServerTransport {
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

        // In server mode, broadcast to all clients
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
