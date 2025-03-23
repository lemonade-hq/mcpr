// cspell:ignore oneshot
#![cfg(test)]
use crate::transport::sse::{SSEClientTransport, SSEServerTransport};
use crate::transport::Transport;
use hyper::header;
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
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
    let endpoint_url = format!("http://{}/messages", addr);

    // Spawn a tokio task that runs the server
    tokio::spawn(async move {
        let make_service = make_service_fn(move |_| {
            let test_messages = test_messages.clone();
            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    let test_messages = test_messages.clone();
                    async move {
                        let response = match (req.method(), req.uri().path()) {
                            (&Method::GET, "/") => {
                                let mut response = Response::new(Body::empty());
                                response.headers_mut().insert(
                                    header::CONTENT_TYPE,
                                    HeaderValue::from_static("text/event-stream"),
                                );
                                response.headers_mut().insert(
                                    header::CACHE_CONTROL,
                                    HeaderValue::from_static("no-cache"),
                                );
                                response.headers_mut().insert(
                                    header::CONNECTION,
                                    HeaderValue::from_static("keep-alive"),
                                );

                                // First, send the endpoint event
                                let endpoint_event =
                                    format!("event: endpoint\ndata: {}\n\n", endpoint_url);

                                // Then, serialize the test messages
                                let mut body = endpoint_event;
                                for msg in test_messages {
                                    let json = serde_json::to_string(&msg).unwrap();
                                    body.push_str(&format!("event: message\ndata: {}\n\n", json));
                                }

                                *response.body_mut() = Body::from(body);
                                response
                            }
                            _ => {
                                let mut response = Response::new(Body::empty());
                                *response.status_mut() = StatusCode::NOT_FOUND;
                                response
                            }
                        };
                        Ok::<_, Infallible>(response)
                    }
                }))
            }
        });

        let server = Server::builder(hyper::server::accept::from_stream(TcpListenerStream::new(
            listener,
        )))
        .serve(make_service);

        let graceful = server.with_graceful_shutdown(async {
            let _ = shutdown_rx.await;
        });

        if let Err(e) = graceful.await {
            eprintln!("Server error: {}", e);
        }
    });

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

    // Create the SSE client transport (now only needs the events URL)
    let mut transport = SSEClientTransport::new(&sse_url).unwrap();

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

    // Wait a bit for the endpoint event to be processed
    tokio::time::sleep(Duration::from_millis(100)).await;

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
    // Create a mock SSE server and POST endpoint
    let (server_addr, shutdown_tx) = create_mock_sse_server().await;
    let (post_addr, post_shutdown_tx, received_messages) = create_mock_post_endpoint().await;

    let sse_url = format!("http://{}", server_addr);

    // Create the SSE client transport (now only needs the events URL)
    let mut transport = SSEClientTransport::new(&sse_url).unwrap();

    // Start the transport
    transport.start().await.unwrap();

    // Wait a bit for the endpoint event to be processed
    tokio::time::sleep(Duration::from_millis(100)).await;

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
    transport.send(&message).await.unwrap();

    // Wait a bit for the message to be received by the server
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify that the message was received by the server
    let messages = received_messages.lock().await;
    assert_eq!(messages.len(), 1);

    // Parse the JSON and verify the content
    let received: TestMessage = serde_json::from_str(&messages[0]).unwrap();
    assert_eq!(received.id, 3);
    assert_eq!(received.method, "test3");
    assert_eq!(received.params["key"], "value");
    assert_eq!(received.params["nested"]["nestedKey"], "nestedValue");

    // Close the transport
    transport.close().await.unwrap();

    // Shut down the mock servers
    let _ = shutdown_tx.send(());
    let _ = post_shutdown_tx.send(());
}

#[tokio::test]
async fn test_sse_transport_with_auth() {
    // This test is now just testing the builder method with auth
    let transport = SSEClientTransport::new("http://localhost:8080")
        .unwrap()
        .with_auth_token("test_token");

    assert!(transport.has_auth_token());
    assert_eq!(transport.get_auth_token(), Some("test_token"));
}

#[tokio::test]
async fn test_sse_transport_reconnect_params() {
    // This test is now just testing the builder method with reconnect params
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
