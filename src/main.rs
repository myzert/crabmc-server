use crabmc_server::{ServerConfig, CrabServer};
use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup PaperMC-like logging
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,crabmc_server=debug")),
        )
        .with(
            fmt::layer()
                .with_target(false)
                .with_thread_ids(false)
                .compact(),
        )
        .init();

    info!("Initializing CrabMC server environment...");

    // Load or generate server.properties
    let config = ServerConfig::load_or_create("server.properties")?;

    let mut server = CrabServer::new(config);

    // Run the server
    if let Err(e) = server.run().await {
        tracing::error!("CrabMC server encountered a fatal error: {}", e);
    }

    Ok(())
}
