mod server;
mod metric_clients;

use std::sync::mpsc;

use server::VoxelServer;

use crate::metric_clients::spawn_tui_thread;

fn main() -> std::io::Result<()> {
    // Create an unbounded channel for server -> TUI metrics
    let (tx_metrics, rx_metrics) = mpsc::channel();

    // Spawn the TUI on a separate thread
    let _ = spawn_tui_thread(rx_metrics);

    let mut server = VoxelServer::new("127.0.0.1:25565", 6, 8, tx_metrics)?;
    server.run();
    Ok(())
}
