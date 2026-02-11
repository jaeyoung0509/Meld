use std::net::SocketAddr;

use alloy_server::AlloyServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    AlloyServer::new()
        .with_addr(SocketAddr::from(([127, 0, 0, 1], 4000)))
        .run()
        .await?;
    Ok(())
}
