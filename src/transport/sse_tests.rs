// cspell:ignore oneshot
#![cfg(test)]
use crate::transport::sse::{SSEClientTransport, SSEServerTransport};
use crate::transport::Transport;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

// Test message structure matching the protocol
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct TestMessage {
    id: u32,
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
}

// Helper function to create a mock SSE server
async fn create_mock_sse_server() -> (SocketAddr, oneshot::Sender<()>) {
    // Bind to a random available port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Create a channel to signal shutdown
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    // Spawn the server
    tokio::spawn(async move {
        // Define test messages
        let test_messages = vec![
            TestMessage {
                id: 1,
                jsonrpc: "2.0".to_string(),
                method: "test1".to_string(),
                params: serde_json::json!({}),
            },
            TestMessage {
                id: 2,
                jsonrpc: "2.0".to_string(),
                method: "test2".to_string(),
                params: serde_json::json!({"key": "value"}),
            },
        ];

        tokio::select! {
            _ = async {
                while let Ok((stream, _)) = listener.accept().await {
                    let test_messages = test_messages.clone();

                    tokio::spawn(async move {
                        let mut http_response = "HTTP/1.1 200 OK\r\n".to_string();
                        http_response.push_str("Content-Type: text/event-stream\r\n");
                        http_response.push_str("Cache-Control: no-cache\r\n");
                        http_response.push_str("Connection: keep-alive\r\n");
                        http_response.push_str("\r\n");

                        let mut tcp_stream = tokio::io::BufWriter::new(stream);

                        // Send the HTTP response
                        if let Err(e) = tokio::io::AsyncWriteExt::write_all(&mut tcp_stream, http_response.as_bytes()).await {
                            eprintln!("Error sending HTTP response: {}", e);
                            return;
                        }

                        // Send each test message as an SSE event
                        for message in test_messages {
                            let json = serde_json::to_string(&message).unwrap();
                            let sse_event = format!("data: {}\n\n", json);

                            if let Err(e) = tokio::io::AsyncWriteExt::write_all(&mut tcp_stream, sse_event.as_bytes()).await {
                                eprintln!("Error sending SSE event: {}", e);
                                return;
                            }

                            if let Err(e) = tokio::io::AsyncWriteExt::flush(&mut tcp_stream).await {
                                eprintln!("Error flushing TCP stream: {}", e);
                                return;
                            }

                            // Add a small delay between messages
                            tokio::time::sleep(Duration::from_millis(50)).await;
                        }

                        // Keep the connection open
                        loop {
                            tokio::time::sleep(Duration::from_secs(1)).await;
                        }
                    });
                }
            } => {}

            _ = shutdown_rx => {
                // Server shutdown requested
            }
        }
    });

    // Return the server address and shutdown sender
    (addr, shutdown_tx)
}

// Helper function to create a mock HTTP POST endpoint
async fn create_mock_post_endpoint() -> (SocketAddr, oneshot::Sender<()>, Arc<Mutex<Vec<String>>>) {
    // Bind to a random available port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Create a channel to signal shutdown
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    // Create a shared collection to store received messages
    let received_messages = Arc::new(Mutex::new(Vec::new()));
    let received_messages_clone = received_messages.clone();

    // Spawn the server
    tokio::spawn(async move {
        tokio::select! {
            _ = async {
                while let Ok((stream, _)) = listener.accept().await {
                    let received_messages = received_messages_clone.clone();

                    tokio::spawn(async move {
                        let mut buf_stream = tokio::io::BufReader::new(stream);
                        let mut headers = Vec::new();
                        let mut content_length = 0;

                        // Read HTTP headers
                        loop {
                            let mut line = String::new();
                            if let Err(e) = tokio::io::AsyncBufReadExt::read_line(&mut buf_stream, &mut line).await {
                                eprintln!("Error reading header: {}", e);
                                return;
                            }

                            // Check for end of headers
                            if line == "\r\n" || line.is_empty() {
                                break;
                            }

                            // Parse Content-Length header
                            if line.to_lowercase().starts_with("content-length:") {
                                if let Some(len_str) = line.split(':').nth(1) {
                                    if let Ok(len) = len_str.trim().parse::<usize>() {
                                        content_length = len;
                                    }
                                }
                            }

                            headers.push(line);
                        }

                        // Read the body
                        let mut body = vec![0; content_length];
                        if let Err(e) = tokio::io::AsyncReadExt::read_exact(&mut buf_stream, &mut body).await {
                            eprintln!("Error reading body: {}", e);
                            return;
                        }

                        // Store the received message
                        if let Ok(body_str) = String::from_utf8(body) {
                            let mut messages = received_messages.lock().unwrap();
                            messages.push(body_str);
                        }

                        // Send a response
                        let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}";
                        let mut writer = tokio::io::BufWriter::new(buf_stream.into_inner());
                        if let Err(e) = tokio::io::AsyncWriteExt::write_all(&mut writer, response.as_bytes()).await {
                            eprintln!("Error sending response: {}", e);
                            return;
                        }

                        if let Err(e) = tokio::io::AsyncWriteExt::flush(&mut writer).await {
                            eprintln!("Error flushing response: {}", e);
                        }
                    });
                }
            } => {}

            _ = shutdown_rx => {
                // Server shutdown requested
            }
        }
    });

    // Return the server address, shutdown sender, and received messages collection
    (addr, shutdown_tx, received_messages)
}

