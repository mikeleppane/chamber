use chamber_vault::{Vault, VaultManager};
use color_eyre::Result;
use std::sync::Once;
static TRACING_INIT: Once = Once::new();

pub async fn handle_api_command(bind: String, port: Option<u16>) -> Result<()> {
    use chamber_api::ApiServer;
    use tracing::{info, warn};

    // Initialize tracing if not already done
    TRACING_INIT.call_once(|| {
        tracing_subscriber::fmt::init();
    });

    // Parse bind address and port
    let bind_address = if let Some(port) = port {
        // Extract IP from a bind address if it contains a port
        let ip = if bind.contains(':') {
            bind.split(':').next().unwrap_or("127.0.0.1")
        } else {
            &bind
        };
        format!("{ip}:{port}")
    } else {
        bind
    };

    println!("🚀 Starting Chamber API server...");
    println!("📡 Binding to: {bind_address}");

    // Open the vault
    let vault = Vault::open_default()?;
    let vault_manager = VaultManager::new()?;

    // Create and start the API server
    let api_server = ApiServer::new(vault, vault_manager, &bind_address).await?;
    let addr = api_server.local_addr()?;

    println!("✅ Chamber API server running on http://{addr}");
    println!("📖 Health check: http://{addr}/api/v1/health");
    println!("🔒 Login endpoint: http://{addr}/api/v1/auth/login");
    println!("📚 Use your master password to authenticate and get a JWT token");
    println!("⚡ API supports all vault operations: items, passwords, import/export");
    println!("📋 Press Ctrl+C to stop the server");
    println!();

    // Example curl commands
    println!("Example usage:");
    println!("  # Health check");
    println!("  curl http://{addr}/api/v1/health");
    println!();
    println!("  # Login (replace 'your_password' with your master password)");
    println!("  curl -X POST http://{addr}/api/v1/auth/login \\");
    println!("    -H 'Content-Type: application/json' \\");
    println!("    -d '{{\"master_password\": \"your_password\"}}'");
    println!();
    println!("  # List items (replace YOUR_TOKEN with the token from login)");
    println!("  curl -H 'Authorization: Bearer YOUR_TOKEN' \\");
    println!("    http://{addr}/api/v1/items");
    println!();

    // Graceful shutdown handling
    let shutdown_signal = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        info!("Shutdown signal received");
    };

    tokio::select! {
        result = api_server.serve() => {
            if let Err(e) = result {
                warn!("API server error: {}", e);
                return Err(e);
            }
        }
        () = shutdown_signal => {
            info!("Shutting down API server...");
            println!("👋 API server shutting down...");
        }
    }

    Ok(())
}
