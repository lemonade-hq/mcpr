// cspell:ignore oneshot
#![cfg(test)]
use crate::transport::sse::{SSEClientTransport, SSEServerTransport};
use crate::transport::Transport;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::net::TcpListener;
use tokio::sync::{oneshot, Mutex};

// Test message structure matching the protocol
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct TestMessage {
    id: u32,
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
}

// Helper function to create a mock SSE server using tokio's TcpListener directly
async fn create_mock_sse_server(
    post_addr: Option<SocketAddr>,
) -> (SocketAddr, oneshot::Sender<()>) {
    // Create a shutdown channel
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    // Build a Vec of test messages to send
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
            params: serde_json::json!({
                "key": "value"
            }),
        },
    ];

    // Create an HTTP server that serves SSE
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Spawn a tokio task that runs the server
    tokio::spawn(async move {
        let test_messages = test_messages.clone();

        tokio::select! {
            _ = async {
                while let Ok((mut stream, _)) = listener.accept().await {
                    let test_messages = test_messages.clone();

                    tokio::spawn(async move {
                        // Send HTTP response header
                        let http_response = "HTTP/1.1 200 OK\r\n\
                                            Content-Type: text/event-stream\r\n\
                                            Cache-Control: no-cache\r\n\
                                            Connection: keep-alive\r\n\
                                            \r\n";

                        let mut writer = BufWriter::new(&mut stream);

                        // Write headers
                        writer.write_all(http_response.as_bytes()).await.unwrap();

                        // First, send the endpoint event with the SEND URL for messages
                        // Use the provided post_addr if available, otherwise construct from the current address
                        let endpoint_url = if let Some(post_address) = post_addr {
                            format!("http://{}", post_address)
                        } else {
                            format!("http://{}/messages", addr)
                        };

                        let endpoint_event = format!("event: endpoint\ndata: {}\n\n", endpoint_url);
                        println!("Sending endpoint event: {}", endpoint_event);
                        writer.write_all(endpoint_event.as_bytes()).await.unwrap();
                        writer.flush().await.unwrap();

                        // Send each test message as an SSE event
                        for msg in test_messages {
                            let json = serde_json::to_string(&msg).unwrap();
                            let sse_event = format!("event: message\ndata: {}\n\n", json);

                            writer.write_all(sse_event.as_bytes()).await.unwrap();
                            writer.flush().await.unwrap();

                            // Add a small delay between messages
                            tokio::time::sleep(Duration::from_millis(50)).await;
                        }

                        // Keep the connection open until the test is done
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

    (addr, shutdown_tx)
}

// Helper function to create a mock HTTP POST endpoint
async fn create_mock_post_endpoint() -> (SocketAddr, oneshot::Sender<()>, Arc<Mutex<Vec<String>>>) {
    // Bind to a random available port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    println!("POST endpoint listening on: {}", addr);

    // Create a channel to signal shutdown
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    // Create a shared collection to store received messages
    let received_messages = Arc::new(Mutex::new(Vec::new()));
    let received_messages_clone = received_messages.clone();

    // Spawn the server
    tokio::spawn(async move {
        tokio::select! {
            _ = async {
                println!("POST endpoint awaiting connections");
                while let Ok((stream, _)) = listener.accept().await {
                    println!("POST endpoint received a connection");
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

                            println!("Header: {}", line);

                            // Check for end of headers
                            if line == "\r\n" || line.is_empty() {
                                break;
                            }

                            // Parse Content-Length header
                            if line.to_lowercase().starts_with("content-length:") {
                                if let Some(len_str) = line.split(':').nth(1) {
                                    if let Ok(len) = len_str.trim().parse::<usize>() {
                                        content_length = len;
                                        println!("Content length: {}", content_length);
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
                            println!("Received body: {}", body_str);
                            let mut messages = received_messages.lock().await;
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
                println!("POST endpoint shutdown requested");
            }
        }
    });

    // Return the server address, shutdown sender, and received messages collection
    (addr, shutdown_tx, received_messages)
}

// Test for sending SSE messages
#[tokio::test]
async fn test_sse_transport_send() {
    // First create the POST endpoint to get its address
    let (post_addr, post_shutdown_tx, received_messages) = create_mock_post_endpoint().await;
    println!("POST endpoint listening on: {}", post_addr);

    // Now create the SSE server with the POST endpoint address
    let (server_addr, shutdown_tx) = create_mock_sse_server(Some(post_addr)).await;

    let sse_url = format!("http://{}", server_addr);
    println!("SSE server URL: {}", sse_url);
    println!("POST endpoint address: {}", post_addr);

    // Create the SSE client transport with auth token
    let mut transport = SSEClientTransport::new(&sse_url)
        .unwrap()
        .with_auth_token("test-token");

    // Start the transport
    println!("Starting transport");
    transport.start().await.unwrap();

    // Wait for the endpoint event to be processed
    println!("Waiting for endpoint event to be processed...");
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Create a test message to send
    let message = TestMessage {
        id: 3,
        jsonrpc: "2.0".to_string(),
        method: "test3".to_string(),
        params: serde_json::json!({
            "key": "value",
            "nested": {
                "nestedKey": "nestedValue"
            }
        }),
    };

    // Send the message
    println!("Sending message");
    match transport.send(&message).await {
        Ok(_) => println!("Message sent successfully"),
        Err(e) => println!("Error sending message: {:?}", e),
    }

    // Wait for the message to be received by the server
    println!("Waiting for the message to be received...");
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify that the message was received by the server
    let messages = received_messages.lock().await;
    println!("Received messages count: {}", messages.len());
    assert_eq!(messages.len(), 1, "Expected one message to be received");

    // Parse the JSON and verify the content
    if !messages.is_empty() {
        let received: TestMessage = serde_json::from_str(&messages[0]).unwrap();
        assert_eq!(received.id, 3);
        assert_eq!(received.method, "test3");
        assert_eq!(received.params["key"], "value");
        assert_eq!(received.params["nested"]["nestedKey"], "nestedValue");
    }

    // Close the transport
    println!("Closing transport");
    transport.close().await.unwrap();

    // Shut down the mock servers
    println!("Shutting down mock servers");
    let _ = shutdown_tx.send(());
    let _ = post_shutdown_tx.send(());
}

#[tokio::test]
async fn test_sse_transport_with_auth() {
    // This test is just testing the builder method with auth
    let transport = SSEClientTransport::new("http://localhost:8080")
        .unwrap()
        .with_auth_token("test_token");

    assert!(transport.has_auth_token());
    assert_eq!(transport.get_auth_token(), Some("test_token"));
}

#[tokio::test]
async fn test_sse_transport_reconnect_params() {
    // This test is just testing the builder method with reconnect params
    let transport = SSEClientTransport::new("http://localhost:8080")
        .unwrap()
        .with_reconnect_params(10, 3);

    assert_eq!(transport.get_reconnect_interval(), Duration::from_secs(10));
    assert_eq!(transport.get_max_reconnect_attempts(), 3);
}

#[tokio::test]
async fn test_sse_transport_clone() {
    let original = SSEClientTransport::new("http://localhost:8080").unwrap();
    let cloned = original.clone();

    // Verify the cloned transport has the same configuration
    assert_eq!(
        original.get_reconnect_interval(),
        cloned.get_reconnect_interval()
    );
    assert_eq!(
        original.get_max_reconnect_attempts(),
        cloned.get_max_reconnect_attempts()
    );
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
