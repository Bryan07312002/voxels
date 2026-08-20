mod flat_world_generator;
mod world_manager;
mod cache;
mod store;

pub use flat_world_generator::FlatWorldGenerator;
pub use world_manager::WorldManager;
pub use cache::MemoryChunkCache;
pub use store::DiskChunkStore;

use core_types::{ChunkData, ChunkPos};

pub trait TerrainGenerator: Send + Sync {
    fn generate_chunk(&self, pos: ChunkPos) -> ChunkData;
}

pub trait ChunkCache: Send + Sync {
    fn get(&self, pos: &ChunkPos) -> Option<ChunkData>;
    fn insert(&mut self, pos: ChunkPos, data: ChunkData);
    fn remove(&mut self, pos: &ChunkPos) -> Option<ChunkData>;
}

pub trait ChunkStore: Send + Sync {
    fn load(&self, pos: &ChunkPos) -> Option<ChunkData>;
    fn save(&self, pos: &ChunkPos, data: &ChunkData);
}
