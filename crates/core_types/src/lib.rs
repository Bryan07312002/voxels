use rkyv::{Archive, Deserialize, Serialize};
use std::fmt;

#[derive(
    rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, serde::Deserialize, Debug, Clone, Copy,
)]
#[archive(check_bytes)]
#[archive_attr(derive(Debug))]
#[serde(transparent)]
pub struct ViewDistance(pub u16);

impl ViewDistance {
    pub fn value(&self) -> u16 {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct CompressedChunk(pub Vec<u8>);

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

///  16x16x16 local chunk grid stored in flat 1D memory
#[derive(Debug, Clone)]
pub struct ChunkData {
    // these cant be pub since we need to check if was
    // changed after beeing compressed and if so need to compress again
    blocks: [BlockId; CHUNK_VOLUME],
    compressed_chunk: Option<CompressedChunk>,
    changed_since_compressed: bool,
}

impl ChunkData {
    pub fn new(blocks: [BlockId; CHUNK_VOLUME], compressed_chunk: Option<CompressedChunk>) -> Self {
        Self {
            blocks,
            compressed_chunk,
            changed_since_compressed: false,
        }
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> BlockId {
        self.blocks[x + (z * CHUNK_SIZE) + (y * CHUNK_SIZE * CHUNK_SIZE)]
    }

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, z: usize, block: BlockId) {
        self.blocks[x + (z * CHUNK_SIZE) + (y * CHUNK_SIZE * CHUNK_SIZE)] = block;
        if self.compressed_chunk.is_some() {
            self.changed_since_compressed = true;
        }
    }

    pub fn get_compressed_data(&mut self) -> CompressedChunk {
        if let Some(ref compressed) = self.compressed_chunk {
            if !self.changed_since_compressed {
                return compressed.clone();
            }
        }

        let compressed = compress_chunk_blocks(&self.blocks);

        self.compressed_chunk = Some(compressed.clone());
        self.changed_since_compressed = false;

        compressed
    }

    pub fn is_all_air(&self) -> bool {
        self.blocks.iter().all(|b| b.0 == 0)
    }
}

impl Default for ChunkData {
    fn default() -> Self {
        Self {
            blocks: [BlockId::AIR; CHUNK_VOLUME],
            compressed_chunk: None,
            changed_since_compressed: false,
        }
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

/// Compresses a slice of `BlockId` chunk blocks into a run-length encoded byte vector.
pub fn compress_chunk_blocks(blocks: &[BlockId; CHUNK_VOLUME]) -> CompressedChunk {
    let mut compressed = Vec::new();
    if blocks.is_empty() {
        return CompressedChunk(compressed);
    }

    let mut current_block = blocks[0];
    let mut count: u16 = 1;

    for block in blocks.iter().skip(1) {
        if *block == current_block && count < u16::MAX {
            count += 1;
        } else {
            // Extract the inner u16 value to write bytes
            compressed.extend_from_slice(&current_block.0.to_le_bytes());
            compressed.extend_from_slice(&count.to_le_bytes());
            current_block = *block;
            count = 1;
        }
    }
    compressed.extend_from_slice(&current_block.0.to_le_bytes());
    compressed.extend_from_slice(&count.to_le_bytes());

    CompressedChunk(compressed)
}

/// Decompresses RLE bytes back into a standard `[BlockId; CHUNK_VOLUME]` array.
pub fn decompress_chunk_blocks(data: &[u8]) -> Option<[BlockId; CHUNK_VOLUME]> {
    // We can initialize the array safely using an initial dummy BlockId
    let mut blocks = [BlockId(0); CHUNK_VOLUME];
    let mut index = 0;
    let mut cursor = 0;

    while cursor < data.len() && index < CHUNK_VOLUME {
        if cursor + 4 > data.len() {
            return None;
        }
        let raw_block = u16::from_le_bytes([data[cursor], data[cursor + 1]]);
        let count = u16::from_le_bytes([data[cursor + 2], data[cursor + 3]]);
        cursor += 4;

        let block = BlockId(raw_block);

        for _ in 0..count {
            if index >= CHUNK_VOLUME {
                return None;
            }
            blocks[index] = block;
            index += 1;
        }
    }

    if index == CHUNK_VOLUME {
        Some(blocks)
    } else {
        None
    }
}
