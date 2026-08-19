use bevy::{
    app::{App, Plugin, Startup, Update},
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    ecs::{
        component::Component,
        query::With,
        system::{Commands, Query, Res},
    },
    render::color::Color,
    text::{Text, TextSection, TextStyle},
    ui::{PositionType, Style, Val, node_bundles::TextBundle},
    utils::default,
};

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
