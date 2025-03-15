//! MCP Client for github-tools project with stdio transport
//!
//! This client demonstrates how to connect to an MCP server using stdio transport.
//!
//! There are two ways to use this client:
//! 1. Connect to an already running server (recommended for production)
//! 2. Start a new server process and connect to it (convenient for development)
//!
//! The client supports both interactive and one-shot modes.

use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, error, info, warn};
use mcpr::{
    error::MCPError,
    schema::json_rpc::{JSONRPCMessage, JSONRPCRequest, JSONRPCResponse, RequestId},
    transport::{stdio::StdioTransport, Transport},
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use std::{
    error::Error,
    io::{self, BufRead, BufReader, Write},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

/// CLI arguments
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Enable debug output
    #[arg(short, long)]
    debug: bool,

    /// Server command to execute (if not connecting to an existing server)
    #[arg(
        short,
        long,
        default_value = "./server/target/debug/github-tools-server"
    )]
    server_cmd: String,

    /// Connect to an already running server instead of starting a new one
    #[arg(short, long)]
    connect: bool,

    /// Run in interactive mode
    #[arg(short, long)]
    interactive: bool,

    /// Use terminal UI (deprecated, will use interactive mode instead)
    #[arg(short, long)]
    terminal_ui: bool,

    /// GitHub repository in format owner/repo
    #[arg(
        short,
        long,
        value_name = "OWNER/REPO",
        help = "GitHub repository in format owner/repo (e.g., rust-lang/rust)"
    )]
    repo: Option<String>,

    /// Query to search for in the repository
    #[arg(
        short,
        long,
        help = "Query to search for (e.g., \"What is Rust used for?\" or \"machine learning\")"
    )]
    query: Option<String>,

    /// Timeout in seconds for operations
    #[arg(short = 'T', long, default_value = "30")]
    timeout: u64,
}

/// High-level MCP client
struct Client<T: Transport> {
    transport: T,
    next_request_id: i64,
}

impl<T: Transport> Client<T> {
    /// Create a new MCP client with the given transport
    fn new(transport: T) -> Self {
        Self {
            transport,
            next_request_id: 1,
        }
    }

    /// Initialize the client
    fn initialize(&mut self) -> Result<Value, MCPError> {
        // Start the transport
        debug!("Starting transport");
        self.transport.start()?;

        // Send initialization request
        let initialize_request = JSONRPCRequest::new(
            self.next_request_id(),
            "initialize".to_string(),
            Some(serde_json::json!({
                "protocol_version": mcpr::constants::LATEST_PROTOCOL_VERSION
            })),
        );

        let message = JSONRPCMessage::Request(initialize_request);
        debug!("Sending initialize request: {:?}", message);
        self.transport.send(&message)?;

        // Wait for response
        info!("Waiting for initialization response");
        let response: JSONRPCMessage = self.transport.receive()?;
        debug!("Received response: {:?}", response);

        match response {
            JSONRPCMessage::Response(resp) => Ok(resp.result),
            JSONRPCMessage::Error(err) => {
                error!("Initialization failed: {:?}", err);
                Err(MCPError::Protocol(format!(
                    "Initialization failed: {:?}",
                    err
                )))
            }
            _ => {
                error!("Unexpected response type");
                Err(MCPError::Protocol("Unexpected response type".to_string()))
            }
        }
    }

