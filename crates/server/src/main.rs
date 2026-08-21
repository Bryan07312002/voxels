mod metric_clients;
mod server;

use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};

use config::{ServerConfig, load_server_config};
use core_types::ViewDistance;
use server::VoxelServer;
use world_gen::{DiskChunkStore, FlatWorldGenerator, WorldManager};

use crate::metric_clients::spawn_tui_thread;

fn main() -> std::io::Result<()> {
    let (tx_metrics, rx_metrics) = mpsc::channel();
    let _ = spawn_tui_thread(rx_metrics);

    let config = load_server_config(Path::new("")).unwrap_or_else(|e| {
        eprintln!("{e}");
        ServerConfig {
            host: String::from("127.0.0.1"),
            port: 25565,
            max_view_distance: ViewDistance(16),
        }
    });

    let mut server = VoxelServer::new(
        config,
        WorldManager::new(
            FlatWorldGenerator::new(6),
            DiskChunkStore::new(PathBuf::from("/tmp/voxel")),
        ),
        tx_metrics,
    )?;
    server.run();
    Ok(())
}
