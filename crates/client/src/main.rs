mod components;
mod plugins;
mod resources;

use bevy::prelude::*;
use plugins::{PhysicsPlugin, PlayerPlugin, WorldPlugin};
use resources::VoxelWorldConfig;

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};

#[derive(Component)]
pub struct FpsText;

pub struct FpsUiPlugin;

impl Plugin for FpsUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin)
            .add_systems(Startup, setup_fps_ui)
            .add_systems(Update, update_fps_ui);
    }
}

fn setup_fps_ui(mut commands: Commands) {
    // FPS Text Overlay
    commands.spawn((
        TextBundle::from_sections([
            TextSection::new(
                "FPS: ",
                TextStyle {
                    font_size: 20.0,
                    color: Color::WHITE,
                    ..default()
                },
            ),
            TextSection::from_style(TextStyle {
                font_size: 20.0,
                color: Color::GREEN,
                ..default()
            }),
        ])
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        }),
        FpsText,
    ));
}

fn update_fps_ui(diagnostics: Res<DiagnosticsStore>, mut query: Query<&mut Text, With<FpsText>>) {
    for mut text in &mut query {
        if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(value) = fps.smoothed() {
                text.sections[1].value = format!("{value:.0}");
            }
        }
    }
}

fn main() {
    // let mut client = ClientChannel::new("127.0.0.1:25565").unwrap();
    //
    // client.request_chunk(0, 0, 0).unwrap();
    // println!("📤 Chunk requested. Polling response...");
    //
    // let packet = client.poll_network();
    //
    // match packet {
    //     Some(ServerPacket::ChunkData { x, y, z, blocks }) => {
    //         println!("⚡ Received ChunkData [{}, {}, {}]!", x, y, z);
    //         println!(
    //             "   First Block: {}, Total Blocks: {}",
    //             blocks[0],
    //             blocks.len()
    //         );
    //     }
    //     None => todo!(),
    //     Some(ServerPacket::Pong) => todo!(),
    // }

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Rust 3D Voxel Engine".into(),
                name: Some("voxel_engine".into()),
                present_mode: bevy::window::PresentMode::AutoVsync,
                ..default()
            }),
            ..default()
        }))
        .init_resource::<VoxelWorldConfig>()
        .add_plugins((WorldPlugin, PlayerPlugin, PhysicsPlugin, FpsUiPlugin))
        .run();
}

//
// fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let mut client = ClientChannel::new("127.0.0.1:25565")?;
//
//     client.request_chunk(0, 0, 0).unwrap();
//     println!("📤 Chunk requested. Polling response...");
//
//     let packet = client.poll_network();
//
//     match packet {
//         Some(ServerPacket::ChunkData { x, y, z, blocks }) => {
//             println!("⚡ Received ChunkData [{}, {}, {}]!", x, y, z);
//             println!("   First Block: {}, Total Blocks: {}", blocks[0], blocks.len());
//         },
//         None => todo!(),
//         Some(ServerPacket::Pong) => todo!()
//
//     }
//
//     Ok(())
// }