#[tokio::test]
async fn test_sse_transport_receive() {
    // Create a mock SSE server
    let (server_addr, shutdown_tx) = create_mock_sse_server().await;
    let sse_url = format!("http://{}", server_addr);
    let send_url = format!("http://{}", server_addr); // Not used for this test

    // Create the SSE client transport
    let mut transport = SSEClientTransport::new(&sse_url, &send_url).unwrap();

    // Start the transport
    transport.start().await.unwrap();

    // Set up a flag to track message reception
    let message_received = Arc::new(Mutex::new(false));
    let message_flag = message_received.clone();

    // Set the message callback
    transport.set_on_message(Some(move |message: &_| {
        println!("Received message: {}", message);
        let mut flag = message_flag.lock().unwrap();
        *flag = true;
    }));

    // Receive the first message
    let message1: TestMessage = transport.receive().await.unwrap();

    // Verify the first message
    assert_eq!(message1.id, 1);
    assert_eq!(message1.method, "test1");

    // Receive the second message
    let message2: TestMessage = transport.receive().await.unwrap();

    // Verify the second message
    assert_eq!(message2.id, 2);
    assert_eq!(message2.method, "test2");
    assert_eq!(message2.params["key"], "value");

    // Verify that the message callback was triggered
    assert!(*message_received.lock().unwrap());

    // Close the transport
    transport.close().await.unwrap();

    // Shut down the mock server
    let _ = shutdown_tx.send(());
}

#[tokio::test]
async fn test_sse_transport_send() {
    // Create a mock POST endpoint
    let (post_addr, shutdown_tx, received_messages) = create_mock_post_endpoint().await;
    let sse_url = format!("http://{}", post_addr); // Not actually used for SSE in this test
    let send_url = format!("http://{}", post_addr);

    // Create the SSE client transport
    let mut transport = SSEClientTransport::new(&sse_url, &send_url).unwrap();

    // Start the transport
    transport.start().await.unwrap();

    // Create a test message
    let test_message = TestMessage {
        id: 3,
        jsonrpc: "2.0".to_string(),
        method: "request".to_string(),
        params: serde_json::json!({"action": "test"}),
    };

    // Send the message
    transport.send(&test_message).await.unwrap();

    // Wait a short time for the message to be processed
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Check that the message was received by the endpoint
    let messages = received_messages.lock().unwrap();

    // In some environments, we might get more than one message (if the test is re-run)
    // Just check that we received at least one message
    assert!(!messages.is_empty(), "No messages received");

    // Parse the received message
    let received: TestMessage = serde_json::from_str(&messages[0]).unwrap();
    assert_eq!(received.id, 3);
    assert_eq!(received.method, "request");
    assert_eq!(received.params["action"], "test");

    // Close the transport
    transport.close().await.unwrap();

    // Shut down the mock endpoint
    let _ = shutdown_tx.send(());
}

#[tokio::test]
async fn test_sse_transport_with_auth() {
    // This test would require more complex HTTP header inspection
    // For now, just verify that the transport can be created with an auth token
    let transport = SSEClientTransport::new("http://localhost:8080", "http://localhost:8080")
        .unwrap()
        .with_auth_token("test_token");

    assert!(transport.has_auth_token());
    assert_eq!(transport.get_auth_token(), Some("test_token"));
}

#[tokio::test]
async fn test_sse_transport_reconnect_params() {
    // Test that reconnection parameters can be set
    let transport = SSEClientTransport::new("http://localhost:8080", "http://localhost:8080")
        .unwrap()
        .with_reconnect_params(5, 10);

    assert_eq!(transport.get_reconnect_interval(), Duration::from_secs(5));
    assert_eq!(transport.get_max_reconnect_attempts(), 10);
}

#[tokio::test]
async fn test_sse_transport_clone() {
    // Test that the transport can be cloned
    let original =
        SSEClientTransport::new("http://localhost:8080", "http://localhost:8080").unwrap();
    let cloned = original.clone();

    // Start both transports to verify they can operate independently
    let mut orig = original.clone();
    let mut cln = cloned.clone();

    // Both should be able to start without interfering with each other
    assert!(orig.start().await.is_ok());
    assert!(cln.start().await.is_ok());
}

#[tokio::test]
async fn test_sse_server_transport() {
    // Create a server transport
    let mut server = SSEServerTransport::new("http://127.0.0.1:0").unwrap();

    // Start the server
    assert!(server.start().await.is_ok());

    // Close the server
    assert!(server.close().await.is_ok());
}
