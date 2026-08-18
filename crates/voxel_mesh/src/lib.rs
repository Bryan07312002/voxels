use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use core_types::{ BlockId, CHUNK_SIZE, CHUNK_VOLUME };

const FACE_VERTICES: [[[f32; 3]; 4]; 6] = [
    // Top (+Y)
    [
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
        [1.0, 1.0, 1.0],
        [1.0, 1.0, 0.0],
    ],
    // Bottom (-Y)
    [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
    ],
    // Front (+Z)
    [
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 1.0],
        [1.0, 1.0, 1.0],
        [0.0, 1.0, 1.0],
    ],
    // Back (-Z)
    [
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
    ],
    // Right (+X)
    [
        [1.0, 0.0, 1.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [1.0, 1.0, 1.0],
    ],
    // Left (-X)
    [
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 1.0, 1.0],
        [0.0, 1.0, 0.0],
    ],
];

const FACE_NORMALS: [[f32; 3]; 6] = [
    [0.0, 1.0, 0.0],  // Top
    [0.0, -1.0, 0.0], // Bottom
    [0.0, 0.0, 1.0],  // Front
    [0.0, 0.0, -1.0], // Back
    [1.0, 0.0, 0.0],  // Right
    [-1.0, 0.0, 0.0], // Left
];

pub fn generate_chunk_mesh(blocks: &[BlockId; CHUNK_VOLUME], block_size: f32) -> Option<Mesh> {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    let mut vertex_count = 0u32;

    let cs = CHUNK_SIZE as i32;

    for ly in 0..cs {
        for lz in 0..cs {
            for lx in 0..cs {
                let idx = (lx + (lz * cs) + (ly * cs * cs)) as usize;
                if blocks[idx] == BlockId::AIR {
                    continue;
                }

                let visible_faces = [
                    !is_solid(blocks, lx, ly + 1, lz), // Top
                    !is_solid(blocks, lx, ly - 1, lz), // Bottom
                    !is_solid(blocks, lx, ly, lz + 1), // Front
                    !is_solid(blocks, lx, ly, lz - 1), // Back
                    !is_solid(blocks, lx + 1, ly, lz), // Right
                    !is_solid(blocks, lx - 1, ly, lz), // Left
                ];

                if !visible_faces.iter().any(|&v| v) {
                    continue;
                }

                let offset = Vec3::new(lx as f32, ly as f32, lz as f32) * block_size;

                for face_idx in 0..6 {
                    if !visible_faces[face_idx] {
                        continue;
                    }

                    let normal = FACE_NORMALS[face_idx];
                    for vert in FACE_VERTICES[face_idx] {
                        positions.push([
                            offset.x + vert[0] * block_size,
                            offset.y + vert[1] * block_size,
                            offset.z + vert[2] * block_size,
                        ]);
                        normals.push(normal);
                    }

                    let v = vertex_count;
                    indices.extend_from_slice(&[v, v + 1, v + 2, v, v + 2, v + 3]);
                    vertex_count += 4;
                }
            }
        }
    }

    if positions.is_empty() {
        return None;
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));

    Some(mesh)
}

#[inline]
fn is_solid(blocks: &[BlockId; CHUNK_VOLUME], x: i32, y: i32, z: i32) -> bool {
    let cs = CHUNK_SIZE as i32;
    if x < 0 || x >= cs || y < 0 || y >= cs || z < 0 || z >= cs {
        return false;
    }

    blocks[(x + (z * cs) + (y * cs * cs)) as usize] != BlockId::AIR
}
