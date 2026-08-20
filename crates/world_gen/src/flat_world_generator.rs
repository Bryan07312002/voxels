use core_types::{BlockId, CHUNK_SIZE, ChunkData, ChunkPos};

use crate::TerrainGenerator;

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
        let mut chunk = ChunkData::default();
        let base_y = pos.y * CHUNK_SIZE as i32;

        let center = CHUNK_SIZE / 2;
        let dirt_layer_top = self.height_level - 1; // Top dirt block directly under grass

        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let is_center_or_corner = (x == center && z == center) || (x == 0 && z == 0);

                for y in 0..CHUNK_SIZE {
                    let world_y = base_y + y as i32;

                    let block = if world_y > self.height_level {
                        BlockId::AIR
                    } else if world_y == self.height_level {
                        BlockId::GRASS
                    } else if is_center_or_corner && world_y == dirt_layer_top {
                        // Replace dirt with AIR at (0, 0) corner and center (CHUNK_SIZE / 2)
                        BlockId::AIR
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