    /// Call a tool on the server
    fn call_tool<P: Serialize + std::fmt::Debug, R: DeserializeOwned>(
        &mut self,
        tool_name: &str,
        params: &P,
    ) -> Result<R, MCPError> {
        // Create tool call request
        let tool_call_request = JSONRPCRequest::new(
            self.next_request_id(),
            "tool_call".to_string(),
            Some(serde_json::json!({
                "name": tool_name,
                "parameters": serde_json::to_value(params)?
            })),
        );

        let message = JSONRPCMessage::Request(tool_call_request);
        info!("Calling tool '{}' with parameters: {:?}", tool_name, params);
        debug!("Sending tool call request: {:?}", message);
        self.transport.send(&message)?;

        // Wait for response
        info!("Waiting for tool call response");
        let response: JSONRPCMessage = self.transport.receive()?;
        debug!("Received response: {:?}", response);

        match response {
            JSONRPCMessage::Response(resp) => {
                // Extract the tool result from the response
                let result_value = resp.result;
                let result = result_value.get("result").ok_or_else(|| {
                    error!("Missing 'result' field in response");
                    MCPError::Protocol("Missing 'result' field in response".to_string())
                })?;

                // Parse the result
                debug!("Parsing result: {:?}", result);
                serde_json::from_value(result.clone()).map_err(|e| {
                    error!("Failed to parse result: {}", e);
                    MCPError::Serialization(e)
                })
            }
            JSONRPCMessage::Error(err) => {
                error!("Tool call failed: {:?}", err);
                Err(MCPError::Protocol(format!("Tool call failed: {:?}", err)))
            }
            _ => {
                error!("Unexpected response type");
                Err(MCPError::Protocol("Unexpected response type".to_string()))
            }
        }
    }

    /// Shutdown the client
    fn shutdown(&mut self) -> Result<(), MCPError> {
        // Send shutdown request
        let shutdown_request =
            JSONRPCRequest::new(self.next_request_id(), "shutdown".to_string(), None);

        let message = JSONRPCMessage::Request(shutdown_request);
        info!("Sending shutdown request");
        debug!("Shutdown request: {:?}", message);
        self.transport.send(&message)?;

        // Wait for response
        info!("Waiting for shutdown response");
        let response: JSONRPCMessage = self.transport.receive()?;
        debug!("Received response: {:?}", response);

        match response {
            JSONRPCMessage::Response(_) => {
                // Close the transport
                info!("Closing transport");
                self.transport.close()?;
                Ok(())
            }
            JSONRPCMessage::Error(err) => {
                error!("Shutdown failed: {:?}", err);
                Err(MCPError::Protocol(format!("Shutdown failed: {:?}", err)))
            }
            _ => {
                error!("Unexpected response type");
                Err(MCPError::Protocol("Unexpected response type".to_string()))
            }
        }
    }

    /// Generate the next request ID
    fn next_request_id(&mut self) -> RequestId {
        let id = self.next_request_id;
        self.next_request_id += 1;
        RequestId::Number(id)
    }
}

/// Connect to an already running server
fn connect_to_running_server(
    command: &str,
    args: &[&str],
) -> Result<(StdioTransport, Option<Child>)> {
    info!(
        "Connecting to running server with command: {} {}",
        command,
        args.join(" ")
    );
    println!(
        "{}",
        format!("Connecting to running server: {}", command).cyan()
    );

    // Start a new process that will connect to the server
    let mut process = Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn process")?;

    // Create a stderr reader to monitor server output
    if let Some(stderr) = process.stderr.take() {
        let stderr_reader = BufReader::new(stderr);
        thread::spawn(move || {
            for line in stderr_reader.lines() {
                if let Ok(line) = line {
                    debug!("Server stderr: {}", line);
                }
            }
        });
    }

    // Give the server a moment to start up
    thread::sleep(Duration::from_millis(500));

    // Create a transport that communicates with the server process
    let transport = StdioTransport::with_reader_writer(
        Box::new(process.stdout.take().context("Failed to get stdout")?),
        Box::new(process.stdin.take().context("Failed to get stdin")?),
    );

    Ok((transport, Some(process)))
}

