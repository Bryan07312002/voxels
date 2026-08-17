use core_types::ChunkPos;
use net::{
    ArchivedClientPacket, CHUNK_VOLUME, ClientPacket, ServerPacket, UdpChannel, check_archived_root,
};
use std::{net::SocketAddr, thread, time::Duration};
use world_gen::{FlatWorldGenerator, TerrainGenerator};

pub struct VoxelServer {
    pub channel: UdpChannel,
    generator: FlatWorldGenerator,
}

impl VoxelServer {
    pub fn new(bind_addr: &str, flat_world_height: i32) -> std::io::Result<Self> {
        Ok(Self {
            channel: UdpChannel::bind(bind_addr)?,
            generator: FlatWorldGenerator::new(flat_world_height),
        })
    }

    pub fn run(&mut self) {
        println!(
            "Voxel UDP Server active on {}",
            self.channel.socket.local_addr().unwrap()
        );

        loop {
            match self.channel.recv_raw_payload() {
                Ok((aligned_bytes, sender_addr)) => {
                    if let Ok(archived) = check_archived_root::<ClientPacket>(&aligned_bytes) {
                        self.handle_packet(archived, sender_addr);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // Prevent 100% CPU thread-spinning when idle
                    thread::sleep(Duration::from_millis(1));
                }
                Err(err) => {
                    eprintln!("Network receive error: {err}");
                }
            }
        }
    }

    fn handle_packet(&mut self, packet: &ArchivedClientPacket, sender: SocketAddr) {
        match packet {
            ArchivedClientPacket::RequestChunk { x, y, z } => {
                // Convert rkyv archived integers to native types
                let (x, y, z) = (*x, *y, *z);
                println!("📥 [Server] RequestChunk [{x}, {y}, {z}] from {sender}");

                let chunk = self.generator.generate_chunk(ChunkPos { x, y, z });

                let mut blocks = Box::new([0u16; CHUNK_VOLUME]);
                for (dst, src) in blocks.iter_mut().zip(chunk.blocks.iter()) {
                    *dst = src.0;
                }

                let response = ServerPacket::ChunkData { x, y, z, blocks };
                if let Err(e) = self.channel.send_packet(&response, sender) {
                    eprintln!("Failed to send chunk response to {sender}: {e}");
                }
            }
            ArchivedClientPacket::PlayerPosition { .. } => {}
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut server = VoxelServer::new("127.0.0.1:25565", 6)?;
    server.run();
    Ok(())
}
