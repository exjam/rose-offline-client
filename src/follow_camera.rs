use smooth_bevy_cameras::{LookAngles, LookTransform, LookTransformBundle, Smoother};

use bevy::{
    app::prelude::*,
    ecs::{bundle::Bundle, prelude::*},
    input::{
        mouse::{MouseMotion, MouseScrollUnit, MouseWheel},
        prelude::*,
    },
    math::prelude::*,
    render::prelude::*,
    transform::components::Transform,
};

#[derive(Default)]
pub struct FollowCameraPlugin;

impl Plugin for FollowCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(control_system)
            .add_event::<ControlEvent>()
            .add_system(default_input_map);
    }
}

#[derive(Bundle)]
pub struct FollowCameraBundle {
    controller: FollowCameraController,
    #[bundle]
    look_transform: LookTransformBundle,
    #[bundle]
    perspective: PerspectiveCameraBundle,
}

impl FollowCameraBundle {
    pub fn new(
        controller: FollowCameraController,
        mut perspective: PerspectiveCameraBundle,
        eye: Vec3,
        target: Vec3,
    ) -> Self {
        // Make sure the transform is consistent with the controller to start.
        perspective.transform = Transform::from_translation(eye).looking_at(target, Vec3::Y);

        Self {
            controller,
            look_transform: LookTransformBundle {
                transform: LookTransform { eye, target },
                smoother: Smoother::new(controller.smoothing_weight),
            },
            perspective,
        }
    }
}

/// A 3rd person camera that orbits around the target.
#[derive(Clone, Component, Copy, Debug)]
pub struct FollowCameraController {
    pub enabled: bool,
    pub mouse_rotate_sensitivity: Vec2,
    pub mouse_translate_sensitivity: Vec2,
    pub mouse_wheel_zoom_sensitivity: f32,
    pub pixels_per_line: f32,
    pub smoothing_weight: f32,
    pub follow_entity: Option<Entity>,
    pub follow_offset: Vec3,
}

impl Default for FollowCameraController {
    fn default() -> Self {
        Self {
            mouse_rotate_sensitivity: Vec2::splat(0.006),
            mouse_translate_sensitivity: Vec2::splat(0.008),
            mouse_wheel_zoom_sensitivity: 0.15,
            smoothing_weight: 0.8,
            enabled: true,
            pixels_per_line: 53.0,
            follow_entity: None,
            follow_offset: Vec3::default(),
        }
    }
}

pub enum ControlEvent {
    Orbit(Vec2),
    TranslateTarget(Vec2),
    Zoom(f32),
}

pub fn default_input_map(
    mut events: EventWriter<ControlEvent>,
    mut mouse_wheel_reader: EventReader<MouseWheel>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mouse_buttons: Res<Input<MouseButton>>,
    controllers: Query<&FollowCameraController>,
) {
    // Can only control one camera at a time.
    let controller = if let Some(controller) = controllers.iter().next() {
        controller
    } else {
        return;
    };
    let FollowCameraController {
        enabled,
        mouse_rotate_sensitivity,
        mouse_translate_sensitivity,
        mouse_wheel_zoom_sensitivity,
        pixels_per_line,
        follow_entity,
        ..
    } = *controller;

    if !enabled {
        return;
    }

    let mut cursor_delta = Vec2::ZERO;
    for event in mouse_motion_events.iter() {
        cursor_delta += event.delta;
    }

    if mouse_buttons.pressed(MouseButton::Right) {
        events.send(ControlEvent::Orbit(mouse_rotate_sensitivity * cursor_delta));
    }

    if follow_entity.is_none() && mouse_buttons.pressed(MouseButton::Left) {
        events.send(ControlEvent::TranslateTarget(
            mouse_translate_sensitivity * cursor_delta,
        ));
    }

    let mut scalar = 1.0;
    for event in mouse_wheel_reader.iter() {
        // scale the event magnitude per pixel or per line
        let scroll_amount = match event.unit {
            MouseScrollUnit::Line => event.y,
            MouseScrollUnit::Pixel => event.y / pixels_per_line,
        };
        scalar *= 1.0 - scroll_amount * mouse_wheel_zoom_sensitivity;
    }
    events.send(ControlEvent::Zoom(scalar));
}

pub fn control_system(
    mut events: EventReader<ControlEvent>,
    mut cameras: Query<(&FollowCameraController, &mut LookTransform, &Transform)>,
    query_target: Query<&Transform>,
) {
    // Can only control one camera at a time.
    let (controller, mut transform, scene_transform) =
        if let Some((controller, transform, scene_transform)) = cameras.iter_mut().next() {
            (controller, transform, scene_transform)
        } else {
            return;
        };

    if controller.enabled {
        let mut look_angles = LookAngles::from_vector(-transform.look_direction().unwrap());
        let mut radius_scalar = 1.0;

        for event in events.iter() {
            match event {
                ControlEvent::Orbit(delta) => {
                    look_angles.add_yaw(-delta.x);
                    look_angles.add_pitch(delta.y);
                }
                ControlEvent::TranslateTarget(delta) => {
                    let right_dir = scene_transform.rotation * -Vec3::X;
                    let up_dir = scene_transform.rotation * Vec3::Y;
                    transform.target += delta.x * right_dir + delta.y * up_dir;
                }
                ControlEvent::Zoom(scalar) => {
                    radius_scalar *= scalar;
                }
            }
        }

        if let Some(target) = controller
            .follow_entity
            .and_then(|entity| query_target.get(entity).ok())
        {
            transform.target = target.translation + controller.follow_offset;
        }

        look_angles.assert_not_looking_up();

        let new_radius = (radius_scalar * transform.radius())
            .min(1000000.0)
            .max(0.001);
        transform.eye = transform.target + new_radius * look_angles.unit_vector();
    } else {
        events.iter(); // Drop the events.
    }
}