/// Start a new server and connect to it
fn start_and_connect_to_server(server_cmd: &str) -> Result<(StdioTransport, Option<Child>)> {
    info!("Starting server process: {}", server_cmd);
    println!(
        "{}",
        format!("Starting server process: {}", server_cmd).cyan()
    );

    // Show a spinner while starting the server
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.blue} {msg}")
            .unwrap(),
    );
    spinner.set_message("Starting server...");
    spinner.enable_steady_tick(Duration::from_millis(100));

    // Start the server process
    let mut server_process = Command::new(server_cmd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn server process")?;

    // Create a stderr reader to monitor server output
    if let Some(stderr) = server_process.stderr.take() {
        let stderr_reader = BufReader::new(stderr);
        thread::spawn(move || {
            for line in stderr_reader.lines() {
                if let Ok(line) = line {
                    debug!("Server stderr: {}", line);
                }
            }
        });
    }

    // Give the server a moment to start up
    thread::sleep(Duration::from_millis(500));

    let server_stdin = server_process.stdin.take().context("Failed to get stdin")?;
    let server_stdout = server_process
        .stdout
        .take()
        .context("Failed to get stdout")?;

    info!("Using stdio transport");
    let transport =
        StdioTransport::with_reader_writer(Box::new(server_stdout), Box::new(server_stdin));

    spinner.finish_with_message("Server started successfully!");

    Ok((transport, Some(server_process)))
}

fn prompt_input(prompt: &str, default: Option<&str>) -> Result<String> {
    let theme = ColorfulTheme::default();
    let mut input = Input::<String>::with_theme(&theme).with_prompt(prompt);

    if let Some(default_value) = default {
        input = input.default(default_value.to_string());
    }

    let result = input.interact_text().context("Failed to get user input")?;

    Ok(result)
}

fn select_tool(tools: &[Value]) -> Result<usize> {
    let tool_names: Vec<String> = tools
        .iter()
        .map(|t| {
            let name = t["name"].as_str().unwrap_or("Unknown");
            let desc = t["description"].as_str().unwrap_or("No description");
            format!("{}: {}", name, desc)
        })
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select a tool to use")
        .default(0)
        .items(&tool_names)
        .interact()?;

    Ok(selection)
}

fn run_interactive_mode(client: &mut Client<StdioTransport>, init_result: Value) -> Result<()> {
    // Extract tools from initialization result
    let tools = init_result["tools"].as_array().cloned().unwrap_or_default();
    if tools.is_empty() {
        return Err(anyhow::anyhow!("No tools available from the server"));
    }

    // Print welcome message with fancy styling
    println!("\n{}", "╭───────────────────────────────────────╮".cyan());
    println!("{}", "│     GitHub Tools Interactive Client    │".cyan());
    println!("{}", "╰───────────────────────────────────────╯".cyan());

    println!("\n{}", "Available tools:".yellow().bold());
    for (i, tool) in tools.iter().enumerate() {
        let name = tool["name"].as_str().unwrap_or("Unknown");
        let desc = tool["description"].as_str().unwrap_or("No description");
        println!("  {}. {} - {}", i + 1, name.green().bold(), desc);
    }

    // Main interaction loop
    loop {
        println!("\n{}", "What would you like to do?".cyan().bold());
        let options = vec![
            "Query a GitHub repository README",
            "Search for GitHub repositories",
            "Exit",
        ];

        for (i, option) in options.iter().enumerate() {
            println!("  {}. {}", i + 1, option);
        }

        print!("\n{} ", "Enter your choice [1]:".yellow());
        io::stdout().flush().unwrap();

        let mut choice = String::new();
        io::stdin().read_line(&mut choice).unwrap();
        let choice = choice.trim();

        let choice_num = if choice.is_empty() {
            1
        } else {
            choice.parse::<usize>().unwrap_or(1)
        };

        match choice_num {
            1 => query_repository(client, &tools)?,
            2 => search_repositories(client, &tools)?,
            3 => {
                println!("{}", "Exiting...".green());
                break;
            }
            _ => {
                println!("{}", "Invalid choice. Please try again.".red());
                continue;
            }
        }

        println!("\n{}", "─".repeat(50).cyan());
    }

    Ok(())
}

