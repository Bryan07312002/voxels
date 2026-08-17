use std::collections::BTreeMap;
use std::net::ToSocketAddrs;
use std::net::{SocketAddr, UdpSocket};

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
    pub socket: UdpSocket,
    /// Pending incoming fragments grouped by total count key
    fragments: BTreeMap<u32, Vec<u8>>,
}

impl UdpChannel {
    pub fn bind(addr: &str) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(addr)?;
        socket.set_nonblocking(true)?;

        Ok(Self {
            socket,
            fragments: BTreeMap::new(),
        })
    }

    pub fn send_packet<T>(&self, packet: &T, target: SocketAddr) -> std::io::Result<()>
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

            // Directly pass the concrete SocketAddr
            self.socket.send_to(&datagram, target)?;
        }

        Ok(())
    }

    pub fn recv_raw_payload(&mut self) -> std::io::Result<(AlignedVec, SocketAddr)> {
        let mut raw_buf = [0u8; MAX_PAYLOAD_SIZE + 64];

        // Non-blocking loop: drain available socket buffers
        loop {
            match self.socket.recv_from(&mut raw_buf) {
                Ok((amt, src_addr)) => {
                    if amt < 8 {
                        continue;
                    }

                    let chunk_idx = u32::from_be_bytes(raw_buf[0..4].try_into().unwrap());
                    let total_chunks = u32::from_be_bytes(raw_buf[4..8].try_into().unwrap());
                    let payload = &raw_buf[8..amt];

                    self.fragments.insert(chunk_idx, payload.to_vec());

                    if self.fragments.len() == total_chunks as usize {
                        let mut aligned = AlignedVec::new();
                        for (_, fragment) in std::mem::take(&mut self.fragments) {
                            aligned.extend_from_slice(&fragment);
                        }
                        return Ok((aligned, src_addr));
                    }
                }
                Err(e) => {
                    // WouldBlock is expected when no packets are pending in non-blocking mode
                    return Err(e);
                }
            }
        }
    }
}

pub struct ClientChannel {
    channel: UdpChannel,
    server_addr: SocketAddr,
}

impl ClientChannel {
    pub fn new(server_addr_str: &str) -> std::io::Result<Self> {
        let server_addr = server_addr_str
            .to_socket_addrs()?
            .find(|addr| addr.is_ipv4())
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Could not resolve IPv4 socket address",
                )
            })?;

        Ok(Self {
            channel: UdpChannel::bind("0.0.0.0:0")?,
            server_addr: server_addr,
        })
    }

    pub fn request_chunk(&self, x: i32, y: i32, z: i32) -> std::io::Result<()> {
        let req = ClientPacket::RequestChunk { x, y, z };
        let result = self.channel.send_packet(&req, self.server_addr);
        if let Err(ref e) = result {
            eprintln!("[Network] Failed to send request for chunk ({x}, {y}, {z}): {e}");
        }
        result
    }

    pub fn poll_network(&mut self) -> Option<ServerPacket> {
        match self.channel.recv_raw_payload() {
            Ok((aligned_payload, _)) => {
                match check_archived_root::<ServerPacket>(&aligned_payload) {
                    Ok(archived) => {
                        let packet = archived.deserialize(&mut rkyv::Infallible).unwrap();
                        Some(packet)
                    }
                    Err(e) => {
                        eprintln!("[Network] Failed to deserialize ServerPacket: {e:?}");
                        None
                    }
                }
            }
            Err(e) => {
                // Ignore WouldBlock (expected when no packet is waiting in non-blocking mode)
                if e.kind() != std::io::ErrorKind::WouldBlock {
                    eprintln!("[Network] Recv error: {e}");
                }
                None
            }
        }
    }
}
