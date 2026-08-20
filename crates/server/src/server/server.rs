use std::sync::mpsc::Sender;
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    thread,
    time::{Duration, Instant},
};

use config::ServerConfig;
use core_types::{CHUNK_SIZE, ChunkPos, ViewDistance};
use net::{ArchivedClientPacket, ClientPacket, ServerPacket, UdpChannel, check_archived_root};
use world_gen::{FlatWorldGenerator, TerrainGenerator};

use crate::{
    metric_clients::ServerMetrics,
    server::client_session::{ClientSession, PendingChunk},
};

pub struct VoxelServer {
    pub channel: UdpChannel,
    generator: FlatWorldGenerator,
    clients: HashMap<SocketAddr, ClientSession>,

    tick_duration: Duration,
    current_tick: u64,
    metrics_tx: Sender<ServerMetrics>,

    config: ServerConfig,
}

impl VoxelServer {
    pub fn new(config: ServerConfig, metrics_tx: Sender<ServerMetrics>) -> std::io::Result<Self> {
        let channel = UdpChannel::bind(&format!("{}:{}", &config.host, config.port))?;
        channel.socket.set_nonblocking(true)?;

        let tps = 20;
        let tick_duration = Duration::from_secs_f64(1.0 / tps as f64);

        Ok(Self {
            channel,
            generator: FlatWorldGenerator::new(6),
            clients: HashMap::new(),
            tick_duration,
            current_tick: 0,
            metrics_tx,
            config,
        })
    }

    pub fn run(&mut self) {
        let mut next_tick = Instant::now();

        loop {
            let tick_start = Instant::now();
            next_tick += self.tick_duration;

            self.process_incoming_packets();
            self.tick();
            self.flush_chunk_queues(16);
            self.current_tick += 1;

            // Gather metrics snapshot
            let total_pending: usize = self.clients.values().map(|c| c.pending_chunks.len()).sum();
            let total_queue: usize = self.clients.values().map(|c| c.send_queue.len()).sum();

            let snapshot = ServerMetrics {
                current_tick: self.current_tick,
                tick_duration: tick_start.elapsed(),
                connected_clients: self.clients.len(),
                pending_chunks: total_pending,
                queue_size: total_queue,
            };

            // Non-blocking send to TUI (ignores error if TUI thread exited)
            let _ = self.metrics_tx.send(snapshot);

            // Sleep logic
            let now = Instant::now();
            if now < next_tick {
                thread::sleep(next_tick - now);
            } else {
                next_tick = now;
            }
        }
    }

    fn tick(&mut self) {
        // Perform game simulation logic here (e.g., 20 times per second)
        // - Update entity positions & physics
        // - Process block changes / tile entities
        // - Handle client timeouts (e.g., drop clients inactive for >10s)
    }

