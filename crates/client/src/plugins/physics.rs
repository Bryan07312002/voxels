use crate::components::{Aabb, Grounded, Velocity, VoxelBlock};
use crate::resources::PhysicsConfig;
use bevy::prelude::*;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PhysicsConfig>()
            .add_systems(Update, (apply_gravity, resolve_collisions).chain());
    }
}

fn apply_gravity(
    time: Res<Time>,
    config: Res<PhysicsConfig>,
    mut query: Query<(&mut Velocity, &Grounded)>,
) {
    let dt = time.delta_seconds();
    for (mut velocity, grounded) in query.iter_mut() {
        if !grounded.0 {
            velocity.0.y += config.gravity * dt;
        }
    }
}

fn resolve_collisions(
    time: Res<Time>,
    mut dynamic_query: Query<(Entity, &mut Transform, &mut Velocity, &mut Grounded, &Aabb)>,
    static_query: Query<(Entity, &Transform, &Aabb), (With<VoxelBlock>, Without<Velocity>)>,
) {
    let dt = time.delta_seconds();

    for (dyn_entity, mut transform, mut velocity, mut grounded, dyn_aabb) in dynamic_query.iter_mut() {
        grounded.0 = false;

        // --- 1. AXIS X RESOLUTION ---
        transform.translation.x += velocity.0.x * dt;
        for (stat_entity, stat_transform, stat_aabb) in static_query.iter() {
            if dyn_entity == stat_entity {
                continue;
            }
            if check_aabb_intersection(
                transform.translation,
                dyn_aabb.half_extents,
                stat_transform.translation,
                stat_aabb.half_extents,
            ) {
                let overlap_x = (dyn_aabb.half_extents.x + stat_aabb.half_extents.x)
                    - (transform.translation.x - stat_transform.translation.x).abs();

                if transform.translation.x > stat_transform.translation.x {
                    transform.translation.x += overlap_x;
                } else {
                    transform.translation.x -= overlap_x;
                }
                velocity.0.x = 0.0;
            }
        }

        // --- 2. AXIS Y RESOLUTION ---
        transform.translation.y += velocity.0.y * dt;
        for (stat_entity, stat_transform, stat_aabb) in static_query.iter() {
            if dyn_entity == stat_entity {
                continue;
            }
            if check_aabb_intersection(
                transform.translation,
                dyn_aabb.half_extents,
                stat_transform.translation,
                stat_aabb.half_extents,
            ) {
                let overlap_y = (dyn_aabb.half_extents.y + stat_aabb.half_extents.y)
                    - (transform.translation.y - stat_transform.translation.y).abs();

                if transform.translation.y > stat_transform.translation.y {
                    transform.translation.y += overlap_y;
                    velocity.0.y = 0.0;
                    grounded.0 = true; // Standing on top of a voxel
                } else {
                    transform.translation.y -= overlap_y;
                    velocity.0.y = 0.0; // Hit ceiling
                }
            }
        }

        // --- 3. AXIS Z RESOLUTION ---
        transform.translation.z += velocity.0.z * dt;
        for (stat_entity, stat_transform, stat_aabb) in static_query.iter() {
            if dyn_entity == stat_entity {
                continue;
            }
            if check_aabb_intersection(
                transform.translation,
                dyn_aabb.half_extents,
                stat_transform.translation,
                stat_aabb.half_extents,
            ) {
                let overlap_z = (dyn_aabb.half_extents.z + stat_aabb.half_extents.z)
                    - (transform.translation.z - stat_transform.translation.z).abs();

                if transform.translation.z > stat_transform.translation.z {
                    transform.translation.z += overlap_z;
                } else {
                    transform.translation.z -= overlap_z;
                }
                velocity.0.z = 0.0;
            }
        }
    }
}

/// Helper function evaluating 3D AABB overlapping
fn check_aabb_intersection(pos_a: Vec3, extents_a: Vec3, pos_b: Vec3, extents_b: Vec3) -> bool {
    (pos_a.x - pos_b.x).abs() < (extents_a.x + extents_b.x)
        && (pos_a.y - pos_b.y).abs() < (extents_a.y + extents_b.y)
        && (pos_a.z - pos_b.z).abs() < (extents_a.z + extents_b.z)
}
