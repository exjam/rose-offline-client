use bevy::{
    input::{
        mouse::{MouseMotion, MouseScrollUnit, MouseWheel},
        Input,
    },
    math::{Quat, Vec2, Vec3},
    prelude::{
        Component, EventReader, KeyCode, Local, MouseButton, Query, Res, Time, Transform, With,
    },
    window::{CursorGrabMode, PrimaryWindow, Window},
};
use bevy_egui::EguiContexts;
use dolly::prelude::{CameraRig, LeftHanded, Position, Smooth, YawPitch};

#[derive(Component)]
pub struct FreeCamera {
    pub rig: CameraRig<LeftHanded>,
    pub move_speed: f32,
    pub drag_speed: f32,
}

impl FreeCamera {
    pub fn new(position: Vec3, yaw_degrees: f32, pitch_degrees: f32) -> Self {
        let mut yaw_pitch = YawPitch::new();
        yaw_pitch.rotate_yaw_pitch(yaw_degrees, pitch_degrees);
        Self {
            rig: CameraRig::builder()
                .with(Position::new(position))
                .with(yaw_pitch)
                .with(Smooth::new_position_rotation(1.0, 1.0))
                .build(),
            move_speed: 20.0,
            drag_speed: 4.0,
        }
    }
}

#[derive(Default)]
pub struct CameraControlState {
    pub is_dragging: bool,
    pub saved_cursor_position: Option<Vec2>,
}

pub fn free_camera_system(
    mut control_state: Local<CameraControlState>,
    mut query: Query<(&mut FreeCamera, &mut Transform)>,
    time: Res<Time>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mut mouse_wheel_reader: EventReader<MouseWheel>,
    keyboard: Res<Input<KeyCode>>,
    mouse_buttons: Res<Input<MouseButton>>,
    mut query_window: Query<&mut Window, With<PrimaryWindow>>,
    mut egui_ctx: EguiContexts,
) {
    let Ok(mut window) = query_window.get_single_mut() else {
        return;
    };

    let (mut free_camera, mut camera_transform) = if let Ok((a, b)) = query.get_single_mut() {
        (a, b)
    } else {
        if control_state.is_dragging {
            // Restore cursor state
            if let Some(saved_cursor_position) = control_state.saved_cursor_position.take() {
                window.set_cursor_position(Some(saved_cursor_position));
            }

            window.cursor.grab_mode = CursorGrabMode::None;
            window.cursor.visible = true;
            control_state.is_dragging = false;
        }

        return;
    };

    let allow_mouse_input = control_state.is_dragging || !egui_ctx.ctx_mut().wants_pointer_input();
    let allow_keyboard_input = !egui_ctx.ctx_mut().wants_keyboard_input();

    let left_pressed = mouse_buttons.pressed(MouseButton::Left);
    let right_pressed = mouse_buttons.pressed(MouseButton::Right);
    let middle_pressed = mouse_buttons.pressed(MouseButton::Middle);

    let mut cursor_delta = Vec2::ZERO;
    let mut move_speed_multiplier = 1.0;
    if allow_mouse_input {
        for event in mouse_motion_events.iter() {
            cursor_delta += event.delta;
        }

        for event in mouse_wheel_reader.iter() {
            match event.unit {
                MouseScrollUnit::Line => move_speed_multiplier *= 1.0 + event.y * 0.10,
                MouseScrollUnit::Pixel => move_speed_multiplier *= 1.0 + event.y * 0.0005,
            }
        }
        free_camera.move_speed = (free_camera.move_speed * move_speed_multiplier).max(1.0);
    }

    let mut drag_vec = Vec3::ZERO;
    let mut translate_vec = Vec3::ZERO;
    let mut move_vec = Vec3::ZERO;
    let mut speed_boost_multiplier = 1.0f32;
    if allow_keyboard_input {
        for key in keyboard.get_pressed() {
            match key {
                KeyCode::W => move_vec.z -= 1.0,      // Forward
                KeyCode::S => move_vec.z += 1.0,      // Backward
                KeyCode::A => move_vec.x -= 1.0,      // Left
                KeyCode::D => move_vec.x += 1.0,      // Right
                KeyCode::Q => translate_vec.y -= 1.0, // Down
                KeyCode::E => translate_vec.y += 1.0, // Up
                KeyCode::LShift => speed_boost_multiplier = 4.0,
                _ => {}
            }
        }
    }

    if middle_pressed || (left_pressed && right_pressed) {
        drag_vec.x += cursor_delta.x;
        drag_vec.z += cursor_delta.y;
    }

    let drag_speed = free_camera.drag_speed;
    let move_speed = free_camera.move_speed;

    if drag_vec.length_squared() > 0.0 {
        let yaw_radians = free_camera
            .rig
            .driver_mut::<YawPitch>()
            .yaw_degrees
            .to_radians();
        let yaw_rot = Quat::from_axis_angle(Vec3::Y, yaw_radians);
        let rot_x = yaw_rot * Vec3::X;
        let rot_z = yaw_rot * Vec3::Z;

        free_camera.rig.driver_mut::<Position>().translate(
            -(drag_vec.x * rot_x + (drag_vec.z * rot_z) - Vec3::new(0.0, drag_vec.y, 0.0))
                * time.delta_seconds()
                * speed_boost_multiplier
                * drag_speed,
        );
    }

    if move_vec.length_squared() > 0.0 || translate_vec.length_squared() > 0.0 {
        free_camera.rig.driver_mut::<Position>().translate(
            (camera_transform.rotation.mul_vec3(move_vec) + translate_vec)
                * time.delta_seconds()
                * speed_boost_multiplier
                * move_speed,
        );
    }

    if right_pressed && !left_pressed && !middle_pressed {
        let sensitivity = 0.1;
        free_camera
            .rig
            .driver_mut::<YawPitch>()
            .rotate_yaw_pitch(-sensitivity * cursor_delta.x, -sensitivity * cursor_delta.y);

        if !control_state.is_dragging {
            window.cursor.grab_mode = CursorGrabMode::Locked;
            window.cursor.visible = false;
            control_state.saved_cursor_position = window.cursor_position();
            control_state.is_dragging = true;
        }
    } else if control_state.is_dragging {
        if let Some(saved_cursor_position) = control_state.saved_cursor_position.take() {
            window.set_cursor_position(Some(saved_cursor_position));
        }

        window.cursor.grab_mode = CursorGrabMode::None;
        window.cursor.visible = true;
        control_state.is_dragging = false;
    }

    let calculated_transform = free_camera.rig.update(time.delta_seconds());
    camera_transform.translation = calculated_transform.position;
    camera_transform.rotation = calculated_transform.rotation;
}
