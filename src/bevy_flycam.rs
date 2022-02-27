use bevy::{
    app::{Events, ManualEventReader},
    input::mouse::MouseMotion,
    prelude::*,
};

/// Keeps track of mouse motion events, pitch, and yaw
#[derive(Default)]
struct InputState {
    reader_motion: ManualEventReader<MouseMotion>,
}

#[derive(Default)]
struct CameraState {
    pitch: f32,
    yaw: f32,
}

/// Mouse sensitivity and movement speed
pub struct MovementSettings {
    pub sensitivity: f32,
    pub speed: f32,
}

impl Default for MovementSettings {
    fn default() -> Self {
        Self {
            sensitivity: 0.00012,
            speed: 12.,
        }
    }
}

/// Used in queries when you want flycams and not other cameras
#[derive(Component)]
pub struct FlyCam;

/// Grabs/ungrabs mouse cursor
fn toggle_grab_cursor(window: &mut Window) {
    window.set_cursor_lock_mode(!window.cursor_locked());
    window.set_cursor_visibility(!window.cursor_visible());
}

/// Grabs the cursor when game first starts
fn initial_grab_cursor(mut windows: ResMut<Windows>) {
    toggle_grab_cursor(windows.get_primary_mut().unwrap());
}

/// Spawns the `Camera3dBundle` to be controlled
fn setup_player(mut commands: Commands) {
    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_xyz(5200.0, 5200.0, 5.0).looking_at(Vec3::ZERO, Vec3::Z),
            ..Default::default()
        })
        .insert(FlyCam);
}

/// Handles keyboard input and movement
fn player_move(
    keys: Res<Input<KeyCode>>,
    time: Res<Time>,
    windows: Res<Windows>,
    settings: Res<MovementSettings>,
    camera_state: ResMut<CameraState>,
    mut query: Query<(&FlyCam, &mut Transform)>,
) {
    let window = windows.get_primary().unwrap();
    for (_camera, mut transform) in query.iter_mut() {
        let mut velocity = Vec3::ZERO;
        let local_y = if camera_state.pitch >= std::f32::consts::PI / 2.0 {
            -transform.local_y()
        } else {
            transform.local_y()
        };
        let forward = Vec3::new(local_y.x, local_y.y, 0.0);
        let right = Vec3::new(local_y.y, -local_y.x, 0.0);

        for key in keys.get_pressed() {
            if window.cursor_locked() {
                match key {
                    KeyCode::W => velocity += forward,
                    KeyCode::S => velocity -= forward,
                    KeyCode::A => velocity -= right,
                    KeyCode::D => velocity += right,
                    KeyCode::Space => velocity += Vec3::Z,
                    KeyCode::LShift => velocity -= Vec3::Z,
                    _ => (),
                }
            }
        }

        velocity = velocity.normalize();

        if !velocity.is_nan() {
            transform.translation += velocity * time.delta_seconds() * settings.speed
        }
    }
}

/// Handles looking around if cursor is locked
fn player_look(
    settings: Res<MovementSettings>,
    windows: Res<Windows>,
    mut state: ResMut<InputState>,
    mut camera_state: ResMut<CameraState>,
    motion: Res<Events<MouseMotion>>,
    mut query: Query<(&FlyCam, &mut Transform)>,
) {
    let window = windows.get_primary().unwrap();
    for (_camera, mut transform) in query.iter_mut() {
        for ev in state.reader_motion.iter(&motion) {
            if window.cursor_locked() {
                // Using smallest of height or width ensures equal vertical and horizontal sensitivity
                let window_scale = window.height().min(window.width());

                camera_state.pitch -=
                    (settings.sensitivity * ev.delta.y * window_scale).to_radians();
                camera_state.yaw -= (settings.sensitivity * ev.delta.x * window_scale).to_radians();
            }

            camera_state.pitch = camera_state.pitch.clamp(0.0, std::f32::consts::PI);

            // Order is important to prevent unintended roll
            transform.rotation = Quat::from_axis_angle(Vec3::Z, camera_state.yaw)
                * Quat::from_axis_angle(Vec3::X, camera_state.pitch);
        }
    }
}

fn cursor_grab(keys: Res<Input<KeyCode>>, mut windows: ResMut<Windows>) {
    let window = windows.get_primary_mut().unwrap();
    if keys.just_pressed(KeyCode::Escape) {
        toggle_grab_cursor(window);
    }
}

/// Contains everything needed to add first-person fly camera behavior to your game
pub struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputState>()
            .init_resource::<CameraState>()
            .init_resource::<MovementSettings>()
            .add_startup_system(setup_player)
            .add_startup_system(initial_grab_cursor)
            .add_system(player_move)
            .add_system(player_look)
            .add_system(cursor_grab);
    }
}
