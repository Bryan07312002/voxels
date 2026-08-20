use std::{
    collections::{HashMap, HashSet, VecDeque},
    time::Instant,
};

use core_types::{ChunkPos, ViewDistance};

pub struct ClientSession {
    pub view_distance: ViewDistance,
    pub current_chunk: ChunkPos,
    pub send_queue: VecDeque<ChunkPos>,
    pub loaded_chunks: HashSet<ChunkPos>,
    pub pending_chunks: HashMap<ChunkPos, Instant>, // Track un-acked chunks
    pub last_pong_recived: Instant,
    pub last_ping_sent: Instant, // Tracks when the server last sent a Ping
}

impl ClientSession {
    pub fn new(view_distance: ViewDistance) -> Self {
        Self {
            current_chunk: ChunkPos {
                x: i32::MAX,
                y: i32::MAX,
                z: i32::MAX,
            },
            view_distance,
            loaded_chunks: HashSet::new(),
            send_queue: VecDeque::new(),
            pending_chunks: HashMap::new(),
            last_pong_recived: Instant::now(),
            last_ping_sent: Instant::now(),
        }
    }
}
