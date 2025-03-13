//! Simple MCP client example

use mcpr::constants::LATEST_PROTOCOL_VERSION;
use mcpr::error::MCPError;
use mcpr::schema::{
    ClientCapabilities, Implementation, InitializeParams, JSONRPCMessage, JSONRPCNotification,
    JSONRPCRequest, RequestId,
};
use mcpr::transport::{stdio::StdioTransport, Transport};

fn main() -> Result<(), MCPError> {
    // Create a stdio transport
    let mut transport = StdioTransport::new();

    // Send initialize request
    let request_id = RequestId::Number(1);
    let initialize_params = InitializeParams {
        protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
        capabilities: ClientCapabilities {
            experimental: None,
            roots: None,
            sampling: None,
        },
        client_info: Implementation {
            name: "simple-client".to_string(),
            version: "0.1.0".to_string(),
        },
    };

    let initialize_request = JSONRPCRequest::new(
        request_id.clone(),
        "initialize".to_string(),
        Some(serde_json::to_value(initialize_params).unwrap()),
    );

    println!("Sending initialize request");
    transport.send(&initialize_request)?;

    // Wait for initialize response
    let message: JSONRPCMessage = transport.receive()?;

    match message {
        JSONRPCMessage::Response(response) => {
            println!("Received initialize response");

            // Send initialized notification
            let initialized_notification =
                JSONRPCNotification::new("notifications/initialized".to_string(), None);

            transport.send(&initialized_notification)?;
            println!("Sent initialized notification");

            // Send ping request
            let ping_request = JSONRPCRequest::new(RequestId::Number(2), "ping".to_string(), None);

            println!("Sending ping request");
            transport.send(&ping_request)?;

            // Wait for ping response
            let message: JSONRPCMessage = transport.receive()?;

            match message {
                JSONRPCMessage::Response(_) => {
                    println!("Received ping response");
                }
                JSONRPCMessage::Error(error) => {
                    println!(
                        "Received error: {} ({})",
                        error.error.message, error.error.code
                    );
                }
                _ => {
                    println!("Received unexpected message type");
                }
            }
        }
        JSONRPCMessage::Error(error) => {
            println!(
                "Received error: {} ({})",
                error.error.message, error.error.code
            );
        }
        _ => {
            println!("Received unexpected message type");
        }
    }

    Ok(())
}
