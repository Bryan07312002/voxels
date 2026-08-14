use world_gen::{TerrainGenerator, FlatWorldGenerator};
use core_types::ChunkPos;
use std::net::SocketAddr;
use net::{
    check_archived_root, ArchivedClientPacket, ClientPacket, ServerPacket, UdpChannel,
    CHUNK_VOLUME,
};

struct VoxelServer {
    channel: UdpChannel,
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
        println!("Voxel UDP Server active!");

        loop {
            let (aligned_bytes, sender_addr) = match self.channel.recv_raw_payload() {
                Ok(res) => res,
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => continue,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                Err(err) => {
                    eprintln!("Network receive error: {err}");
                    continue;
                }
            };

            if let Ok(archived) = check_archived_root::<ClientPacket>(&aligned_bytes) {
                self.handle_packet(archived, sender_addr);
            }
        }
    }

    fn handle_packet(&mut self, packet: &ArchivedClientPacket, sender: SocketAddr) {
        match packet {
            ArchivedClientPacket::RequestChunk { x, y, z } => {
                let (x, y, z) = (*x, *y, *z);
                println!("📥 [Server] RequestChunk [{x}, {y}, {z}] from {sender}");

                let chunk = self.generator.generate_chunk(ChunkPos{ x, y, z });

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
