use bevy::prelude::*;

#[derive(Resource)]
pub struct VoxelWorldConfig {
    pub map_radius: i32,
    pub block_size: f32,
}

impl Default for VoxelWorldConfig {
    fn default() -> Self {
        Self {
            map_radius: 12, // Generates a 25x25 grid
            block_size: 1.0,
        }
    }
}

#[derive(Resource)]
pub struct PhysicsConfig {
    pub gravity: f32,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self { gravity: -25.0 }
    }
}
