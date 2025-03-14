mod common;

use anyhow::{Context, Result};
use clap::Parser;
use common::{ScrapeAndSearchRequest, ScrapeAndSearchResponse, TOOL_NAME};
use log::{error, info};
use mcpr::{client::Client, transport::stdio::StdioTransport};
use std::io::{self, Write};

#[derive(Parser, Debug)]
#[clap(author, version, about = "MCP Web Scraper Search Client")]
struct Args {
    /// URI to scrape
    #[clap(short, long)]
    uri: Option<String>,

    /// Search term
    #[clap(short, long)]
    search_term: Option<String>,

    /// Interactive mode
    #[clap(short, long)]
    interactive: bool,
}

fn prompt_input(prompt: &str) -> Result<String> {
    print!("{}: ", prompt);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_string())
}

fn run_search(client: &mut Client<StdioTransport>, uri: &str, search_term: &str) -> Result<()> {
    info!(
        "Sending scrape and search request for URI: {}, search term: {}",
        uri, search_term
    );

    // Create the request
    let request = ScrapeAndSearchRequest {
        uri: uri.to_string(),
        search_term: search_term.to_string(),
    };

    // Send the request
    let response: ScrapeAndSearchResponse = client
        .call_tool(TOOL_NAME, &request)
        .context("Failed to call tool")?;

    // Process the response
    println!("\n=== Search Results ===");
    println!("URI: {}", response.uri);
    println!("Search Term: {}", response.search_term);

    if let Some(error) = response.error {
        println!("\nError: {}", error);
    } else {
        println!("\nResults:");
        println!("{}", response.result);
    }
    println!("=====================\n");

    Ok(())
}

fn interactive_mode(client: &mut Client<StdioTransport>) -> Result<()> {
    println!("=== Web Scraper Search Interactive Mode ===");
    println!("Type 'exit' or 'quit' to exit");

    loop {
        let uri = prompt_input("Enter URI to scrape (or 'exit' to quit)")?;
        if uri.to_lowercase() == "exit" || uri.to_lowercase() == "quit" {
            break;
        }

        let search_term = prompt_input("Enter search term")?;
        if search_term.to_lowercase() == "exit" || search_term.to_lowercase() == "quit" {
            break;
        }

        if let Err(e) = run_search(client, &uri, &search_term) {
            error!("Error during search: {}", e);
            println!("Error: {}", e);
        }

        println!("\nReady for next search...\n");
    }

    println!("Exiting interactive mode");
    Ok(())
}

fn main() -> Result<()> {
    // Initialize logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    // Parse command line arguments
    let args = Args::parse();

    // Create transport and client
    let transport = StdioTransport::new();
    let mut client = Client::new(transport);

    // Initialize the client
    client
        .initialize()
        .context("Failed to initialize MCP client")?;

    // Check if we're in interactive mode or one-shot mode
    if args.interactive {
        interactive_mode(&mut client)?;
    } else {
        // One-shot mode requires URI and search term
        let uri = args
            .uri
            .context("URI is required in non-interactive mode. Use --uri or -u")?;
        let search_term = args
            .search_term
            .context("Search term is required in non-interactive mode. Use --search-term or -s")?;

        run_search(&mut client, &uri, &search_term)?;
    }

    // Shutdown the client
    client.shutdown().context("Failed to shutdown MCP client")?;

    Ok(())
}
