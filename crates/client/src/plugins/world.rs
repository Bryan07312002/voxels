use crate::ClientConfigRes;
use crate::resources::VoxelWorldConfig;
use crate::{components::Player, plugins::network::NetworkClient};

use bevy::{
    pbr::wireframe::{Wireframe, WireframeColor},
    prelude::*,
};

use core_types::{BlockId, CHUNK_SIZE, CHUNK_VOLUME, ChunkPos, decompress_chunk_blocks};
use net::ServerPacket;
use std::collections::HashMap;
use voxel_mesh::generate_chunk_mesh;

#[derive(Resource)]
pub struct ChunkMaterialHandle(pub Handle<StandardMaterial>);

#[derive(Resource, Default)]
pub struct LoadedChunks(pub HashMap<ChunkPos, Entity>);

#[derive(Resource)]
pub struct ChunkCheckTimer(pub Timer);

impl Default for ChunkCheckTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(0.2, TimerMode::Repeating))
    }
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoadedChunks>()
            .init_resource::<ChunkCheckTimer>()
            .add_systems(Startup, setup_world_environment)
            .add_systems(
                Update,
                (
                    poll_network_system,
                    unload_distant_chunks,
                    //request_missing_chunks,
                ),
            );
    }
}

fn setup_world_environment(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let material_handle = materials.add(StandardMaterial {
        base_color: Color::rgb(0.2, 0.8, 0.2),
        cull_mode: None,
        ..default()
    });

    commands.insert_resource(ChunkMaterialHandle(material_handle));
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
        transform: Transform::from_xyz(15.0, 30.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

fn poll_network_system(
    mut commands: Commands,
    mut net_client: ResMut<NetworkClient>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Res<ChunkMaterialHandle>,
    config: Res<VoxelWorldConfig>,
    mut loaded_chunks: ResMut<LoadedChunks>,
) {
    while let Some(packet) = net_client.0.poll_network() {
        match packet {
            ServerPacket::ChunkDataCompressed {
                x,
                y,
                z,
                compressed_blocks,
            } => {
                handle_chunk_compressed(
                    x,
                    y,
                    z,
                    compressed_blocks,
                    &material,
                    &config,
                    &mut net_client,
                    &mut loaded_chunks,
                    &mut commands,
                    &mut meshes,
                );
            }
            ServerPacket::UnloadChunk { x, y, z } => {
                let pos = ChunkPos { x, y, z };
                if let Some(entity) = loaded_chunks.0.remove(&pos) {
                    commands.entity(entity).despawn_recursive();
                }
            }
            ServerPacket::Pong => {}
            ServerPacket::ChunkData { .. } => {
                warn!("Uncompressed chunk data packets are deprecated.");
            }
        }
    }
}

fn handle_chunk_compressed(
    x: i32,
    y: i32,
    z: i32,
    compressed_blocks: Vec<u8>,
    material: &ChunkMaterialHandle,
    config: &VoxelWorldConfig,
    net_client: &mut NetworkClient,
    loaded_chunks: &mut LoadedChunks,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
) {
    let pos = ChunkPos { x, y, z };

    let Some(blocks) = decompress_chunk_blocks(&compressed_blocks) else {
        error!("Failed to decompress chunk at ({}, {}, {})", x, y, z);
        return;
    };

    let _ = net_client.0.send_ack_chunk(x, y, z);

    if let Some(old_entity) = loaded_chunks.0.remove(&pos) {
        commands.entity(old_entity).despawn_recursive();
    }

    if let Some(entity) = spawn_chunk(commands, meshes, material, config, x, y, z, &blocks) {
        loaded_chunks.0.insert(pos, entity);
    }
}

fn get_player_chunk_pos(player_query: &Query<&Transform, With<Player>>) -> Option<ChunkPos> {
    let transform = player_query.get_single().ok()?;
    let p_pos = transform.translation;
    let cs = CHUNK_SIZE as f32;

    Some(ChunkPos {
        x: (p_pos.x / cs).floor() as i32,
        y: (p_pos.y / cs).floor() as i32,
        z: (p_pos.z / cs).floor() as i32,
    })
}

fn unload_distant_chunks(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    mut loaded_chunks: ResMut<LoadedChunks>,
    config: Res<ClientConfigRes>,
) {
    let Some(player_chunk) = get_player_chunk_pos(&player_query) else {
        return;
    };

    let max_chunk_radius = config.0.view_distance.value() as i32;
    let max_radius_sq = max_chunk_radius * max_chunk_radius;

    loaded_chunks.0.retain(|pos, &mut entity| {
        let dx = pos.x - player_chunk.x;
        let dy = pos.y - player_chunk.y;
        let dz = pos.z - player_chunk.z;

        if (dx * dx + dy * dy + dz * dz) > max_radius_sq {
            commands.entity(entity).despawn_recursive();
            false
        } else {
            true
        }
    });
}

fn request_missing_chunks(
    time: Res<Time>,
    mut timer: ResMut<ChunkCheckTimer>,
    player_query: Query<&Transform, With<Player>>,
    loaded_chunks: Res<LoadedChunks>,
    net_client: Res<NetworkClient>,
    config: Res<ClientConfigRes>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    let Some(player_chunk) = get_player_chunk_pos(&player_query) else {
        return;
    };

    let max_chunk_radius = config.0.view_distance.value() as i32;
    let max_radius_sq = max_chunk_radius * max_chunk_radius;
    let mut requests_sent = 0;
    let max_requests_per_batch = 8;

    'outer: for dx in -max_chunk_radius..=max_chunk_radius {
        for dy in -max_chunk_radius..=max_chunk_radius {
            for dz in -max_chunk_radius..=max_chunk_radius {
                if dx * dx + dy * dy + dz * dz <= max_radius_sq {
                    let pos = ChunkPos {
                        x: player_chunk.x + dx,
                        y: player_chunk.y + dy,
                        z: player_chunk.z + dz,
                    };

                    if !loaded_chunks.0.contains_key(&pos) {
                        let _ = net_client.0.request_chunk(pos.x, pos.y, pos.z);
                        requests_sent += 1;

                        if requests_sent >= max_requests_per_batch {
                            break 'outer;
                        }
                    }
                }
            }
        }
    }
}

fn spawn_chunk(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: &ChunkMaterialHandle,
    config: &VoxelWorldConfig,
    chunk_x: i32,
    chunk_y: i32,
    chunk_z: i32,
    blocks: &[BlockId; CHUNK_VOLUME],
) -> Option<Entity> {
    if blocks.len() != CHUNK_VOLUME {
        error!("Received malformed chunk data packet layout!");
        return None;
    }

    let mesh = generate_chunk_mesh(blocks, config.block_size)?;

    let chunk_world_pos = Vec3::new(
        chunk_x as f32 * (CHUNK_SIZE as f32) * config.block_size,
        chunk_y as f32 * (CHUNK_SIZE as f32) * config.block_size,
        chunk_z as f32 * (CHUNK_SIZE as f32) * config.block_size,
    );

    Some(
        commands
            .spawn((
                PbrBundle {
                    mesh: meshes.add(mesh),
                    material: material.0.clone(),
                    transform: Transform::from_translation(chunk_world_pos),
                    ..default()
                },
                Wireframe,
                WireframeColor {
                    color: Color::BLACK,
                },
            ))
            .id(),
    )
}
