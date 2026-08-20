mod metric_clients;
mod server;

use std::{path::Path, sync::mpsc};

use config::{ServerConfig, load_server_config};
use core_types::ViewDistance;
use server::VoxelServer;

use crate::metric_clients::spawn_tui_thread;

fn main() -> std::io::Result<()> {
    // Create an unbounded channel for server -> TUI metrics
    let (tx_metrics, rx_metrics) = mpsc::channel();

    // Spawn the TUI on a separate thread
    let _ = spawn_tui_thread(rx_metrics);

    let config = load_server_config(Path::new("")).unwrap_or_else(|e| {
        eprintln!("{e}");
        ServerConfig {
            host: String::from("127.0.0.1"),
            port: 25565,
            max_view_distance: ViewDistance(12),
        }
    });

    let mut server = VoxelServer::new(config, tx_metrics)?;
    server.run();
    Ok(())
}
