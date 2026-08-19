mod components;
mod plugins;
mod resources;

use bevy::{pbr::wireframe::WireframePlugin, prelude::*};
use plugins::{FpsUiPlugin, PhysicsPlugin, PlayerPlugin, WorldPlugin};

fn main() {
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
        .add_plugins((
            WorldPlugin,
            PlayerPlugin,
            PhysicsPlugin,
            FpsUiPlugin,
            WireframePlugin,
        ))
        .run();
}
