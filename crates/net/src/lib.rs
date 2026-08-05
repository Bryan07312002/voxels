use std::collections::BTreeMap;
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;
use std::net::ToSocketAddrs;

pub use rkyv::check_archived_root;
use rkyv::{AlignedVec, Archive, Deserialize, Serialize};

use core_types::CHUNK_SIZE;

pub const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE; // 4096
const MAX_PAYLOAD_SIZE: usize = 1200; // MTU safe chunking size

// --- 1. Domain Packets ---

#[derive(Archive, Serialize, Deserialize, Debug, Clone)]
#[archive(check_bytes)]
pub enum ClientPacket {
    RequestChunk { x: i32, y: i32, z: i32 },
    PlayerPosition { x: f32, y: f32, z: f32 },
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone)]
#[archive(check_bytes)]
pub enum ServerPacket {
    ChunkData {
        x: i32,
        y: i32,
        z: i32,
        blocks: Box<[u16; CHUNK_VOLUME]>,
    },
    Pong,
}

// --- 2. Abbreviated Low-Level Fragment Manager ---

/// Handles sending and reassembling fragmented UDP packets transparently
pub struct UdpChannel {
    socket: UdpSocket,
    /// Pending incoming fragments grouped by total count key
    fragments: BTreeMap<u32, Vec<u8>>,
}

impl UdpChannel {
    pub fn bind(addr: &str) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(addr)?;
        socket.set_read_timeout(Some(Duration::from_millis(100)))?;
        Ok(Self {
            socket,
            fragments: BTreeMap::new(),
        })
    }

    /// High-level generic serializer + sender over UDP
    pub fn send_packet<T>(&self, packet: &T, target: impl ToSocketAddrs) -> std::io::Result<()>
    where
        T: rkyv::Serialize<rkyv::ser::serializers::AllocSerializer<1024>>,
    {
        let serialized = rkyv::to_bytes::<_, 1024>(packet)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        let chunks = serialized.chunks(MAX_PAYLOAD_SIZE);
        let total_chunks = chunks.len() as u32;

        for (idx, chunk) in chunks.enumerate() {
            let mut datagram = Vec::with_capacity(8 + chunk.len());
            datagram.extend_from_slice(&(idx as u32).to_be_bytes());
            datagram.extend_from_slice(&total_chunks.to_be_bytes());
            datagram.extend_from_slice(chunk);

            self.socket.send_to(&datagram, &target)?;
        }

        Ok(())
    }

    /// Reads raw datagrams, reassembles fragments, and returns aligned payload once full packet arrives
    pub fn recv_raw_payload(&mut self) -> std::io::Result<(AlignedVec, SocketAddr)> {
        let mut raw_buf = [0u8; MAX_PAYLOAD_SIZE + 64];

        loop {
            let (amt, src_addr) = self.socket.recv_from(&mut raw_buf)?;
            if amt < 8 {
                continue;
            }

            let chunk_idx = u32::from_be_bytes(raw_buf[0..4].try_into().unwrap());
            let total_chunks = u32::from_be_bytes(raw_buf[4..8].try_into().unwrap());
            let payload = &raw_buf[8..amt];

            self.fragments.insert(chunk_idx, payload.to_vec());

            // If all fragments arrived, reassemble into an AlignedVec
            if self.fragments.len() == total_chunks as usize {
                let mut aligned = AlignedVec::new();
                for (_, fragment) in std::mem::take(&mut self.fragments) {
                    aligned.extend_from_slice(&fragment);
                }
                return Ok((aligned, src_addr));
            }
        }
    }
}
