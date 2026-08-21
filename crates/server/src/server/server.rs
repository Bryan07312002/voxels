use std::sync::mpsc::Sender;
use std::sync::{Mutex, OnceLock};
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    thread,
    time::{Duration, Instant},
};

use config::ServerConfig;
use core_types::{CHUNK_SIZE, ChunkData, ChunkPos, ViewDistance};
use net::{ArchivedClientPacket, ClientPacket, ServerPacket, UdpChannel, check_archived_root};
use world_gen::{ChunkStore, TerrainGenerator, WorldManager};

use crate::{metric_clients::ServerMetrics, server::client_session::ClientSession};

/// Thread-safe cached table of relative spherical chunk offsets sorted by distance.
fn get_sphere_offsets(radius: i32) -> &'static [ChunkPos] {
    static OFFSET_CACHE: OnceLock<Mutex<HashMap<i32, &'static [ChunkPos]>>> = OnceLock::new();
    let cache = OFFSET_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = cache.lock().unwrap();

    *map.entry(radius).or_insert_with(|| {
        let mut offsets = Vec::new();
        let r_sq = radius * radius;
        for dx in -radius..=radius {
            for dy in -radius..=radius {
                for dz in -radius..=radius {
                    if dx * dx + dy * dy + dz * dz <= r_sq {
                        offsets.push(ChunkPos {
                            x: dx,
                            y: dy,
                            z: dz,
                        });
                    }
                }
            }
        }
        // Pre-sort offsets by distance squared from origin
        offsets.sort_by_key(|pos| pos.x * pos.x + pos.y * pos.y + pos.z * pos.z);

        // Leak the vector into a &'static slice. Safe because radius values are bounded (e.g. 1..32)
        Vec::leak(offsets)
    })
}

pub struct VoxelServer<G, S>
where
    G: TerrainGenerator,
    S: ChunkStore,
{
    channel: UdpChannel,

    loaded_chunks: HashMap<ChunkPos, ChunkData>,
    clients: HashMap<SocketAddr, ClientSession>,
    world_manager: WorldManager<G, S>,

    tick_duration: Duration,
    current_tick: u64,
    metrics_tx: Sender<ServerMetrics>,

    config: ServerConfig,
}

