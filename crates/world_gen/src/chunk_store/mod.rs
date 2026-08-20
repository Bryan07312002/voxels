use core_types::{ChunkData, ChunkPos};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub struct ChunkStore {
    base_path: PathBuf,
}

impl ChunkStore {
    pub fn new<P: AsRef<Path>>(world_directory: P) -> std::io::Result<Self> {
        let base_path = world_directory.as_ref().join("chunks");
        fs::create_dir_all(&base_path)?;
        Ok(Self { base_path })
    }

    fn chunk_file_path(&self, pos: &ChunkPos) -> PathBuf {
        self.base_path
            .join(format!("{}_{}_{}.bin", pos.x, pos.y, pos.z))
    }

    /// Check if a chunk exists on disk
    pub fn exists(&self, pos: &ChunkPos) -> bool {
        self.chunk_file_path(pos).exists()
    }

    /// Save uncompressed ChunkData directly to disk
    pub fn save_chunk(&self, pos: &ChunkPos, chunk: &ChunkData) -> std::io::Result<()> {
        let path = self.chunk_file_path(pos);
        let encoded = bincode::serialize(chunk)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let mut file = File::create(path)?;
        file.write_all(&encoded)?;
        Ok(())
    }

    /// Load ChunkData from disk
    pub fn load_chunk(&self, pos: &ChunkPos) -> std::io::Result<Option<ChunkData>> {
        let path = self.chunk_file_path(pos);
        if !path.exists() {
            return Ok(None);
        }

        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        let chunk: ChunkData = bincode::deserialize(&buffer)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        Ok(Some(chunk))
    }

    /// Save pre-compressed chunk bytes directly to disk (saves CPU during reload!)
    pub fn save_compressed(&self, pos: &ChunkPos, compressed_bytes: &[u8]) -> std::io::Result<()> {
        let path = self.chunk_file_path(pos);
        let mut file = File::create(path)?;
        file.write_all(compressed_bytes)?;
        Ok(())
    }

    /// Load pre-compressed bytes directly from disk
    pub fn load_compressed(&self, pos: &ChunkPos) -> std::io::Result<Option<Vec<u8>>> {
        let path = self.chunk_file_path(pos);
        if !path.exists() {
            return Ok(None);
        }

        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        Ok(Some(buffer))
    }
}
