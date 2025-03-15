use anyhow::Result;
use clap::Parser;
use log::info;
use mcpr::server::{Server, ServerConfig};
use mcpr::transport::sse::SSETransport;
use mcpr::transport::stdio::StdioTransport;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

mod tools;
use tools::{GitHubTool, ReadmeQueryTool, RepoSearchTool};

/// GitHub Tools Server
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value_t = 3000)]
    port: u16,

    /// Host to bind to
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    host: String,

    /// Enable terminal UI mode
    #[arg(long)]
    terminal_ui: bool,
}

fn main() -> Result<()> {
    // Initialize logger
    env_logger::init_from_env(
        env_logger::Env::default().default_filter_or("info,github_tools_server=debug"),
    );

    // Parse command line arguments
    let args = Args::parse();
    info!("Starting GitHub Tools Server");

    // Initialize tools
    let tools: HashMap<String, Arc<dyn GitHubTool>> = HashMap::from([
        (
            "readme_query".to_string(),
            Arc::new(ReadmeQueryTool::new()) as Arc<dyn GitHubTool>,
        ),
        (
            "repo_search".to_string(),
            Arc::new(RepoSearchTool::new()) as Arc<dyn GitHubTool>,
        ),
    ]);

    // Configure the server
    let mut server_config = ServerConfig::new()
        .with_name("GitHub Tools Server")
        .with_version("0.1.0");

    // Register tools with the server config
    for (_, tool) in &tools {
        server_config = server_config.with_tool(tool.get_tool_definition());
    }

    // Create the server with the appropriate transport based on the mode
    if args.terminal_ui {
        // Use SSE transport for terminal UI mode
        let server_addr = format!("{}:{}", args.host, args.port);
        info!("Starting server with terminal UI on {}", server_addr);

        let transport = SSETransport::new(&server_addr);
        let mut server = Server::new(server_config);

        // Register tool handlers
        register_tool_handlers(&mut server, tools);

        // Start the server
        server.start(transport)?;
    } else {
        // Use stdio transport for standard mode
        info!("Starting server with stdio transport");

        let transport = StdioTransport::new();
        let mut server = Server::new(server_config);

        // Register tool handlers
        register_tool_handlers(&mut server, tools);

        // Start the server
        server.start(transport)?;
    }

    Ok(())
}

fn register_tool_handlers<T: mcpr::transport::Transport>(
    server: &mut Server<T>,
    tools: HashMap<String, Arc<dyn GitHubTool>>,
) {
    for (name, tool) in tools {
        let tool_clone = tool.clone();
        server
            .register_tool_handler(&name, move |params: Value| {
                match tool_clone.handle(params) {
                    Ok(result) => Ok(result),
                    Err(err) => Err(mcpr::error::MCPError::Protocol(err.to_string())),
                }
            })
            .expect(&format!("Failed to register tool handler for {}", name));
    }
}
