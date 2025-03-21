use log::info;
use mcpr::{
    error::MCPError,
    schema::common::{Tool, ToolInputSchema},
    server::{Server, ServerConfig},
    transport::stdio::StdioTransport,
};
use serde_json::{json, Value};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), MCPError> {
    // Initialize logging
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    // Create a transport
    let transport = StdioTransport::new();

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

    // Create a hello tool
    let hello_tool = Tool {
        name: "hello".to_string(),
        description: Some("Says hello to someone".to_string()),
        input_schema: ToolInputSchema {
            r#type: "object".to_string(),
            properties: Some(
                [(
                    "name".to_string(),
                    json!({
                        "type": "string",
                        "description": "The name to greet"
                    }),
                )]
                .into_iter()
                .collect::<HashMap<_, _>>(),
            ),
            required: Some(vec!["name".to_string()]),
        },
    };

    // Configure the server
    let server_config = ServerConfig::new()
        .with_name("Echo Server")
        .with_version("1.0.0")
        .with_tool(echo_tool)
        .with_tool(hello_tool);

    // Create the server
    let mut server = Server::new(server_config);

    // Register tool handlers
    server.register_tool_handler("echo", |params: Value| async move {
        let message = params
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MCPError::Protocol("Missing message parameter".to_string()))?;

        info!("Echo request: {}", message);

        Ok(json!({
            "result": message
        }))
    })?;

    server.register_tool_handler("hello", |params: Value| async move {
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MCPError::Protocol("Missing name parameter".to_string()))?;

        info!("Hello request for name: {}", name);

        Ok(json!({
            "result": format!("Hello, {}!", name)
        }))
    })?;

    // Start the server
    info!("Starting echo server...");
    server.serve(transport).await?;

    Ok(())
}
