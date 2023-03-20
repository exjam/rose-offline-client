use bevy::{
    hierarchy::BuildChildren,
    input::Input,
    math::Vec3,
    pbr::{AlphaMode, StandardMaterial},
    prelude::{
        shape, Assets, Camera, Camera3d, Color, Commands, ComputedVisibility, GlobalTransform,
        Handle, KeyCode, Local, Mesh, Query, Res, ResMut, Time, Transform, Visibility, With,
    },
    render::camera::Projection,
    window::{PrimaryWindow, Window},
};
use bevy_egui::{egui, EguiContext};
use bevy_rapier3d::{
    plugin::{RapierConfiguration, RapierContext},
    prelude::{Collider, CollisionGroups, Group, QueryFilter, Restitution, RigidBody},
};
use rand::prelude::SliceRandom;
use rose_data::NpcId;
use rose_game_common::components::Npc;

use crate::{
    components::{ColliderEntity, COLLISION_FILTER_CLICKABLE, COLLISION_GROUP_PHYSICS_TOY},
    ray_from_screenspace::ray_from_screenspace,
    ui::UiStateDebugWindows,
};

const COLOR_LIST: [Color; 38] = [
    Color::ALICE_BLUE,
    Color::ANTIQUE_WHITE,
    Color::AQUAMARINE,
    Color::AZURE,
    Color::BEIGE,
    Color::BISQUE,
    Color::BLACK,
    Color::BLUE,
    Color::CRIMSON,
    Color::CYAN,
    Color::DARK_GRAY,
    Color::DARK_GREEN,
    Color::FUCHSIA,
    Color::GOLD,
    Color::GRAY,
    Color::GREEN,
    Color::INDIGO,
    Color::LIME_GREEN,
    Color::MAROON,
    Color::MIDNIGHT_BLUE,
    Color::NAVY,
    Color::NONE,
    Color::OLIVE,
    Color::ORANGE,
    Color::ORANGE_RED,
    Color::PINK,
    Color::PURPLE,
    Color::RED,
    Color::SALMON,
    Color::SEA_GREEN,
    Color::SILVER,
    Color::TEAL,
    Color::TOMATO,
    Color::TURQUOISE,
    Color::VIOLET,
    Color::WHITE,
    Color::YELLOW,
    Color::YELLOW_GREEN,
];

pub struct UiDebugPhysicsState {
    pub spawn_balls: bool,
    pub spawn_height: f32,
    pub restitution: f32,
    pub ball_radius: f32,
    pub spawn_timer: f32,
    pub spawn_interval: f32,
    pub materials: Vec<Handle<StandardMaterial>>,
    pub ball_meshes: Vec<(f32, Handle<Mesh>)>,
}

impl Default for UiDebugPhysicsState {
    fn default() -> Self {
        Self {
            spawn_balls: false,
            spawn_height: 10.0,
            restitution: 0.7,
            ball_radius: 0.5,
            spawn_timer: 0.0,
            spawn_interval: 0.1,
            materials: Vec::with_capacity(COLOR_LIST.len()),
            ball_meshes: Vec::with_capacity(32),
        }
    }
}

