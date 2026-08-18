use rkyv::{Archive, Deserialize, Serialize};
use std::fmt;

pub const CHUNK_SIZE: usize = 16;
pub const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub u16);

impl BlockId {
    pub const AIR: BlockId = BlockId(0);
    pub const GRASS: BlockId = BlockId(1);
    pub const DIRT: BlockId = BlockId(2);
    pub const STONE: BlockId = BlockId(3);
}

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BlockId: {}", self.0)
    }
}

/// A 16x16x16 local chunk grid stored in flat 1D memory
#[derive(Debug, Clone)]
pub struct ChunkData {
    pub blocks: [BlockId; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
}

impl ChunkData {
    pub fn new() -> Self {
        Self {
            blocks: [BlockId::AIR; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
        }
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> BlockId {
        self.blocks[x + (z * CHUNK_SIZE) + (y * CHUNK_SIZE * CHUNK_SIZE)]
    }

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, z: usize, block: BlockId) {
        self.blocks[x + (z * CHUNK_SIZE) + (y * CHUNK_SIZE * CHUNK_SIZE)] = block;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Archive, Serialize, Deserialize)]
#[archive(check_bytes)]
pub struct ChunkPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkPos {
    pub const ZERO: Self = Self { x: 0, y: 0, z: 0 };

    #[inline]
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// Returns the origin block coordinate of this chunk in world space
    #[inline]
    pub fn world_origin(&self) -> (i32, i32, i32) {
        let cs = CHUNK_SIZE as i32;
        (self.x * cs, self.y * cs, self.z * cs)
    }
}
