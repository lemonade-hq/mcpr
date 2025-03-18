use log::info;
use mcpr::{
    error::MCPError,
    schema::common::{Tool, ToolInputSchema},
    server::{Server, ServerConfig},
    transport::websocket::WebSocketTransport,
};
use serde_json::json;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Notify;

#[tokio::main]
async fn main() -> Result<(), MCPError> {
    // Initialize logging
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    // Create a transport for WebSocket server (listens on localhost:8080)
    let transport = WebSocketTransport::new_server("127.0.0.1:8080");

    // Create an echo tool
    let echo_tool = Tool {
        name: "echo".to_string(),
        description: Some("Echoes back the input".to_string()),
        input_schema: ToolInputSchema {
            r#type: "object".to_string(),
            properties: Some(
                [(
                    "message".to_string(),
                    json!({
                        "type": "string",
                        "description": "The message to echo"
                    }),
                )]
                .into_iter()
                .collect::<HashMap<_, _>>(),
            ),
            required: Some(vec!["message".to_string()]),
        },
    };

    // Configure the server
    let server_config = ServerConfig::new()
        .with_name("WebSocket Echo Server")
        .with_version("1.0.0")
        .with_tool(echo_tool);

    // Create the server
    let mut server = Server::new(server_config);

    // Register the echo tool handler
    server.register_tool_handler("echo", |params| async move {
        // Extract the message parameter
        let message = params
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MCPError::Protocol("Missing message parameter".to_string()))?;

        info!("Echo request: {}", message);

        // Return the message as the result
        Ok(json!({
            "result": message
        }))
    })?;

    // Create a shutdown signal
    let shutdown = Arc::new(Notify::new());
    let shutdown_clone = shutdown.clone();

    // Handle Ctrl+C
    tokio::spawn(async move {
        if let Ok(()) = tokio::signal::ctrl_c().await {
            info!("Received Ctrl+C, shutting down...");
            shutdown_clone.notify_one();
        }
    });

    // Start the server in a separate task
    let server_task = tokio::spawn(async move {
        info!("WebSocket server running on ws://127.0.0.1:8080");
        info!("Press Ctrl+C to exit");
        server.serve(transport).await
    });

    // Wait for shutdown signal
    shutdown.notified().await;

    // Join the server task
    match server_task.await {
        Ok(result) => result?,
        Err(e) => {
            eprintln!("Server task failed: {}", e);
        }
    }

    info!("Server shut down gracefully");
    Ok(())
}
