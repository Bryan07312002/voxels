use std::collections::HashMap;

use core_types::{ChunkData, ChunkPos};

use crate::ChunkCache;

pub struct MemoryChunkCache {
    storage: HashMap<ChunkPos, ChunkData>,
    capacity: usize,
}

impl MemoryChunkCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            storage: HashMap::with_capacity(capacity),
            capacity,
        }
    }
}

impl ChunkCache for MemoryChunkCache {
    fn get(&self, pos: &ChunkPos) -> Option<ChunkData> {
        self.storage.get(pos).cloned()
    }

    fn insert(&mut self, pos: ChunkPos, data: ChunkData) {
        if self.storage.len() >= self.capacity {
            if let Some(first_key) = self.storage.keys().next().cloned() {
                self.storage.remove(&first_key);
            }
        }
        self.storage.insert(pos, data);
    }

    fn remove(&mut self, pos: &ChunkPos) -> Option<ChunkData> {
        self.storage.remove(pos)
    }
}
