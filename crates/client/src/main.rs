mod components;
mod plugins;
mod resources;

use std::path::Path;

use bevy::{pbr::wireframe::WireframePlugin, prelude::*};
use config::{ClientConfig, load_client_config};
use plugins::{FpsUiPlugin, PhysicsPlugin, PlayerPlugin, WorldPlugin};

use crate::{plugins::NetworkPlugin, resources::VoxelWorldConfig};

#[derive(Resource)]
pub struct ClientConfigRes(pub ClientConfig);

fn main() {
    let config = load_client_config(Path::new("./config")).unwrap_or_else(|_| {
        warn!("No client.toml found or failed to load, using default config.");
        ClientConfig {
            view_distance: core_types::ViewDistance(16),
            wireframe: true,
        }
    });

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Rust 3D Voxel Engine".into(),
                name: Some("voxel_engine".into()),
                present_mode: bevy::window::PresentMode::Immediate,
                ..default()
            }),
            ..default()
        }))
        .init_resource::<VoxelWorldConfig>()
        .insert_resource(ClientConfigRes(config))
        .add_plugins((
            NetworkPlugin,
            WorldPlugin,
            PlayerPlugin,
            PhysicsPlugin,
            FpsUiPlugin,
            //WireframePlugin,
        ))
        .run();
}
