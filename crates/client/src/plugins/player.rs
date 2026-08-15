use crate::components::{Aabb, FpsCamera, Grounded, Player, Velocity};
use bevy::input::mouse::MouseMotion;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use bevy::prelude::*;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player)
            .add_systems(Update, (handle_cursor_lock, player_look, player_move));
    }
}

fn spawn_player(mut commands: Commands) {
    // Player Root Body (Collider base at y = 5.0)
    commands
        .spawn((
            SpatialBundle {
                transform: Transform::from_xyz(0.0, 5.0, 0.0),
                ..default()
            },
            Player,
            Velocity::default(),
            Grounded(false),
            Aabb {
                half_extents: Vec3::new(0.35, 0.9, 0.35), // Player dimensions: 0.7m x 1.8m x 0.7m
            },
        ))
        .with_children(|parent| {
            // First-person Camera offset at eye level
            parent.spawn((
                Camera3dBundle {
                    transform: Transform::from_xyz(0.0, 0.6, 0.0),
                    ..default()
                },
                FpsCamera::default(),
            ));
        });
}

fn handle_cursor_lock(
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    key_input: Res<ButtonInput<KeyCode>>,
) {
    let Ok(mut window) = windows.get_single_mut() else {
        return;
    };

    if mouse_button.just_pressed(MouseButton::Left) {
        window.cursor.grab_mode = CursorGrabMode::Locked;
        window.cursor.visible = false;
    }

    if key_input.just_pressed(KeyCode::Escape) {
        window.cursor.grab_mode = CursorGrabMode::None;
        window.cursor.visible = true;
    }
}

fn player_look(
    mut mouse_motion: EventReader<MouseMotion>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut query: Query<(&mut Transform, &mut FpsCamera)>,
    mut player_query: Query<&mut Transform, (With<Player>, Without<FpsCamera>)>,
) {
    let Ok(window) = windows.get_single() else {
        return;
    };
    if window.cursor.grab_mode != CursorGrabMode::Locked {
        return;
    }

    let Ok((mut camera_transform, mut camera)) = query.get_single_mut() else {
        return;
    };
    let Ok(mut player_transform) = player_query.get_single_mut() else {
        return;
    };

    for motion in mouse_motion.read() {
        camera.yaw -= motion.delta.x * camera.sensitivity;
        camera.pitch -= motion.delta.y * camera.sensitivity;

        // Clamp vertical pitch to prevent camera flips
        camera.pitch = camera.pitch.clamp(-1.54, 1.54);

        // Rotate body around Y-axis (Yaw)
        player_transform.rotation = Quat::from_rotation_y(camera.yaw);

        // Rotate camera locally around X-axis (Pitch)
        camera_transform.rotation = Quat::from_rotation_x(camera.pitch);
    }
}

fn player_move(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<(&Transform, &mut Velocity, &Grounded), With<Player>>,
) {
    let Ok((transform, mut velocity, grounded)) = player_query.get_single_mut() else {
        return;
    };

    let mut move_dir = Vec3::ZERO;
    let forward = transform.forward();
    let right = transform.right();

    // Flatten direction vectors so moving pitch doesn't change horizontal speed
    let forward_planar = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
    let right_planar = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();

    if keyboard.pressed(KeyCode::KeyW) {
        move_dir += forward_planar;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        move_dir -= forward_planar;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        move_dir += right_planar;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        move_dir -= right_planar;
    }

    let speed = 7.0;
    let move_vector = move_dir.normalize_or_zero() * speed;

    velocity.0.x = move_vector.x;
    velocity.0.z = move_vector.z;

    // Jump
    if keyboard.just_pressed(KeyCode::Space) && grounded.0 {
        velocity.0.y = 8.5;
    }
}
