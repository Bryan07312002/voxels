use std::{fs, path::PathBuf};

use core_types::{ChunkData, ChunkPos};

use crate::ChunkStore;

pub struct DiskChunkStore {
    base_path: PathBuf,
}

impl DiskChunkStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let base_path = path.into();
        let _ = fs::create_dir_all(&base_path);
        Self { base_path }
    }

    fn get_path(&self, pos: &ChunkPos) -> PathBuf {
        self.base_path.join(format!("chunk_{}_{}_{}.bin", pos.x, pos.y, pos.z))
    }
}

impl ChunkStore for DiskChunkStore {
    fn load(&self, pos: &ChunkPos) -> Option<ChunkData> {
        let path = self.get_path(pos);
        if path.exists() {
            let bytes = fs::read(path).ok()?;
            if bytes.len() == std::mem::size_of::<ChunkData>() {
                let data: ChunkData = unsafe { std::ptr::read(bytes.as_ptr() as *const _) };
                return Some(data);
            }
        }
        None
    }

    fn save(&self, pos: &ChunkPos, data: &ChunkData) {
        let path = self.get_path(pos);
        let bytes = unsafe {
            std::slice::from_raw_parts(data as *const ChunkData as *const u8, std::mem::size_of::<ChunkData>())
        };
        let _ = fs::write(path, bytes);
    }
}