impl<G, S> VoxelServer<G, S>
where
    G: TerrainGenerator,
    S: ChunkStore,
{
    pub fn new(
        config: ServerConfig,
        world_manager: WorldManager<G, S>,
        metrics_tx: Sender<ServerMetrics>,
    ) -> std::io::Result<Self> {
        let channel = UdpChannel::bind(&format!("{}:{}", &config.host, config.port))?;
        channel.socket.set_nonblocking(true)?;

        let tps = 20;
        let tick_duration = Duration::from_secs_f64(1.0 / tps as f64);

        Ok(Self {
            clients: HashMap::new(),
            current_tick: 0,
            loaded_chunks: HashMap::new(),
            channel,
            world_manager,
            tick_duration,
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
            self.flush_chunk_queues(70);
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

            let _ = self.metrics_tx.send(snapshot);

            let now = Instant::now();
            if now < next_tick {
                thread::sleep(next_tick - now);
            } else {
                next_tick = now;
            }
        }
    }

    fn tick(&mut self) {
        let now = Instant::now();
        let ping_interval = Duration::from_secs(3);
        let timeout_duration = Duration::from_secs(10);

        let mut timed_out_clients = Vec::new();

        for (addr, session) in &mut self.clients {
            if now.duration_since(session.last_ping_sent) >= ping_interval {
                session.last_ping_sent = now;
                let _ = self.channel.send_packet(&ServerPacket::Ping, *addr);
            }

            if now.duration_since(session.last_pong_recived) > timeout_duration {
                timed_out_clients.push(*addr);
            }
        }

        for addr in timed_out_clients {
            println!("Client {addr} timed out. Disconnecting.");
            self.clients.remove(&addr);
        }
    }

    fn get_chunk_clone(&mut self, chunk_pos: ChunkPos) -> ChunkData {
        if let Some(chunk) = self.loaded_chunks.get(&chunk_pos) {
            return chunk.clone();
        }

        let mut chunk = self.world_manager.load_or_generate_chunk(&chunk_pos);
        let _ = chunk.get_compressed_data();

        self.loaded_chunks.insert(chunk_pos, chunk.clone());

        chunk
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

                let session = ClientSession::new(ViewDistance(view_distance.0));
                self.clients.insert(sender, session);
                return;
            }
            _ => {}
        }

        let Some(session) = self.clients.get_mut(&sender) else {
            return;
        };

        session.last_pong_recived = Instant::now();

        match packet {
            ArchivedClientPacket::RequestChunk { x, y, z } => {
                self.handle_request_chunk(x, y, z, sender);
            }
            ArchivedClientPacket::PlayerPosition { x, y, z } => {
                self.handle_update_player_position(sender, (*x, *y, *z));
            }
            ArchivedClientPacket::AckChunk { x, y, z } => {
                self.handle_ack_chunk(*x, *y, *z, sender);
            }
            ArchivedClientPacket::Pong {} => {
                session.last_pong_recived = Instant::now();
            }
            ArchivedClientPacket::Connect { .. } => {}
        }
    }

    fn handle_ack_chunk(&mut self, x: i32, y: i32, z: i32, sender: SocketAddr) {
        let pos = ChunkPos { x, y, z };

        if let Some(session) = self.clients.get_mut(&sender) {
            if session.pending_chunks.remove(&pos).is_some() {
                session.loaded_chunks.insert(pos);
            }
        }
    }

    fn handle_request_chunk(&mut self, x: &i32, y: &i32, z: &i32, sender: SocketAddr) {
        let pos = ChunkPos {
            x: *x,
            y: *y,
            z: *z,
        };

        let mut chunk = self.get_chunk_clone(pos);
        let compressed_blocks = chunk.get_compressed_data();

        let _ = self.channel.send_packet(
            &ServerPacket::ChunkDataCompressed {
                x: *x,
                y: *y,
                z: *z,
                compressed_blocks: compressed_blocks.0.clone(),
            },
            sender,
        );

        if let Some(session) = self.clients.get_mut(&sender) {
            session.pending_chunks.insert(pos, Instant::now());
        }
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

        let r = session.view_distance.0 as i32;
        let r_sq = r * r;
        let p_x = session.current_chunk.x;
        let p_y = session.current_chunk.y;
        let p_z = session.current_chunk.z;

        // 1. FAST UNLOAD DETECT: Check loaded chunks against scalar sphere equation (No HashSet needed!)
        let to_unload: Vec<ChunkPos> = session
            .loaded_chunks
            .iter()
            .filter_map(|pos| {
                let dx = pos.x - p_x;
                let dy = pos.y - p_y;
                let dz = pos.z - p_z;
                if dx * dx + dy * dy + dz * dz > r_sq {
                    Some(*pos)
                } else {
                    None
                }
            })
            .collect();

        for pos in to_unload {
            session.loaded_chunks.remove(&pos);
            session.pending_chunks.remove(&pos);
            session.send_queue.retain(|p| p != &pos);

            let packet = ServerPacket::UnloadChunk {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            };
            let _ = self.channel.send_packet(&packet, sender);
        }

        // 2. FAST PRUNE: Strip send_queue and pending_chunks outside view distance
        session.send_queue.retain(|pos| {
            let dx = pos.x - p_x;
            let dy = pos.y - p_y;
            let dz = pos.z - p_z;
            dx * dx + dy * dy + dz * dz <= r_sq
        });

        session.pending_chunks.retain(|pos, _| {
            let dx = pos.x - p_x;
            let dy = pos.y - p_y;
            let dz = pos.z - p_z;
            dx * dx + dy * dy + dz * dz <= r_sq
        });

        // 3. FAST LOAD ENQUEUE: Iterate pre-sorted offset table (No dynamic sorting or loop allocations!)
        let offsets = get_sphere_offsets(r);
        for rel in offsets {
            let target_pos = ChunkPos {
                x: p_x + rel.x,
                y: p_y + rel.y,
                z: p_z + rel.z,
            };

            if !session.loaded_chunks.contains(&target_pos)
                && !session.pending_chunks.contains_key(&target_pos)
                && !session.send_queue.contains(&target_pos)
            {
                session.send_queue.push_back(target_pos);
            }
        }
    }

    pub fn flush_chunk_queues(&mut self, max_chunks_to_send_per_tick: usize) {
        self.process_send_queues(max_chunks_to_send_per_tick);
    }

    fn process_send_queues(&mut self, max_chunks_to_send_per_tick: usize) {
        let mut tasks = Vec::new();
        for (sender, session) in &mut self.clients {
            let mut sent = 0;
            while sent < max_chunks_to_send_per_tick {
                let Some(pos) = session.send_queue.pop_front() else {
                    break;
                };
                tasks.push((*sender, pos));
                sent += 1;
            }
        }

        for (sender, pos) in tasks {
            let mut chunk = self.get_chunk_clone(pos);

            // OPTIMIZATION: Skip sending empty air chunks over UDP
            if chunk.is_all_air() {
                if let Some(session) = self.clients.get_mut(&sender) {
                    session.loaded_chunks.insert(pos);
                }
                continue;
            }

            let compressed_blocks = chunk.get_compressed_data();

            let response = ServerPacket::ChunkDataCompressed {
                x: pos.x,
                y: pos.y,
                z: pos.z,
                compressed_blocks: compressed_blocks.0.clone(),
            };

            if let Err(e) = self.channel.send_packet(&response, sender) {
                eprintln!("Failed to send chunk {} {} {}: {}", pos.x, pos.y, pos.z, e);
            } else if let Some(session) = self.clients.get_mut(&sender) {
                session.pending_chunks.insert(pos, Instant::now());
            }
        }
    }
}
