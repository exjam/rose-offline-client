use bevy::{
    input::{
        mouse::{MouseMotion, MouseWheel},
        Input,
    },
    math::{Quat, Vec2, Vec3},
    prelude::{
        Component, Entity, EventReader, GlobalTransform, Local, MouseButton, Query, Res, ResMut,
        Time, Transform, With,
    },
    window::{CursorGrabMode, PrimaryWindow, Window},
};
use bevy_egui::EguiContext;
use bevy_rapier3d::{
    plugin::RapierContext,
    prelude::{Collider, CollisionGroups, QueryFilter},
};
use dolly::prelude::{Arm, CameraRig, LeftHanded, Position, Smooth, YawPitch};

use crate::components::{
    COLLISION_FILTER_COLLIDABLE, COLLISION_FILTER_MOVEABLE, COLLISION_GROUP_PHYSICS_TOY,
};

#[derive(Component)]
pub struct OrbitCamera {
    pub rig: CameraRig<LeftHanded>,
    pub has_initial_position: bool,
    pub follow_entity: Entity,
    pub follow_offset: Vec3,
    pub follow_distance: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    pub current_distance: ExpSmoothed<f32>,
}

impl OrbitCamera {
    pub fn new(follow_entity: Entity, follow_offset: Vec3, follow_distance: f32) -> Self {
        Self {
            rig: CameraRig::builder()
                .with(Position::new(Vec3::new(0.0, 0.0, 0.0)))
                .with(YawPitch::new().yaw_degrees(45.0).pitch_degrees(-30.0))
                .with(Smooth::new_position_rotation(1.0, 1.0))
                .with(Arm::new(Vec3::Z * 4.0))
                .build(),
            has_initial_position: false,
            follow_entity,
            follow_offset,
            follow_distance,
            min_distance: 1.0,
            max_distance: 1000.0,
            current_distance: Default::default(),
        }
    }
}

#[derive(Default)]
pub struct CameraControlState {
    pub is_dragging: bool,
    pub saved_cursor_position: Option<Vec2>,
}

