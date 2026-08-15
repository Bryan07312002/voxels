use bevy::prelude::*;

/// Marker for the local player entity
#[derive(Component)]
pub struct Player;

/// Linear velocity vector (m/s)
#[derive(Component, Default)]
pub struct Velocity(pub Vec3);

/// Tracks if the entity is currently resting on a voxel surface
#[derive(Component, Default)]
pub struct Grounded(pub bool);

/// Axis-Aligned Bounding Box (half-extents from entity center)
#[derive(Component)]
pub struct Aabb {
    pub half_extents: Vec3,
}

/// Marker for collidable voxel blocks
#[derive(Component)]
pub struct VoxelBlock;

/// First-Person Camera controller state
#[derive(Component)]
pub struct FpsCamera {
    pub pitch: f32,
    pub yaw: f32,
    pub sensitivity: f32,
}

impl Default for FpsCamera {
    fn default() -> Self {
        Self {
            pitch: 0.0,
            yaw: 0.0,
            sensitivity: 0.002,
        }
    }
}