pub fn ui_debug_physics_system(
    mut commands: Commands,
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
    mut ui_state_debug_physics: Local<UiDebugPhysicsState>,
    mut rapier_configuration: ResMut<RapierConfiguration>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    key_code_input: Res<Input<KeyCode>>,
    rapier_context: Res<RapierContext>,
    time: Res<Time>,
    query_primary_window: Query<&Window, With<PrimaryWindow>>,
    query_camera: Query<(&Camera, &Projection, &GlobalTransform), With<Camera3d>>,
) {
    if !ui_state_debug_windows.debug_ui_open {
        return;
    }
    let window = query_primary_window.single();

    egui::Window::new("Physics")
        .open(&mut ui_state_debug_windows.physics_open)
        .show(egui_context.ctx_mut(), |ui| {
            egui::Grid::new("debug_physics")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Physics pipeline active:");
                    ui.checkbox(&mut rapier_configuration.physics_pipeline_active, "Enabled");
                    ui.end_row();
                });

            ui.separator();

            egui::Grid::new("debug_physics_spawn")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Spawn balls with B key:");
                    ui.checkbox(&mut ui_state_debug_physics.spawn_balls, "Enabled");
                    ui.end_row();

                    ui.label("Spawn interval:");
                    ui.add(
                        egui::Slider::new(&mut ui_state_debug_physics.spawn_interval, 0.01..=5.0)
                            .show_value(true),
                    );
                    ui.end_row();

                    ui.label("Spawn height:");
                    ui.add(
                        egui::Slider::new(&mut ui_state_debug_physics.spawn_height, 1.0..=100.0)
                            .show_value(true),
                    );
                    ui.end_row();

                    ui.label("Ball Radius:");
                    ui.add(
                        egui::Slider::new(&mut ui_state_debug_physics.ball_radius, 0.1..=10.0)
                            .show_value(true),
                    );
                    ui.end_row();

                    ui.label("Restitution:");
                    ui.add(
                        egui::Slider::new(&mut ui_state_debug_physics.restitution, 0.0..=1.0)
                            .show_value(true),
                    );
                    ui.end_row();
                });
        });

    if ui_state_debug_physics.spawn_balls
        && key_code_input.pressed(KeyCode::B)
        && !egui_context.ctx_mut().wants_keyboard_input()
        && !egui_context.ctx_mut().wants_pointer_input()
    {
        if ui_state_debug_physics.materials.is_empty() {
            // Initialise our materials
            for color in COLOR_LIST.iter() {
                ui_state_debug_physics
                    .materials
                    .push(materials.add(StandardMaterial {
                        base_color: *color.clone().set_a(0.3),
                        alpha_mode: AlphaMode::Blend,
                        ..Default::default()
                    }));
            }
        }

        ui_state_debug_physics.spawn_timer += time.delta_seconds();

        let cursor_position = window.cursor_position();
        if let Some(cursor_position) = cursor_position {
            let (camera, camera_projection, camera_transform) = query_camera.single();

            if let Some((ray_origin, ray_direction)) = ray_from_screenspace(
                cursor_position,
                window,
                camera,
                camera_projection,
                camera_transform,
            ) {
                if let Some((_, distance)) = rapier_context.cast_ray(
                    ray_origin,
                    ray_direction,
                    10000000.0,
                    false,
                    QueryFilter::new().groups(CollisionGroups::new(
                        COLLISION_FILTER_CLICKABLE,
                        Group::all(),
                    )),
                ) {
                    while ui_state_debug_physics.spawn_timer
                        >= ui_state_debug_physics.spawn_interval
                    {
                        let mesh = if let Some(ball_mesh) = ui_state_debug_physics
                            .ball_meshes
                            .iter()
                            .find(|(radius, _)| *radius == ui_state_debug_physics.ball_radius)
                            .map(|(_, mesh)| mesh)
                        {
                            ball_mesh.clone()
                        } else {
                            let ball_radius = ui_state_debug_physics.ball_radius;
                            ui_state_debug_physics.ball_meshes.push((
                                ball_radius,
                                meshes.add(
                                    shape::Icosphere {
                                        radius: ball_radius,
                                        ..Default::default()
                                    }
                                    .into(),
                                ),
                            ));
                            ui_state_debug_physics.ball_meshes.last().unwrap().1.clone()
                        };

                        let material = ui_state_debug_physics
                            .materials
                            .choose(&mut rand::thread_rng())
                            .unwrap()
                            .clone();

                        let hit_position = ray_origin + ray_direction * distance;

                        let entity_id = commands
                            .spawn((
                                mesh,
                                material,
                                RigidBody::Dynamic,
                                Restitution::coefficient(ui_state_debug_physics.restitution),
                                Collider::ball(ui_state_debug_physics.ball_radius),
                                Transform::from_translation(Vec3::new(
                                    hit_position.x,
                                    hit_position.y + ui_state_debug_physics.spawn_height,
                                    hit_position.z,
                                )),
                                GlobalTransform::default(),
                                Visibility::default(),
                                ComputedVisibility::default(),
                                CollisionGroups::new(
                                    COLLISION_GROUP_PHYSICS_TOY,
                                    bevy_rapier3d::geometry::Group::all(),
                                ),
                            ))
                            .id();

                        let npc_entity = commands
                            .spawn((
                                Npc::new(NpcId::new(1).unwrap(), 0),
                                ColliderEntity::new(entity_id),
                                Transform::from_translation(Vec3::new(
                                    0.0,
                                    -ui_state_debug_physics.ball_radius / 2.0,
                                    0.0,
                                )),
                                GlobalTransform::default(),
                                Visibility::default(),
                                ComputedVisibility::default(),
                            ))
                            .id();

                        commands.entity(entity_id).add_child(npc_entity);

                        ui_state_debug_physics.spawn_timer -= ui_state_debug_physics.spawn_interval;
                    }
                }
            }
        }
    }

    if !key_code_input.pressed(KeyCode::B) {
        ui_state_debug_physics.spawn_timer = ui_state_debug_physics.spawn_interval;
    }
}
