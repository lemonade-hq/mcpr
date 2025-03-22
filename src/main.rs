//! MCP CLI tool for generating server and client stubs

use clap::{Parser, Subcommand};
use env_logger;
use log::{error, info, warn};
use mcpr::{
    client::Client,
    error::MCPError,
    schema::common::Tool,
    server::{Server, ServerConfig},
    transport::{
        sse::{SSEClientTransport, SSEServerTransport},
        stdio::StdioTransport,
        Transport,
    },
};
use std::path::PathBuf;

/// MCP CLI tool for generating server and client stubs
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a server stub
    GenerateServer {
        /// Name of the server
        #[arg(short, long)]
        name: String,

        /// Output directory
        #[arg(short, long, default_value = ".")]
        output: String,
    },

    /// Generate a client stub
    GenerateClient {
        /// Name of the client
        #[arg(short, long)]
        name: String,

        /// Output directory
        #[arg(short, long, default_value = ".")]
        output: String,
    },

    /// Generate a complete "hello mcp" project with both client and server
    GenerateProject {
        /// Name of the project
        #[arg(short, long)]
        name: String,

        /// Output directory
        #[arg(short, long, default_value = ".")]
        output: String,

        /// Transport type to use (stdio, sse, websocket)
        #[arg(short, long, default_value = "stdio")]
        transport: String,
    },

    /// Run a server
    RunServer {
        /// Port to listen on
        #[arg(short, long, default_value_t = 8080)]
        port: u16,

        /// Transport type to use (stdio, sse, websocket)
        #[arg(short, long, default_value = "stdio")]
        transport: String,

        /// Enable debug mode
        #[arg(long)]
        debug: bool,
    },

    /// Connect to a server as a client
    Connect {
        /// URI of the server to connect to
        #[arg(short, long)]
        uri: String,

        /// Run in interactive mode
        #[arg(short, long)]
        interactive: bool,

        /// Name to use for greeting
        #[arg(short, long, default_value = "Default User")]
        name: String,

        /// Transport type to use (stdio, sse, websocket)
        #[arg(short, long)]
        transport: String,

        /// Operation to perform (interactive, repl, tool_name)
        #[arg(short, long)]
        operation: Option<String>,

        /// Parameters for the operation
        #[arg(short, long)]
        params: Option<String>,
    },

    /// Validate an MCP message
    Validate {
        /// Path to the message file
        #[arg(short, long)]
        path: String,
    },
}

/// Connect command parameters
#[derive(Debug, Clone)]
struct Connect {
    uri: String,
    interactive: bool,
    name: String,
    transport: String,
    operation: Option<String>,
    params: Option<String>,
}

/// Main entry point of the CLI application
#[tokio::main]
async fn main() -> Result<(), MCPError> {
    // Initialize logging
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    // Parse command-line arguments
    let cli = Cli::parse();

    match cli.command {
        Commands::GenerateServer { name, output } => {
            info!(
                "Generating server stub with name '{}' to '{}'",
                name, output
            );
            let _output_path = PathBuf::from(output.clone());

            // TODO: Generate server stub
            Err(MCPError::UnsupportedFeature(
                "Server stub generation not yet implemented".to_string(),
            ))
        }
        Commands::GenerateClient { name, output } => {
            info!(
                "Generating client stub with name '{}' to '{}'",
                name, output
            );
            let _output_path = PathBuf::from(output.clone());

            // TODO: Generate client stub
            Err(MCPError::UnsupportedFeature(
                "Client stub generation not yet implemented".to_string(),
            ))
        }
        Commands::GenerateProject {
            name,
            output,
            transport,
        } => {
            info!(
                "Generating project '{}' in '{}' with transport '{}'",
                name, output, transport
            );
            let _output_path = PathBuf::from(output.clone());

            // TODO: Generate project
            Err(MCPError::UnsupportedFeature(
                "Project generation not yet implemented".to_string(),
            ))
        }
        Commands::RunServer {
            port,
            transport,
            debug,
        } => run_server(port, &transport, debug).await,
        Commands::Connect {
            uri,
            interactive,
            name,
            transport,
            operation,
            params,
        } => {
            run_client(Connect {
                uri,
                interactive,
                name,
                transport,
                operation,
                params,
            })
            .await
        }
        Commands::Validate { path } => {
            info!("Validating message from '{}'", path);
            // TODO: Implement message validation
            info!("Message validation not yet implemented");
            Err(MCPError::UnsupportedFeature(
                "Message validation not yet implemented".to_string(),
            ))
        }
    }
}

