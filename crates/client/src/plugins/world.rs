use crate::components::{Aabb, VoxelBlock};
use crate::resources::VoxelWorldConfig;
use bevy::prelude::*;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_world);
    }
}

fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<VoxelWorldConfig>,
) {
    // 1. Sun & Ambient Lighting
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 400.0,
    });

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            illuminance: 10000.0,
            ..default()
        },
        transform: Transform::from_xyz(15.0, 30.0, 15.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // 2. Shared Assets for Voxel Cubes
    let cube_mesh = meshes.add(Cuboid::new(
        config.block_size,
        config.block_size,
        config.block_size,
    ));

    let grass_material = materials.add(StandardMaterial {
        base_color: Color::rgb(0.2, 0.7, 0.3),
        perceptual_roughness: 0.9,
        ..default()
    });

    let half_extent = Vec3::splat(config.block_size * 0.5);

    // 3. Spawn Flat Voxel Floor (y = 0)
    for x in -config.map_radius..=config.map_radius {
        for z in -config.map_radius..=config.map_radius {
            commands.spawn((
                PbrBundle {
                    mesh: cube_mesh.clone(),
                    material: grass_material.clone(),
                    transform: Transform::from_xyz(
                        x as f32 * config.block_size,
                        0.0,
                        z as f32 * config.block_size,
                    ),
                    ..default()
                },
                VoxelBlock,
                Aabb {
                    half_extents: half_extent,
                },
            ));
        }
    }
}
