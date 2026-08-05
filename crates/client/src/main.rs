// crates/client/src/main.rs
use net::{
    check_archived_root, ArchivedServerPacket, ClientPacket, ServerPacket, UdpChannel,
};

struct VoxelClient {
    channel: UdpChannel,
    server_addr: String,
}

impl VoxelClient {
    pub fn new(server_addr: &str) -> std::io::Result<Self> {
        Ok(Self {
            channel: UdpChannel::bind("127.0.0.1:0")?,
            server_addr: server_addr.to_string(),
        })
    }

    pub fn request_chunk(&self, x: i32, y: i32, z: i32) -> std::io::Result<()> {
        let req = ClientPacket::RequestChunk { x, y, z };
        self.channel.send_packet(&req, &self.server_addr)
    }

    pub fn poll_network(&mut self) {
        if let Ok((aligned_payload, _)) = self.channel.recv_raw_payload() {
            if let Ok(archived) = check_archived_root::<ServerPacket>(&aligned_payload) {
                match archived {
                    ArchivedServerPacket::ChunkData { x, y, z, blocks } => {
                        println!("⚡ Received ChunkData [{}, {}, {}]!", x, y, z);
                        println!("   First Block: {}, Total Blocks: {}", blocks[0], blocks.len());
                    }
                    ArchivedServerPacket::Pong => {}
                }
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    println!("🎮 Initializing Voxel Client...");
    let mut client = VoxelClient::new("127.0.0.1:25565")?;

    client.request_chunk(0, 0, 0)?;
    println!("📤 Chunk requested. Polling response...");

    // Main Game Loop simulation
    loop {
        client.poll_network();
    }
}