fn query_repository(client: &mut Client<StdioTransport>, tools: &[Value]) -> Result<()> {
    // Find the readme_query tool
    let tool_index = tools
        .iter()
        .position(|t| t["name"].as_str() == Some("readme_query"))
        .ok_or_else(|| anyhow::anyhow!("readme_query tool not found"))?;

    let tool_name = "readme_query";

    println!("\n{}", "Query GitHub Repository README".yellow().bold());
    println!(
        "{}",
        "This tool allows you to ask questions about a GitHub repository based on its README."
            .italic()
    );

    // Get repository
    print!(
        "{} ",
        "Enter repository (owner/repo) [rust-lang/rust]:".cyan()
    );
    io::stdout().flush().unwrap();
    let mut repo = String::new();
    io::stdin().read_line(&mut repo).unwrap();
    let repo = repo.trim();
    let repo = if repo.is_empty() {
        "rust-lang/rust"
    } else {
        repo
    };

    // Get query
    print!(
        "{} ",
        "Enter your question [What is this project about?]:".cyan()
    );
    io::stdout().flush().unwrap();
    let mut query = String::new();
    io::stdin().read_line(&mut query).unwrap();
    let query = query.trim();
    let query = if query.is_empty() {
        "What is this project about?"
    } else {
        query
    };

    // Prepare parameters
    let params = serde_json::json!({
        "repo": repo,
        "query": query
    });

    // Show a spinner while processing
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    spinner.set_message(format!("Querying repository {}...", repo.cyan()));
    spinner.enable_steady_tick(Duration::from_millis(100));

    // Call the tool
    match client.call_tool::<Value, Value>(tool_name, &params) {
        Ok(result) => {
            spinner.finish_with_message(format!("✅ Query completed successfully!"));

            // Extract and display the result
            let repo = result["repository"].as_str().unwrap_or("unknown");
            let query = result["query"].as_str().unwrap_or("unknown");
            let answer = result["answer"].as_str().unwrap_or("No answer found");

            println!("\n{}", "Query Results:".green().bold());
            println!("{}: {}", "Repository".yellow().bold(), repo.cyan());
            println!("{}: {}", "Question".yellow().bold(), query.cyan());
            println!("\n{}", "Answer:".yellow().bold());
            println!("{}", answer);

            Ok(())
        }
        Err(e) => {
            spinner.finish_with_message(format!("❌ Error: {}", e));
            println!("{}", format!("Error querying repository: {}", e).red());
            Ok(())
        }
    }
}

fn search_repositories(client: &mut Client<StdioTransport>, tools: &[Value]) -> Result<()> {
    // Find the repo_search tool
    let tool_index = tools
        .iter()
        .position(|t| t["name"].as_str() == Some("repo_search"))
        .ok_or_else(|| anyhow::anyhow!("repo_search tool not found"))?;

    let tool_name = "repo_search";

    println!("\n{}", "Search GitHub Repositories".yellow().bold());
    println!(
        "{}",
        "This tool allows you to search for GitHub repositories based on keywords.".italic()
    );

    // Get search query
    print!("{} ", "Enter search query [rust language]:".cyan());
    io::stdout().flush().unwrap();
    let mut query = String::new();
    io::stdin().read_line(&mut query).unwrap();
    let query = query.trim();
    let query = if query.is_empty() {
        "rust language"
    } else {
        query
    };

    // Get limit
    print!("{} ", "Enter result limit [5]:".cyan());
    io::stdout().flush().unwrap();
    let mut limit_str = String::new();
    io::stdin().read_line(&mut limit_str).unwrap();
    let limit_str = limit_str.trim();
    let limit = if limit_str.is_empty() {
        5
    } else {
        limit_str.parse::<u32>().unwrap_or(5)
    };

    // Prepare parameters
    let params = serde_json::json!({
        "query": query,
        "limit": limit
    });

    // Show a spinner while processing
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    spinner.set_message(format!(
        "Searching for repositories matching '{}'...",
        query.cyan()
    ));
    spinner.enable_steady_tick(Duration::from_millis(100));

    // Call the tool
    match client.call_tool::<Value, Value>(tool_name, &params) {
        Ok(result) => {
            spinner.finish_with_message(format!("✅ Search completed successfully!"));

            // Extract and display the result
            let query = result["query"].as_str().unwrap_or("unknown");
            let count = result["count"].as_u64().unwrap_or(0);
            let empty_vec = Vec::new();
            let repos = result["repositories"].as_array().unwrap_or(&empty_vec);

            println!("\n{}", "Search Results:".green().bold());
            println!("{}: {}", "Query".yellow().bold(), query.cyan());
            println!(
                "{}: {}",
                "Found".yellow().bold(),
                format!("{} repositories", count).cyan()
            );

            println!("\n{}", "Repositories:".yellow().bold());
            for (i, repo) in repos.iter().enumerate() {
                let name = repo["name"].as_str().unwrap_or("unknown");
                let desc = repo["description"].as_str().unwrap_or("No description");
                let stars = repo["stars"].as_u64().unwrap_or(0);
                let language = repo["language"].as_str().unwrap_or("unknown");
                let url = repo["url"].as_str().unwrap_or("");

                println!(
                    "  {}. {} ({})",
                    i + 1,
                    name.green().bold(),
                    url.blue().underline()
                );
                println!("     {}", desc);
                println!("     {} stars, Language: {}", stars, language.cyan());
                println!("     {}", "---".dimmed());
            }

            Ok(())
        }
        Err(e) => {
            spinner.finish_with_message(format!("❌ Error: {}", e));
            println!("{}", format!("Error searching repositories: {}", e).red());
            Ok(())
        }
    }
}

