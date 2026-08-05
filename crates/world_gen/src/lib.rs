use core_types::{BlockId, ChunkData, ChunkPos, CHUNK_SIZE};

pub trait TerrainGenerator: Send + Sync {
    fn generate_chunk(&self, pos: ChunkPos) -> ChunkData;
}

pub struct FlatWorldGenerator {
    /// The global Y coordinate where grass generates
    pub height_level: i32,
}

impl FlatWorldGenerator {
    pub fn new(height_level: i32) -> Self {
        Self { height_level }
    }
}

impl TerrainGenerator for FlatWorldGenerator {
    /// Generates a chunk at the given chunk grid coordinates (chunk_y determines vertical tier)
    fn generate_chunk(&self, pos: ChunkPos) -> ChunkData {
        let mut chunk = ChunkData::new();

        let base_y = pos.y * CHUNK_SIZE as i32;

        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    let world_y = base_y + y as i32;

                    let block = if world_y > self.height_level {
                        BlockId::AIR
                    } else if world_y == self.height_level {
                        BlockId::GRASS
                    } else if world_y >= self.height_level - 3 {
                        BlockId::DIRT
                    } else {
                        BlockId::STONE
                    };

                    chunk.set(x, y, z, block);
                }
            }
        }

        chunk
    }
}
