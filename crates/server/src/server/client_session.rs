use std::{
    collections::{HashMap, HashSet, VecDeque},
    time::Instant,
};

use core_types::{ChunkPos, ViewDistance};

pub struct PendingChunk {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub compressed_data: Vec<u8>,
    pub last_sent: Instant,
}

pub struct ClientSession {
    pub current_chunk: ChunkPos,
    pub view_distance: ViewDistance,
    pub loaded_chunks: HashSet<ChunkPos>,
    pub send_queue: VecDeque<ChunkPos>,
    pub pending_chunks: HashMap<ChunkPos, PendingChunk>, // Track un-acked chunks
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
        }
    }
}