fn run_one_shot_mode(
    client: &mut Client<StdioTransport>,
    args: &Args,
    init_result: Value,
) -> Result<()> {
    // Extract tools from initialization result
    let tools = init_result["tools"].as_array().cloned().unwrap_or_default();

    // Determine which tool to use based on the arguments
    let (tool_name, params) = if let Some(repo) = &args.repo {
        if let Some(query) = &args.query {
            // If both repo and query are provided, use readme_query
            (
                "readme_query",
                serde_json::json!({
                    "repo": repo,
                    "query": query
                }),
            )
        } else {
            return Err(anyhow::anyhow!(
                "Query is required when repository is specified"
            ));
        }
    } else if let Some(query) = &args.query {
        // If only query is provided, use repo_search
        (
            "repo_search",
            serde_json::json!({
                "query": query,
                "limit": 5
            }),
        )
    } else {
        return Err(anyhow::anyhow!(
            "Either repository and query, or just query must be specified"
        ));
    };

    // Show a spinner while processing
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    spinner.set_message(format!("Processing with {}...", tool_name));
    spinner.enable_steady_tick(Duration::from_millis(100));

    // Call the tool
    match client.call_tool::<Value, Value>(tool_name, &params) {
        Ok(result) => {
            spinner.finish_with_message(format!("{} completed successfully!", tool_name));

            // Format and display the result
            println!("\n{}", format_result(&result, tool_name));

            Ok(())
        }
        Err(e) => {
            spinner.finish_with_message(format!("Error: {}", e));
            Err(anyhow::anyhow!("Error calling tool: {}", e))
        }
    }
}

fn format_result(result: &Value, tool_name: &str) -> String {
    match tool_name {
        "readme_query" => {
            let repo = result["repository"].as_str().unwrap_or("unknown");
            let query = result["query"].as_str().unwrap_or("unknown");
            let answer = result["answer"].as_str().unwrap_or("No answer found");

            format!(
                "{}\n{} {}\n{} {}\n\n{}\n{}",
                "Query Results:".green().bold(),
                "Repository:".yellow().bold(),
                repo.cyan(),
                "Question:".yellow().bold(),
                query.cyan(),
                "Answer:".yellow().bold(),
                answer
            )
        }
        "repo_search" => {
            let query = result["query"].as_str().unwrap_or("unknown");
            let count = result["count"].as_u64().unwrap_or(0);
            let empty_vec = Vec::new();
            let repos = result["repositories"].as_array().unwrap_or(&empty_vec);

            let mut output = format!(
                "{}\n{} {}\n{} {}\n\n{}\n",
                "Search Results:".green().bold(),
                "Query:".yellow().bold(),
                query.cyan(),
                "Found:".yellow().bold(),
                format!("{} repositories", count).cyan(),
                "Repositories:".yellow().bold()
            );

            for (i, repo) in repos.iter().enumerate() {
                let name = repo["name"].as_str().unwrap_or("unknown");
                let desc = repo["description"].as_str().unwrap_or("No description");
                let stars = repo["stars"].as_u64().unwrap_or(0);
                let language = repo["language"].as_str().unwrap_or("unknown");
                let url = repo["url"].as_str().unwrap_or("");

                output.push_str(&format!(
                    "  {}. {} ({})\n     {}\n     {} stars, Language: {}\n     {}\n",
                    i + 1,
                    name.green().bold(),
                    url.blue().underline(),
                    desc,
                    stars,
                    language.cyan(),
                    "---".dimmed()
                ));
            }

            output
        }
        _ => serde_json::to_string_pretty(result)
            .unwrap_or_else(|_| "Failed to format result".to_string()),
    }
}

