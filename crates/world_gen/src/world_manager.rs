use core_types::{ChunkData, ChunkPos};
use crate::{ChunkStore, TerrainGenerator};

pub struct WorldManager<G: TerrainGenerator, S: ChunkStore> {
    generator: G,
    store: S,
}

impl<G: TerrainGenerator, S: ChunkStore> WorldManager<G, S> {
    pub fn new(generator: G, store: S) -> Self {
        Self { generator, store }
    }

    pub fn load_or_generate_chunk(&self, pos: &ChunkPos) -> ChunkData {
        if let Some(chunk) = self.store.load(&pos) {
            return chunk;
        }

        let chunk = self.generator.generate_chunk(pos.clone());
        self.store.save(&pos, &chunk);

        chunk
    }
}
