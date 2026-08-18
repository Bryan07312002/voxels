use crate::resources::VoxelWorldConfig;
use bevy::prelude::*;
use core_types::{CHUNK_SIZE, CHUNK_VOLUME};
use net::{ClientChannel, ServerPacket};
use voxel_mesh::generate_chunk_mesh;

#[derive(Resource)]
pub struct NetworkClient(pub ClientChannel);

#[derive(Resource)]
pub struct ChunkMaterialHandle(pub Handle<StandardMaterial>);

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(client) = ClientChannel::new("127.0.0.1:25565") {
            info!("Network client bound to server at 127.0.0.1:25565");
            app.insert_resource(NetworkClient(client));
        } else {
            error!("Failed to bind UDP client channel!");
        }

        app.add_systems(Startup, setup_world_environment)
            .add_systems(Update, poll_network_system);
    }
}

fn setup_world_environment(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    net_client: Option<Res<NetworkClient>>,
) {
    // Shared material cached across all spawned chunk entities
    let material_handle = materials.add(StandardMaterial {
        base_color: Color::rgb(0.2, 0.8, 0.2),
        cull_mode: None,
        ..default()
    });
    commands.insert_resource(ChunkMaterialHandle(material_handle));

    // Sun & Ambient Lighting
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

    // Request initial chunk at origin
    if let Some(net) = net_client {
        if let Err(e) = net.0.request_chunk(0, 0, 0) {
            warn!("Failed to request initial chunk: {e:?}");
        }
    }
}

fn poll_network_system(
    mut commands: Commands,
    mut net_client: ResMut<NetworkClient>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Res<ChunkMaterialHandle>,
    config: Res<VoxelWorldConfig>,
) {
    while let Some(packet) = net_client.0.poll_network() {
        match packet {
            ServerPacket::ChunkData { x, y, z, blocks } => {
                info!("Received chunk at ({x}, {y}, {z})");
                spawn_chunk(
                    &mut commands,
                    &mut meshes,
                    &material,
                    &config,
                    x,
                    y,
                    z,
                    &blocks,
                );
            }
            ServerPacket::Pong => {}
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
    blocks: &[u16; CHUNK_VOLUME],
) {
    let Some(mesh) = generate_chunk_mesh(blocks, config.block_size) else {
        return; // Empty chunk
    };

    let chunk_world_pos = Vec3::new(
        chunk_x as f32 * (CHUNK_SIZE as f32) * config.block_size,
        chunk_y as f32 * (CHUNK_SIZE as f32) * config.block_size,
        chunk_z as f32 * (CHUNK_SIZE as f32) * config.block_size,
    );

    commands.spawn(PbrBundle {
        mesh: meshes.add(mesh),
        material: material.0.clone(),
        transform: Transform::from_translation(chunk_world_pos),
        ..default()
    });
}
