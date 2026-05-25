mod domain;
mod server;

use rmcp::{ServiceExt, transport::stdio};
use server::NotificationServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();
    let service = NotificationServer::seeded().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
