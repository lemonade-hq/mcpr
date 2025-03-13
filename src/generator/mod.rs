//! Module for generating MCP server and client stubs

use std::fs;
use std::io;
use std::path::Path;

mod templates;

/// Error type for generator operations
#[derive(Debug, thiserror::Error)]
pub enum GeneratorError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Template error: {0}")]
    Template(String),
    #[error("Invalid name: {0}")]
    InvalidName(String),
}

/// Generate a server stub
pub fn generate_server(name: &str, output_dir: &Path) -> Result<(), GeneratorError> {
    // Validate name
    if !is_valid_name(name) {
        return Err(GeneratorError::InvalidName(format!(
            "Invalid server name: {}",
            name
        )));
    }

    // Create output directory if it doesn't exist
    let server_dir = output_dir.join(name);
    fs::create_dir_all(&server_dir)?;

    // Generate server files
    generate_server_main(&server_dir, name)?;
    generate_server_cargo_toml(&server_dir, name)?;
    generate_server_readme(&server_dir, name)?;

    println!(
        "Server stub '{}' generated successfully in '{}'",
        name,
        server_dir.display()
    );
    println!("To run the server:");
    println!("  cd {}", server_dir.display());
    println!("  cargo run");

    Ok(())
}

/// Generate a client stub
pub fn generate_client(name: &str, output_dir: &Path) -> Result<(), GeneratorError> {
    // Validate name
    if !is_valid_name(name) {
        return Err(GeneratorError::InvalidName(format!(
            "Invalid client name: {}",
            name
        )));
    }

    // Create output directory if it doesn't exist
    let client_dir = output_dir.join(name);
    fs::create_dir_all(&client_dir)?;

    // Generate client files
    generate_client_main(&client_dir, name)?;
    generate_client_cargo_toml(&client_dir, name)?;
    generate_client_readme(&client_dir, name)?;

    println!(
        "Client stub '{}' generated successfully in '{}'",
        name,
        client_dir.display()
    );
    println!("To run the client:");
    println!("  cd {}", client_dir.display());
    println!("  cargo run -- --uri <server_uri>");

    Ok(())
}

// Helper functions

fn is_valid_name(name: &str) -> bool {
    // Check if name is a valid Rust crate name
    if name.is_empty() {
        return false;
    }

    // Must start with a letter
    if !name.chars().next().unwrap().is_ascii_alphabetic() {
        return false;
    }

    // Can only contain letters, numbers, and underscores
    name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn generate_server_main(server_dir: &Path, name: &str) -> Result<(), GeneratorError> {
    let main_rs = server_dir.join("src").join("main.rs");
    fs::create_dir_all(main_rs.parent().unwrap())?;

    let content = templates::SERVER_MAIN_TEMPLATE.replace("{{name}}", name);

    fs::write(main_rs, content)?;
    Ok(())
}

fn generate_server_cargo_toml(server_dir: &Path, name: &str) -> Result<(), GeneratorError> {
    let cargo_toml = server_dir.join("Cargo.toml");

    let content = templates::SERVER_CARGO_TEMPLATE.replace("{{name}}", name);

    fs::write(cargo_toml, content)?;
    Ok(())
}

fn generate_server_readme(server_dir: &Path, name: &str) -> Result<(), GeneratorError> {
    let readme = server_dir.join("README.md");

    let content = templates::SERVER_README_TEMPLATE.replace("{{name}}", name);

    fs::write(readme, content)?;
    Ok(())
}

fn generate_client_main(client_dir: &Path, name: &str) -> Result<(), GeneratorError> {
    let main_rs = client_dir.join("src").join("main.rs");
    fs::create_dir_all(main_rs.parent().unwrap())?;

    let content = templates::CLIENT_MAIN_TEMPLATE.replace("{{name}}", name);

    fs::write(main_rs, content)?;
    Ok(())
}

fn generate_client_cargo_toml(client_dir: &Path, name: &str) -> Result<(), GeneratorError> {
    let cargo_toml = client_dir.join("Cargo.toml");

    let content = templates::CLIENT_CARGO_TEMPLATE.replace("{{name}}", name);

    fs::write(cargo_toml, content)?;
    Ok(())
}

fn generate_client_readme(client_dir: &Path, name: &str) -> Result<(), GeneratorError> {
    let readme = client_dir.join("README.md");

    let content = templates::CLIENT_README_TEMPLATE.replace("{{name}}", name);

    fs::write(readme, content)?;
    Ok(())
}
