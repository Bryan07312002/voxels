use crate::components::Player;
use crate::resources::VoxelWorldConfig;
use bevy::prelude::*;
use core_types::{BlockId, CHUNK_SIZE, CHUNK_VOLUME, ChunkPos, decompress_chunk_blocks};
use net::{ClientChannel, ServerPacket};
use std::collections::HashMap;
use voxel_mesh::generate_chunk_mesh;

#[derive(Resource)]
pub struct NetworkClient(pub ClientChannel);

#[derive(Resource)]
pub struct ChunkMaterialHandle(pub Handle<StandardMaterial>);

#[derive(Resource, Default)]
pub struct LoadedChunks(pub HashMap<ChunkPos, Entity>);

// Timer resource to prevent checking/spamming requests every single frame
#[derive(Resource)]
pub struct ChunkCheckTimer(pub Timer);

impl Default for ChunkCheckTimer {
    fn default() -> Self {
        // Check and request missing chunks 5 times per second (every 200ms)
        Self(Timer::from_seconds(0.2, TimerMode::Repeating))
    }
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(client) = ClientChannel::new("127.0.0.1:25565") {
            info!("Network client bound to server at 127.0.0.1:25565");
            app.insert_resource(NetworkClient(client));
        }

        app.init_resource::<LoadedChunks>()
            .init_resource::<ChunkCheckTimer>()
            .add_systems(Startup, setup_world_environment)
            .add_systems(
                Update,
                (poll_network_system, client_chunk_request_and_unload),
            );
    }
}

fn setup_world_environment(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    _net_client: Option<Res<NetworkClient>>,
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
    player_query: Query<&Transform, With<Player>>,
) {
    let Ok(player_transform) = player_query.get_single() else {
        return;
    };
    let p_pos = player_transform.translation;
    let cs = CHUNK_SIZE as f32;
    let player_chunk_x = (p_pos.x / cs).floor() as i32;
    let player_chunk_y = (p_pos.y / cs).floor() as i32;
    let player_chunk_z = (p_pos.z / cs).floor() as i32;
    let max_radius = 15;

    while let Some(packet) = net_client.0.poll_network() {
        match packet {
            ServerPacket::ChunkDataCompressed {
                x,
                y,
                z,
                compressed_blocks,
            } => {
                let pos = ChunkPos { x, y, z };

                let Some(blocks) = decompress_chunk_blocks(&compressed_blocks) else {
                    eprintln!("Failed to decompress chunk at {}, {}, {}", x, y, z);
                    continue;
                };

                // Send immediate acknowledgment back to the server (Reliable UDP pattern)
                let _ = net_client.0.send_ack_chunk(x, y, z);

                // Despawn existing mesh if re-loading
                if let Some(old_entity) = loaded_chunks.0.remove(&pos) {
                    commands.entity(old_entity).despawn_recursive();
                }

                if let Some(entity) = spawn_chunk(
                    &mut commands,
                    &mut meshes,
                    &material,
                    &config,
                    x,
                    y,
                    z,
                    &blocks,
                ) {
                    loaded_chunks.0.insert(pos, entity);
                }
            }

            ServerPacket::UnloadChunk { x, y, z } => {
                let pos = ChunkPos { x, y, z };
                if let Some(entity) = loaded_chunks.0.remove(&pos) {
                    commands.entity(entity).despawn_recursive();
                }
            }
            ServerPacket::Pong => {}
            ServerPacket::ChunkData { x, y, z, blocks } => todo!(),
        }
    }
}

fn client_chunk_request_and_unload(
    time: Res<Time>,
    mut timer: ResMut<ChunkCheckTimer>,
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    mut loaded_chunks: ResMut<LoadedChunks>,
    net_client: ResMut<NetworkClient>,
) {
    let Ok(player_transform) = player_query.get_single() else {
        return;
    };

    // Always run unloads every frame so they are snappy
    let p_pos = player_transform.translation;
    let cs = CHUNK_SIZE as f32;
    let player_chunk_x = (p_pos.x / cs).floor() as i32;
    let player_chunk_y = (p_pos.y / cs).floor() as i32;
    let player_chunk_z = (p_pos.z / cs).floor() as i32;

    let max_radius = 30;
    let max_radius_sq = (max_radius * max_radius) as i32;

    let mut to_despawn = Vec::new();

    for (pos, &entity) in &loaded_chunks.0 {
        let dx = pos.x - player_chunk_x;
        let dy = pos.y - player_chunk_y;
        let dz = pos.z - player_chunk_z;

        if (dx * dx + dy * dy + dz * dz) > max_radius_sq {
            to_despawn.push((*pos, entity));
        }
    }

    for (pos, entity) in to_despawn {
        loaded_chunks.0.remove(&pos);
        commands.entity(entity).despawn_recursive();
    }

    // Tick the timer for re-requesting missing chunks to prevent CPU freezing
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    // Rate-limited missing chunk request pass
    let mut requests_sent = 0;
    let max_requests_per_batch = 12; 

    for dx in -max_radius..=max_radius {
        for dy in -max_radius..=max_radius {
            for dz in -max_radius..=max_radius {
                if dx * dx + dy * dy + dz * dz <= max_radius_sq {
                    let pos = ChunkPos {
                        x: player_chunk_x + dx,
                        y: player_chunk_y + dy,
                        z: player_chunk_z + dz,
                    };

                    if !loaded_chunks.0.contains_key(&pos) {
                        let _ = net_client.0.request_chunk(pos.x, pos.y, pos.z);
                        requests_sent += 1;
                        if requests_sent >= max_requests_per_batch {
                            return; // Spread out network recovery across ticks
                        }
                    }
                }
            }
        }
    }
}

fn spawn_chunk(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    material: &Res<ChunkMaterialHandle>,
    config: &VoxelWorldConfig,
    chunk_x: i32,
    chunk_y: i32,
    chunk_z: i32,
    blocks: &[BlockId; CHUNK_VOLUME],
) -> Option<Entity> {
    if blocks.len() != CHUNK_VOLUME {
        eprintln!("Received malformed chunk data packet!");
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
            .spawn(PbrBundle {
                mesh: meshes.add(mesh),
                material: material.0.clone(),
                transform: Transform::from_translation(chunk_world_pos),
                ..default()
            })
            .id(),
    )
}