    fn process_incoming_packets(&mut self) {
        loop {
            match self.channel.recv_raw_payload() {
                Ok((aligned_bytes, sender_addr)) => {
                    if let Ok(archived) = check_archived_root::<ClientPacket>(&aligned_bytes) {
                        self.handle_packet(archived, sender_addr);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(err) => {
                    eprintln!("Network receive error: {err}");
                    break;
                }
            }
        }
    }

    fn handle_packet(&mut self, packet: &ArchivedClientPacket, sender: SocketAddr) {
        match packet {
            ArchivedClientPacket::Connect { view_distance } => {
                println!(
                    "Player connected from {sender} with view distance {:?}",
                    view_distance
                );

                // Explicitly create and insert the session using their settings
                let session = ClientSession::new(ViewDistance(view_distance.0));
                self.clients.insert(sender, session);

                // TODO: send a ServerPacket::AcceptConnection
            }
            other => {
                if !self.clients.contains_key(&sender) {
                    // Ignore packets from unauthenticated clients
                    return;
                }

                match other {
                    ArchivedClientPacket::RequestChunk { x, y, z } => {
                        self.handle_request_chunk(x, y, z, sender);
                    }
                    ArchivedClientPacket::PlayerPosition { x, y, z } => {
                        self.handle_update_player_position(sender, (*x, *y, *z));
                    }
                    ArchivedClientPacket::AckChunk { x, y, z } => {
                        self.handle_ack_chunk(x, y, z, sender);
                    }
                    ArchivedClientPacket::Connect { view_distance: _ } => {
                        /*should already be connected*/
                    }
                }
            }
        }
    }

    fn handle_ack_chunk(&mut self, x: &i32, y: &i32, z: &i32, sender: SocketAddr) {
        let pos = ChunkPos {
            x: x.clone(),
            y: y.clone(),
            z: z.clone(),
        };

        if let Some(session) = self.clients.get_mut(&sender) {
            session.pending_chunks.remove(&pos);
            session.loaded_chunks.insert(pos);
        }
    }

    fn handle_request_chunk(&mut self, x: &i32, y: &i32, z: &i32, sender: SocketAddr) {
        // If the client requested it, generate or retrieve blocks from world generator
        let pos = ChunkPos {
            x: x.clone(),
            y: y.clone(),
            z: z.clone(),
        };

        let mut chunk = self.generator.generate_chunk(pos);

        let compressed_blocks = chunk.get_compressed_data();

        // Send compressed packet
        let _ = self.channel.send_packet(
            &ServerPacket::ChunkDataCompressed {
                x: x.clone(),
                y: y.clone(),
                z: z.clone(),
                compressed_blocks: compressed_blocks.0.clone(),
            },
            sender,
        );

        // Track as pending until AckChunk is received
        self.clients
            .get_mut(&sender)
            .unwrap()
            .pending_chunks
            .insert(
                pos,
                PendingChunk {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                    compressed_data: compressed_blocks.0,
                    last_sent: Instant::now(),
                },
            );
    }

    fn handle_update_player_position(&mut self, sender: SocketAddr, (px, py, pz): (f32, f32, f32)) {
        let cs = CHUNK_SIZE as f32;
        let new_chunk_pos = ChunkPos {
            x: (px / cs).floor() as i32,
            y: (py / cs).floor() as i32,
            z: (pz / cs).floor() as i32,
        };

        let view_dist = self.config.max_view_distance;
        let session = self
            .clients
            .entry(sender)
            .or_insert_with(|| ClientSession::new(view_dist));

        if session.current_chunk == new_chunk_pos {
            return;
        }

        session.current_chunk = new_chunk_pos;

        let mut required_chunks = HashSet::new();
        let r = session.view_distance.0 as i32;
        let p_x = session.current_chunk.x;
        let p_y = session.current_chunk.y;
        let p_z = session.current_chunk.z;

        // True 3D dynamic sphere around the player's current chunk in all directions
        for dx in -r..=r {
            for dy in -r..=r {
                for dz in -r..=r {
                    if dx * dx + dy * dy + dz * dz <= r * r {
                        required_chunks.insert(ChunkPos {
                            x: p_x + dx,
                            y: p_y + dy, // Moves seamlessly up and down with you!
                            z: p_z + dz,
                        });
                    }
                }
            }
        }

        // 1. PRUNE STALE QUEUE: Instantly drop any queued chunks you already flew past
        session
            .send_queue
            .retain(|pos| required_chunks.contains(pos));

        // 2. PRUNE STALE PENDING: Stop resending unacknowledged chunks you flew past!
        session
            .pending_chunks
            .retain(|pos, _| required_chunks.contains(pos));

        // Identify missing chunks not loaded, not queued, AND not currently pending
        let mut to_load: Vec<ChunkPos> = required_chunks
            .iter()
            .filter(|pos| {
                !session.loaded_chunks.contains(pos)
                    && !session.send_queue.contains(pos)
                    && !session.pending_chunks.contains_key(pos) // Ensure we don't re-queue it!
            })
            .copied()
            .collect(); // Identify chunks outside view distance

        let to_unload: Vec<ChunkPos> = session
            .loaded_chunks
            .difference(&required_chunks)
            .copied()
            .collect();

        // Process unloads immediately
        for pos in to_unload {
            session.loaded_chunks.remove(&pos);
            let packet = ServerPacket::UnloadChunk {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            };
            let _ = self.channel.send_packet(&packet, sender);
        }

        to_load.sort_by_key(|pos| {
            let dx = pos.x - new_chunk_pos.x;
            let dy = pos.y - new_chunk_pos.y;
            let dz = pos.z - new_chunk_pos.z;
            dx * dx + dy * dy + dz * dz
        });

        session.send_queue.extend(to_load);
    }

    fn flush_chunk_queues(&mut self, max_chunks_to_send_per_tick: usize) {
        let now = std::time::Instant::now();

        for (sender, session) in self.clients.iter_mut() {
            let mut sent = 0;

            // 1. Send new chunks from the queue
            while sent < max_chunks_to_send_per_tick {
                let Some(pos) = session.send_queue.pop_front() else {
                    break;
                };

                let mut chunk = self.generator.generate_chunk(pos);

                // OPTIMIZATION: Skip sending empty air chunks over UDP
                if chunk.is_all_air() {
                    session.loaded_chunks.insert(pos); // Safe to mark loaded
                    sent += 1;
                    continue;
                }

                let compressed_blocks = chunk.get_compressed_data();

                let response = ServerPacket::ChunkDataCompressed {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                    compressed_blocks: compressed_blocks.0.clone(),
                };

                if let Err(e) = self.channel.send_packet(&response, *sender) {
                    eprintln!("Failed to send chunk {} {} {}: {}", pos.x, pos.y, pos.z, e);
                } else {
                    // RELIABILITY: Track this chunk as pending until the client ACKs it
                    session.pending_chunks.insert(
                        pos,
                        PendingChunk {
                            x: pos.x,
                            y: pos.y,
                            z: pos.z,
                            compressed_data: compressed_blocks.0,
                            last_sent: now,
                        },
                    );
                }

                sent += 1;
            }

            // 2. Resend unacknowledged chunks (Reliable UDP)
            let resend_timeout = std::time::Duration::from_millis(200);
            for pending in session.pending_chunks.values_mut() {
                if now.duration_since(pending.last_sent) > resend_timeout {
                    let packet = ServerPacket::ChunkDataCompressed {
                        x: pending.x,
                        y: pending.y,
                        z: pending.z,
                        compressed_blocks: pending.compressed_data.clone(),
                    };

                    let _ = self.channel.send_packet(&packet, *sender);
                    pending.last_sent = now; // Reset the timeout
                }
            }
        }
    }
}