pub fn orbit_camera_system(
    mut control_state: Local<CameraControlState>,
    mut query: Query<(&mut OrbitCamera, &mut Transform)>,
    query_global_transform: Query<&GlobalTransform>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mut mouse_wheel_reader: EventReader<MouseWheel>,
    mut query_window: Query<&mut Window, With<PrimaryWindow>>,
    mut egui_ctx: ResMut<EguiContext>,
    mouse_buttons: Res<Input<MouseButton>>,
    time: Res<Time>,
    rapier_context: Res<RapierContext>,
) {
    let Ok(window) = query_window.get_single_mut() else {
        return;
    };

    let (mut orbit_camera, mut camera_transform) = if let Ok((a, b)) = query.get_single_mut() {
        (a, b)
    } else {
        if control_state.is_dragging {
            // Restore cursor state
            if let Some(saved_cursor_position) = control_state.saved_cursor_position.take() {
                window.set_cursor_position(Some(saved_cursor_position));
            }

            window.set_cursor_grab_mode(CursorGrabMode::None);
            window.set_cursor_visibility(true);
            control_state.is_dragging = false;
        }

        return;
    };

    // If the camera has not had its initial position yet, move straight to entity
    if !orbit_camera.has_initial_position {
        if let Ok(follow_transform) = query_global_transform.get(orbit_camera.follow_entity) {
            orbit_camera.rig = CameraRig::builder()
                .with(Position::new(follow_transform.translation()))
                .with(YawPitch::new().yaw_degrees(45.0).pitch_degrees(-30.0))
                .with(Smooth::new_position_rotation(1.0, 1.0))
                .with(Arm::new(Vec3::Z * orbit_camera.follow_distance))
                .build();
            orbit_camera.has_initial_position = true;
        }

        return;
    }

    let allow_mouse_input = control_state.is_dragging || !egui_ctx.ctx_mut().wants_pointer_input();
    let right_pressed = mouse_buttons.pressed(MouseButton::Right);
    let mut drag_delta = Vec2::ZERO;
    let mut zoom_multiplier = 1.0;

    if right_pressed {
        if allow_mouse_input {
            for event in mouse_motion_events.iter() {
                drag_delta += event.delta;
            }

            if !control_state.is_dragging {
                window.set_cursor_grab_mode(CursorGrabMode::Locked);
                window.set_cursor_visibility(false);
                control_state.saved_cursor_position = window.cursor_position();
            }
        }

        control_state.is_dragging = true;
    } else {
        if control_state.is_dragging {
            if let Some(saved_cursor_position) = control_state.saved_cursor_position.take() {
                window.set_cursor_position(Some(saved_cursor_position));
            }

            window.set_cursor_grab_mode(CursorGrabMode::None);
            window.set_cursor_visibility(true);
        }

        control_state.is_dragging = false;
    }

    if allow_mouse_input {
        for event in mouse_wheel_reader.iter() {
            zoom_multiplier *= 1.0 - event.y * 0.10;
        }
    }

    // Follow target
    let mut camera_collide_distance = orbit_camera.max_distance;

    if let Ok(follow_transform) = query_global_transform.get(orbit_camera.follow_entity) {
        let follow_position = follow_transform.translation() + orbit_camera.follow_offset;
        orbit_camera.rig.driver_mut::<Position>().position = follow_position;

        // Camera collision
        let ray_direction = (camera_transform.translation - follow_position).normalize();
        let ball_radius = 0.5;
        if let Some((_, distance)) = rapier_context.cast_shape(
            follow_position + ray_direction * ball_radius,
            Quat::default(),
            ray_direction,
            &Collider::ball(ball_radius),
            orbit_camera.max_distance,
            QueryFilter::new().groups(CollisionGroups::new(
                COLLISION_FILTER_MOVEABLE | COLLISION_FILTER_COLLIDABLE,
                !COLLISION_GROUP_PHYSICS_TOY,
            )),
        ) {
            camera_collide_distance = distance.toi;
        }
    }

    // Rotate with mouse drag
    if right_pressed {
        let sensitivity = 0.1;
        orbit_camera
            .rig
            .driver_mut::<YawPitch>()
            .rotate_yaw_pitch(-sensitivity * drag_delta.x, -sensitivity * drag_delta.y);
    }

    // Adjust zoom with mouse wheel
    orbit_camera.follow_distance = (orbit_camera.follow_distance * zoom_multiplier)
        .clamp(orbit_camera.min_distance, orbit_camera.max_distance);

    let target_distance = orbit_camera.follow_distance;
    let arm_distance = orbit_camera.current_distance.exp_smooth_towards(
        &target_distance,
        ExpSmoothingParams {
            smoothness: 1.0,
            output_offset_scale: 1.0,
            delta_time_seconds: time.delta_seconds(),
        },
    );

    if arm_distance > camera_collide_distance {
        orbit_camera.current_distance.0 = Some(camera_collide_distance);
        orbit_camera.rig.driver_mut::<Arm>().offset.z = camera_collide_distance;
    } else {
        orbit_camera.rig.driver_mut::<Arm>().offset.z = arm_distance;
    }

    // Update camera
    let calculated_transform = orbit_camera.rig.update(time.delta_seconds());
    camera_transform.translation = calculated_transform.position;
    camera_transform.rotation = calculated_transform.rotation;
}

pub trait Interpolate {
    fn interpolate(self, other: Self, t: f32) -> Self;
}

impl Interpolate for f32 {
    fn interpolate(self, other: Self, t: f32) -> Self {
        self + ((other - self) * t)
    }
}

impl Interpolate for Vec3 {
    fn interpolate(self, other: Self, t: f32) -> Self {
        Vec3::lerp(self, other, t)
    }
}

impl Interpolate for Quat {
    fn interpolate(self, other: Self, t: f32) -> Self {
        // Technically should be a `slerp` for framerate independence, but the latter
        // will rotate in the negative direction when interpolating a 180..360 degree rotation
        // to the 0..180 range. See the comment about `yaw_degrees` in `YawPitch` for more details.
        Quat::lerp(self.normalize(), other.normalize(), t).normalize()
    }
}

pub struct ExpSmoothingParams {
    pub smoothness: f32,
    pub output_offset_scale: f32,
    pub delta_time_seconds: f32,
}

#[derive(Default, Debug)]
pub struct ExpSmoothed<T: Interpolate + Copy + std::fmt::Debug>(Option<T>);

impl<T: Interpolate + Copy + std::fmt::Debug> ExpSmoothed<T> {
    pub fn exp_smooth_towards(&mut self, other: &T, params: ExpSmoothingParams) -> T {
        // An ad-hoc multiplier to make default smoothness parameters
        // produce good-looking results.
        const SMOOTHNESS_MULT: f32 = 8.0;

        // Calculate the exponential blending based on frame time
        let interp_t = 1.0
            - (-SMOOTHNESS_MULT * params.delta_time_seconds / params.smoothness.max(1e-5)).exp();

        let prev = self.0.unwrap_or(*other);
        let smooth = prev.interpolate(*other, interp_t);

        self.0 = Some(smooth);

        #[allow(clippy::float_cmp)]
        if params.output_offset_scale != 1.0 {
            Interpolate::interpolate(*other, smooth, params.output_offset_scale)
        } else {
            smooth
        }
    }
}
