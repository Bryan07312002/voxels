use crate::components::{Aabb, FpsCamera, Grounded, Player, Velocity};
use crate::plugins::network::NetworkClient;
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PositionSendTimer>()
            .add_systems(Startup, spawn_player)
            .add_systems(
                Update,
                (
                    handle_cursor_lock,
                    player_look,
                    player_move,
                    send_player_position,
                ),
            );
    }
}

#[derive(Resource)]
pub struct PositionSendTimer(pub Timer);

impl Default for PositionSendTimer {
    fn default() -> Self {
        // Send position updates 25 times per second (every 40ms) for smooth chunk streaming
        Self(Timer::from_seconds(0.04, TimerMode::Repeating))
    }
}

fn spawn_player(mut commands: Commands) {
    // Spawn player safely above the grass surface at y = 6.0
    commands
        .spawn((
            SpatialBundle {
                transform: Transform::from_xyz(0.0, 7.5, 0.0),
                ..default()
            },
            Player,
            Velocity::default(),
            Grounded(false),
            Aabb {
                half_extents: Vec3::new(0.35, 0.9, 0.35),
            },
        ))
        .with_children(|parent| {
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
    mut player_query: Query<(&Transform, &mut Velocity), With<Player>>,
) {
    let Ok((transform, mut velocity)) = player_query.get_single_mut() else {
        return;
    };

    let mut move_dir = Vec3::ZERO;
    let forward = transform.forward();
    let right = transform.right();

    // Flatten horizontal direction vectors
    let forward_planar = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
    let right_planar = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();

    // Horizontal Movement (WASD)
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

    // Vertical Movement (Space = Up, Shift = Down)
    if keyboard.pressed(KeyCode::Space) {
        move_dir += Vec3::Y;
    }

    if keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ShiftRight) {
        move_dir -= Vec3::Y;
    }

    let speed = 70.0;
    let move_vector = move_dir.normalize_or_zero() * speed;

    // Apply 3D movement to all axes
    velocity.0 = move_vector;
}

fn send_player_position(
    time: Res<Time>,
    mut timer: ResMut<PositionSendTimer>,
    player_query: Query<&Transform, With<Player>>,
    net_client: Option<Res<NetworkClient>>,
) {
    let Some(net) = net_client else {
        return;
    };

    // Tick the timer independent of frame-rate
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    let Ok(transform) = player_query.get_single() else {
        return;
    };

    let pos = transform.translation;
    let _ = net.0.send_player_position(pos.x, pos.y, pos.z);
}
