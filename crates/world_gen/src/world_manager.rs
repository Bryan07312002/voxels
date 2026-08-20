use std::sync::{Arc, RwLock};

use core_types::{ChunkData, ChunkPos};

use crate::{ChunkCache, ChunkStore, TerrainGenerator};

pub struct WorldManager<G: TerrainGenerator, S: ChunkStore, C: ChunkCache> {
    generator: G,
    store: S,
    cache: Arc<RwLock<C>>,
}

impl<G: TerrainGenerator, S: ChunkStore, C: ChunkCache> WorldManager<G, S, C> {
    pub fn new(generator: G, store: S, cache: C) -> Self {
        Self {
            generator,
            store,
            cache: Arc::new(RwLock::new(cache)),
        }
    }

    /// Primary lookup pipeline: Cache -> Store -> Generator
    pub fn get_or_generate_chunk(&self, pos: ChunkPos) -> ChunkData {
        // 1. Check Memory Cache
        {
            let cache = self.cache.read().unwrap();
            if let Some(chunk) = cache.get(&pos) {
                return chunk;
            }
        }

        // 2. Check Disk Store
        if let Some(chunk) = self.store.load(&pos) {
            let mut cache = self.cache.write().unwrap();
            cache.insert(pos, chunk.clone());
            return chunk;
        }

        // 3. Fallback to Generator
        let chunk = self.generator.generate_chunk(pos);

        self.store.save(&pos, &chunk);
        let mut cache = self.cache.write().unwrap();
        cache.insert(pos, chunk.clone());

        chunk
    }
}