fn main() -> Result<()> {
    // Initialize logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    // Parse command line arguments
    let args = Args::parse();

    // Set log level based on debug flag
    if args.debug {
        log::set_max_level(log::LevelFilter::Debug);
        debug!("Debug logging enabled");
    }

    // Print welcome message
    println!("{}", "=== GitHub Tools Client ===".green().bold());

    // Set timeout
    let timeout = Duration::from_secs(args.timeout);
    info!("Operation timeout set to {} seconds", args.timeout);

    // Create transport and server process based on connection mode
    let (transport, mut server_process) = if args.connect {
        info!("Connecting to already running server");
        connect_to_running_server(&args.server_cmd, &[])
            .context("Failed to connect to running server")?
    } else {
        info!("Starting new server process");
        start_and_connect_to_server(&args.server_cmd).context("Failed to start server process")?
    };

    let mut client = Client::new(transport);

    // Initialize the client with timeout
    info!("Initializing client...");
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.blue} {msg}")
            .unwrap(),
    );
    spinner.set_message("Initializing client...");
    spinner.enable_steady_tick(Duration::from_millis(100));

    let start_time = Instant::now();
    let init_result = loop {
        if start_time.elapsed() >= timeout {
            spinner.finish_with_message("Initialization timed out!");
            return Err(anyhow::anyhow!(
                "Initialization timed out after {:?}",
                timeout
            ));
        }

        match client.initialize() {
            Ok(result) => {
                info!("Server info: {:?}", result);
                spinner.finish_with_message("Client initialized successfully!");
                break result;
            }
            Err(e) => {
                warn!("Initialization attempt failed: {}", e);
                thread::sleep(Duration::from_millis(500));
                continue;
            }
        }
    };

    // Run the appropriate mode - default to interactive mode unless explicitly requested otherwise
    let result = if args.terminal_ui {
        // Terminal UI mode is deprecated, use interactive mode instead
        println!(
            "{}",
            "Terminal UI mode is deprecated, using interactive mode instead.".yellow()
        );
        run_interactive_mode(&mut client, init_result)
    } else if args.repo.is_some() || args.query.is_some() {
        // One-shot mode if repo or query is specified
        run_one_shot_mode(&mut client, &args, init_result)
    } else {
        // Interactive mode by default
        run_interactive_mode(&mut client, init_result)
    };

    // Shutdown the client
    info!("Shutting down client");
    let shutdown_spinner = ProgressBar::new_spinner();
    shutdown_spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.blue} {msg}")
            .unwrap(),
    );
    shutdown_spinner.set_message("Shutting down client...");
    shutdown_spinner.enable_steady_tick(Duration::from_millis(100));

    if let Err(e) = client.shutdown() {
        error!("Error during shutdown: {}", e);
        shutdown_spinner.finish_with_message(format!("Error during shutdown: {}", e));
    } else {
        info!("Client shutdown complete");
        shutdown_spinner.finish_with_message("Client shutdown complete!");
    }

    // If we started the server, terminate it gracefully
    if let Some(mut process) = server_process {
        info!("Terminating server process...");
        println!("{}", "Terminating server process...".cyan());
        let _ = process.kill();
    }

    // Return the result from the mode we ran
    result
}
