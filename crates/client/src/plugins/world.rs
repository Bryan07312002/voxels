use crate::resources::VoxelWorldConfig;

use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;

use bevy::{prelude::*, render::render_resource::Face};
use core_types::CHUNK_SIZE;
use net::{CHUNK_VOLUME, ClientChannel, ServerPacket};

#[derive(Resource)]
pub struct NetworkClient(pub ClientChannel);

#[derive(Resource)]
pub struct VoxelAssets {
    pub mesh: Handle<Mesh>,
    pub grass_material: Handle<StandardMaterial>,
    pub outline_material: Handle<StandardMaterial>,
    pub half_extent: Vec3,
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        // Initialize network client resource
        if let Ok(client) = ClientChannel::new("127.0.0.1:25565") {
            println!("OK returned");
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
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<VoxelWorldConfig>,
    net_client: Option<Res<NetworkClient>>,
) {
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

    // Store shared mesh and material assets in a resource
    let half_extent = Vec3::splat(config.block_size * 0.5);
    let voxel_assets = VoxelAssets {
        mesh: meshes.add(Cuboid::new(
            config.block_size,
            config.block_size,
            config.block_size,
        )),
        grass_material: materials.add(StandardMaterial {
            base_color: Color::rgb(0.2, 0.7, 0.3),
            perceptual_roughness: 0.9,
            ..default()
        }),
        outline_material: materials.add(StandardMaterial {
            base_color: Color::BLACK,
            unlit: true,
            cull_mode: Some(Face::Front),
            ..default()
        }),

        half_extent,
    };
    commands.insert_resource(voxel_assets);

    // Request initial chunks from the server around origin
    if let Some(net) = net_client {
        let _ = net.0.request_chunk(0, 0, 0);
        //     for x in -config.map_radius..=config.map_radius {
        //         for z in -config.map_radius..=config.map_radius {
        //             println!("request_chunk");
        //             let _ = net.0.request_chunk(x, 0, z);
        //         }
        //     }
    }
}

fn spawn_chunk(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    config: &VoxelWorldConfig,
    chunk_x: i32,
    chunk_y: i32,
    chunk_z: i32,
    blocks: &[u16; CHUNK_VOLUME],
) {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let mut vertex_index = 0;

    for ly in 0..CHUNK_SIZE {
        for lz in 0..CHUNK_SIZE {
            for lx in 0..CHUNK_SIZE {
                let idx = lx + (lz * CHUNK_SIZE) + (ly * CHUNK_SIZE * CHUNK_SIZE);
                if blocks[idx] == 0 {
                    continue;
                }

                let lx_i = lx as i32;
                let ly_i = ly as i32;
                let lz_i = lz as i32;

                // Face is visible ONLY if neighbor is NOT solid (i.e. air)
                let top = !is_solid(blocks, lx_i, ly_i + 1, lz_i);
                let bottom = !is_solid(blocks, lx_i, ly_i - 1, lz_i);
                let front = !is_solid(blocks, lx_i, ly_i, lz_i + 1);
                let back = !is_solid(blocks, lx_i, ly_i, lz_i - 1);
                let right = !is_solid(blocks, lx_i + 1, ly_i, lz_i);
                let left = !is_solid(blocks, lx_i - 1, ly_i, lz_i);

                // Skip if completely buried inside solid ground
                if !top && !bottom && !front && !back && !right && !left {
                    continue;
                }

                let x = lx as f32 * config.block_size;
                let y = ly as f32 * config.block_size;
                let z = lz as f32 * config.block_size;

                add_cube_faces(
                    &mut positions,
                    &mut normals,
                    &mut indices,
                    &mut vertex_index,
                    Vec3::new(x, y, z),
                    config.block_size,
                    top,
                    bottom,
                    front,
                    back,
                    right,
                    left,
                );
            }
        }
    }
    if positions.is_empty() {
        return; // Empty chunk
    }

    // Build 1 combined mesh for the entire chunk
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));

    let chunk_world_pos = Vec3::new(
        chunk_x as f32 * (CHUNK_SIZE as f32) * config.block_size,
        chunk_y as f32 * (CHUNK_SIZE as f32) * config.block_size,
        chunk_z as f32 * (CHUNK_SIZE as f32) * config.block_size,
    );

    // Spawn ONLY 1 Entity for the entire chunk!
    commands.spawn(PbrBundle {
        mesh: meshes.add(mesh),
        material: materials.add(Color::rgb(0.2, 0.8, 0.2)),
        transform: Transform::from_translation(chunk_world_pos),
        ..default()
    });
}

fn is_solid(blocks: &[u16; CHUNK_VOLUME], x: i32, y: i32, z: i32) -> bool {
    // If coordinate is outside this chunk's boundaries, treat it as air (render the outer boundary face)
    if x < 0
        || x >= CHUNK_SIZE as i32
        || y < 0
        || y >= CHUNK_SIZE as i32
        || z < 0
        || z >= CHUNK_SIZE as i32
    {
        return false;
    }

    let idx = (x + (z * CHUNK_SIZE as i32) + (y * CHUNK_SIZE as i32 * CHUNK_SIZE as i32)) as usize;
    blocks[idx] != 0
}

fn add_cube_faces(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    vertex_count: &mut u32,
    pos: Vec3,
    s: f32,
    // Visibility flags for each direction
    top: bool,
    bottom: bool,
    front: bool,
    back: bool,
    right: bool,
    left: bool,
) {
    let x = pos.x;
    let y = pos.y;
    let z = pos.z;

    let faces: [(bool, [[f32; 3]; 4], [f32; 3]); 6] = [
        (
            top,
            [
                [x, y + s, z],
                [x, y + s, z + s],
                [x + s, y + s, z + s],
                [x + s, y + s, z],
            ],
            [0.0, 1.0, 0.0],
        ),
        (
            bottom,
            [[x, y, z], [x + s, y, z], [x + s, y, z + s], [x, y, z + s]],
            [0.0, -1.0, 0.0],
        ),
        (
            front,
            [
                [x, y, z + s],
                [x + s, y, z + s],
                [x + s, y + s, z + s],
                [x, y + s, z + s],
            ],
            [0.0, 0.0, 1.0],
        ),
        (
            back,
            [[x + s, y, z], [x, y, z], [x, y + s, z], [x + s, y + s, z]],
            [0.0, 0.0, -1.0],
        ),
        (
            right,
            [
                [x + s, y, z + s],
                [x + s, y, z],
                [x + s, y + s, z],
                [x + s, y + s, z + s],
            ],
            [1.0, 0.0, 0.0],
        ),
        (
            left,
            [[x, y, z], [x, y, z + s], [x, y + s, z + s], [x, y + s, z]],
            [-1.0, 0.0, 0.0],
        ),
    ];

    for (should_draw, face_verts, normal) in faces {
        if !should_draw {
            continue; // Skip hidden internal face
        }

        positions.extend_from_slice(&face_verts);
        normals.extend_from_slice(&[normal; 4]);

        let v = *vertex_count;
        indices.extend_from_slice(&[v, v + 1, v + 2, v, v + 2, v + 3]);

        *vertex_count += 4;
    }
}

fn poll_network_system(
    mut commands: Commands,
    mut net_client: ResMut<NetworkClient>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<VoxelWorldConfig>,
) {
    // Process all pending packets received on this frame
    while let Some(packet) = net_client.0.poll_network() {
        match packet {
            ServerPacket::ChunkData { x, y, z, blocks } => {
                println!("Chunk packet received at ({x}, {y}, {z})");
                spawn_chunk(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
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
