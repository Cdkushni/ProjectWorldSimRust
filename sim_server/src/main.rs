use anyhow::Result;
use std::time::Duration;
use tokio::time::interval;
use tracing::{info, warn};

mod simulation;
use simulation::Simulation;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🌍 Starting World Simulation Server");

    // Create simulation
    let mut simulation = Simulation::new().await?;

    info!("✅ Simulation initialized");

    // Spawn API server task
    let _api_handle = {
        let admin_api = simulation.get_admin_api_server();
        tokio::spawn(async move {
            if let Err(e) = admin_api.serve("127.0.0.1:8080").await {
                warn!("Admin API server error: {}", e);
            }
        })
    };

    info!("🌐 Admin API listening on http://127.0.0.1:8080");

    // Main simulation loop
    let mut tick_interval = interval(Duration::from_millis(100)); // 10 Hz
    let mut slow_tick_interval = interval(Duration::from_secs(1)); // 1 Hz
    let mut very_slow_tick_interval = interval(Duration::from_secs(60)); // 1/min

    info!("🚀 Simulation running");

    loop {
        tokio::select! {
            _ = tick_interval.tick() => {
                // Fast tick (real-time systems)
                simulation.tick_fast(0.1).await?;
            }
            _ = slow_tick_interval.tick() => {
                // Slow tick (economy, utility AI)
                simulation.tick_slow(1.0).await?;
            }
            _ = very_slow_tick_interval.tick() => {
                // Very slow tick (ecology, demographics)
                simulation.tick_very_slow(60.0).await?;
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Received shutdown signal");
                break;
            }
        }
    }

    info!("💾 Saving world state...");
    simulation.save_snapshot().await?;

    info!("👋 Simulation shutdown complete");
    Ok(())
}