/// Run the server with the specified configuration
async fn run_server(port: u16, transport_type: &str, debug: bool) -> Result<(), MCPError> {
    info!(
        "Running server on port {} with transport type {}",
        port, transport_type
    );

    if debug {
        info!("Debug mode enabled");
    }

    // Depending on the transport type, start the appropriate server
    match transport_type {
        "stdio" => {
            info!("Starting server with stdio transport");
            // TODO: Implement stdio server
            Err(MCPError::UnsupportedFeature(
                "Stdio server not yet implemented".to_string(),
            ))
        }
        "sse" => {
            info!("Starting server with SSE transport on port {}", port);

            // Create a URI for the SSE server
            let uri = format!("http://0.0.0.0:{}", port);

            // Create the SSE transport
            let transport = SSEServerTransport::new(&uri)?;

            // Configure a basic echo tool
            let echo_tool = Tool {
                name: "echo".to_string(),
                description: Some("Echo tool".to_string()),
                input_schema: mcpr::schema::common::ToolInputSchema {
                    r#type: "object".to_string(),
                    properties: Some(
                        [(
                            "message".to_string(),
                            serde_json::json!({
                                "type": "string",
                                "description": "Message to echo"
                            }),
                        )]
                        .into_iter()
                        .collect(),
                    ),
                    required: Some(vec!["message".to_string()]),
                },
            };

            // Create server config
            let server_config = ServerConfig::new()
                .with_name("MCP SSE Server")
                .with_version("1.0.0")
                .with_tool(echo_tool);

            // Create and start the server
            let mut server = Server::new(server_config);

            // Register the echo tool handler
            server.register_tool_handler("echo", |params| async move {
                let message = params
                    .get("message")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| MCPError::Protocol("Missing message parameter".to_string()))?;

                info!("Echo request: {}", message);

                Ok(serde_json::json!({"result": message}))
            })?;

            // Start the server
            info!("Starting SSE server on {}", uri);
            server.serve(transport).await
        }
        "websocket" => {
            info!("Starting server with WebSocket transport on port {}", port);
            // TODO: Implement WebSocket server
            Err(MCPError::UnsupportedFeature(
                "WebSocket server not yet implemented".to_string(),
            ))
        }
        _ => {
            error!("Unsupported transport type: {}", transport_type);
            Err(MCPError::Transport(format!(
                "Unsupported transport type: {}",
                transport_type
            )))
        }
    }
}

/// Run the client with the specified configuration
async fn run_client(cmd: Connect) -> Result<(), MCPError> {
    info!("Connecting to {}", cmd.uri);

    // Handle different transport types directly
    match cmd.transport.as_str() {
        "sse" => {
            info!("SSE transport is only supported for servers");
            Err(MCPError::Transport(
                "SSE transport is only supported for servers".to_string(),
            ))
        }
        "stdio" => {
            info!("Using stdio transport");
            let transport = StdioTransport::new();
            let mut client = Client::new(transport);
            handle_client_session(&mut client, cmd).await
        }
        _ => Err(MCPError::Transport(format!(
            "Unsupported transport type: {}",
            cmd.transport
        ))),
    }
}

// Helper function to handle the client session logic
async fn handle_client_session<T: Transport + Send + Sync>(
    client: &mut Client<T>,
    cmd: Connect,
) -> Result<(), MCPError> {
    // Initialize the client
    client.initialize().await?;

    // Display server information if available
    info!("Connected to server");

    // Get available tools by calling the list_tools method
    let tools_result = client
        .call_tool::<_, serde_json::Value>("list_tools", &serde_json::Value::Null)
        .await;

    let _tools = match tools_result {
        Ok(response) => {
            if let Some(tools_array) = response.as_array() {
                info!("Available tools:");
                for tool in tools_array {
                    if let (Some(name), Some(description)) = (
                        tool.get("name").and_then(|n| n.as_str()),
                        tool.get("description").and_then(|d| d.as_str()),
                    ) {
                        info!(" - {}: {}", name, description);
                    }
                }
                tools_array.to_vec()
            } else {
                info!("No tools available");
                Vec::new()
            }
        }
        Err(e) => {
            warn!("Could not retrieve tools: {}", e);
            Vec::new()
        }
    };

    // Handle requested operations
    match cmd.operation.as_deref() {
        Some("interactive") => {
            info!("Starting interactive session");
            // Interactive session logic here
            Err(MCPError::UnsupportedFeature(
                "Interactive session not implemented yet".to_string(),
            ))
        }
        Some("repl") => {
            info!("Starting REPL");
            // REPL session logic here
            Err(MCPError::UnsupportedFeature(
                "REPL not implemented yet".to_string(),
            ))
        }
        Some(tool_name) => {
            info!("Calling tool: {}", tool_name);

            // Parse parameters
            let params = if let Some(params_str) = cmd.params.as_ref() {
                serde_json::from_str(params_str)
                    .map_err(|e| MCPError::Transport(format!("Invalid parameters: {}", e)))?
            } else {
                serde_json::Value::Null
            };

            // Call the requested tool
            match client
                .call_tool::<_, serde_json::Value>(tool_name, &params)
                .await
            {
                Ok(response) => {
                    println!("{}", serde_json::to_string_pretty(&response).unwrap());
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
        None => {
            // Default to the hello tool if no operation is specified
            info!("No operation specified, using hello tool");

            // Use empty params for hello
            let params = serde_json::Value::Null;

            // Call the hello tool
            let response: serde_json::Value = client.call_tool("hello", &params).await?;

            println!("{}", serde_json::to_string_pretty(&response).unwrap());
            Ok(())
        }
    }?;

    // Shut down the client
    client.shutdown().await?;

    Ok(())
}
