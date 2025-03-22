// SSE transport module
mod client;
mod server;
mod session;

pub use client::SSEClientTransport;
pub use server::SSEServerTransport;
pub use session::{Session, SessionManager};

// Re-export for backward compatibility
pub use self::client::SSEClientTransport as SSETransport;
