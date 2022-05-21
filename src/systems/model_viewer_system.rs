use std::{cmp::Ordering, path::Path};

use arrayvec::ArrayVec;
use bevy::{
    core::Time,
    hierarchy::{BuildChildren, DespawnRecursiveExt},
    math::{Quat, Vec2, Vec3},
    pbr::{AlphaMode, AmbientLight, StandardMaterial},
    prelude::{
        AssetServer, Assets, Color, Commands, Component, ComputedVisibility, Entity, EventWriter,
        GlobalTransform, Handle, Mesh, PerspectiveCameraBundle, Query, Res, ResMut, Transform,
        Visibility, With,
    },
    render::{camera::Camera3d, render_resource::Face},
};
use bevy_egui::{egui, EguiContext};
use bevy_rapier3d::prelude::{AsyncCollider, CollisionGroups};
use enum_map::{enum_map, EnumMap};
use rand::prelude::SliceRandom;

use rose_data::{
    CharacterMotionAction, EquipmentIndex, EquipmentItem, ItemReference, NpcMotionAction, ZoneId,
};
use rose_file_readers::{IfoObject, LitObject, ZscFile};
use rose_game_common::components::{CharacterGender, CharacterInfo, Equipment, Npc};

use crate::{
    components::{
        ActiveMotion, CharacterModel, ColliderParent, Effect, NpcModel,
        COLLISION_FILTER_INSPECTABLE, COLLISION_GROUP_ZONE_OBJECT,
    },
    events::{SpawnEffectData, SpawnEffectEvent},
    fly_camera::{FlyCameraBundle, FlyCameraController},
    follow_camera::FollowCameraController,
    render::{RgbTextureLoader, StaticMeshMaterial},
    resources::GameData,
    ui::UiStateDebugWindows,
    VfsResource,
};

#[derive(Component)]
pub struct PbrObjectTing;

pub struct ModelViewerState {
    valid_items: EnumMap<EquipmentIndex, Vec<ItemReference>>,

    npcs: Vec<Entity>,
    num_npcs: usize,
    max_num_npcs: usize,

    characters: Vec<Entity>,
    num_characters: usize,
    max_num_characters: usize,

    last_effect_entity: Option<Entity>,
}

fn load_block_object(
    commands: &mut Commands,
    asset_server: &AssetServer,
    vfs_resource: &VfsResource,
    standard_materials: &mut Assets<StandardMaterial>,
    zsc: &ZscFile,
    object_id: usize,
    object_transform: Transform,
) -> Entity {
    let object = &zsc.objects[object_id as usize];
    let mut material_cache: Vec<Option<Handle<StandardMaterial>>> = vec![None; zsc.materials.len()];
    let mut mesh_cache: Vec<Option<Handle<Mesh>>> = vec![None; zsc.meshes.len()];

    let mut part_entities: ArrayVec<Entity, 32> = ArrayVec::new();
    let mut object_entity_commands = commands.spawn_bundle((
        PbrObjectTing {},
        object_transform,
        GlobalTransform::default(),
    ));

    let object_entity = object_entity_commands.id();

    object_entity_commands.with_children(|object_commands| {
        for (part_index, object_part) in object.parts.iter().enumerate() {
            let part_transform = Transform::default()
                .with_translation(
                    Vec3::new(
                        object_part.position.x,
                        object_part.position.z,
                        -object_part.position.y,
                    ) / 100.0,
                )
                .with_rotation(Quat::from_xyzw(
                    object_part.rotation.x,
                    object_part.rotation.z,
                    -object_part.rotation.y,
                    object_part.rotation.w,
                ))
                .with_scale(Vec3::new(
                    object_part.scale.x,
                    object_part.scale.z,
                    object_part.scale.y,
                ));

            let mesh_id = object_part.mesh_id as usize;
            let mesh = mesh_cache[mesh_id].clone().unwrap_or_else(|| {
                let handle = asset_server.load(zsc.meshes[mesh_id].path());
                mesh_cache.insert(mesh_id, Some(handle.clone()));
                handle
            });

            let material_id = object_part.material_id as usize;
            let material = material_cache[material_id].clone().unwrap_or_else(|| {
                let zsc_material = &zsc.materials[material_id];
                let texture_path = zsc_material.path.path();
                let filename_tmp = texture_path.with_extension("");
                let filename = filename_tmp.file_name().unwrap().to_string_lossy();
                let handle = standard_materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(
                        asset_server.load(texture_path.with_file_name(format!("{}.dds", filename))),
                    ),
                    emissive: Color::BLACK,
                    emissive_texture: None,
                    perceptual_roughness: 1.0,
                    metallic: 1.0,
                    metallic_roughness_texture: Some(
                        asset_server.load(
                            texture_path.with_file_name(format!("{}_smoothness.dds", filename)),
                        ),
                    ),
                    reflectance: 0.05,
                    normal_map_texture: Some(
                        asset_server
                            .load(texture_path.with_file_name(format!("{}_normal.dds", filename))),
                    ),
                    flip_normal_map_y: false,
                    occlusion_texture: Some(
                        asset_server
                            .load(texture_path.with_file_name(format!("{}_ao.dds", filename))),
                    ),
                    double_sided: true,
                    cull_mode: None, // Some(Face::Back),
                    unlit: false,
                    alpha_mode: AlphaMode::Opaque,
                });
                material_cache.insert(material_id, Some(handle.clone()));
                handle
            });

            let mut part_commands = object_commands.spawn_bundle((
                mesh.clone(),
                material,
                part_transform,
                GlobalTransform::default(),
                Visibility::default(),
                ComputedVisibility::default(),
            ));

            part_commands.with_children(|builder| {
                // Transform for collider must be absolute
                let collider_transform = object_transform * part_transform;
                builder.spawn_bundle((
                    ColliderParent::new(object_entity),
                    AsyncCollider::Mesh(mesh),
                    CollisionGroups::new(COLLISION_GROUP_ZONE_OBJECT, COLLISION_FILTER_INSPECTABLE),
                    collider_transform,
                ));
            });

            let active_motion = object_part.animation_path.as_ref().map(|animation_path| {
                ActiveMotion::new_repeating(asset_server.load(animation_path.path()))
            });
            if let Some(active_motion) = active_motion {
                part_commands.insert(active_motion);
            }

            part_entities.push(part_commands.id());
        }
    });

    object_entity
}

#[allow(clippy::too_many_arguments)]
pub fn model_viewer_enter_system(
    mut commands: Commands,
    query_cameras: Query<Entity, With<Camera3d>>,
    game_data: Res<GameData>,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
    vfs_resource: Res<VfsResource>,
    asset_server: Res<AssetServer>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
) {
    // Reset camera
    for entity in query_cameras.iter() {
        commands
            .entity(entity)
            .remove::<FollowCameraController>()
            .remove::<ActiveMotion>()
            .insert_bundle(FlyCameraBundle::new(
                FlyCameraController::default(),
                PerspectiveCameraBundle::default(),
                Vec3::new(10.0, 10.0, 10.0),
                Vec3::new(0.0, 0.0, 0.0),
            ));
    }

    // Initialise state
    let get_valid_items = |item_type| -> Vec<ItemReference> {
        game_data
            .items
            .iter_items(item_type)
            .filter_map(|item| {
                game_data
                    .items
                    .get_base_item(item)
                    .map(|base_item| (item, base_item))
            })
            .filter(|(_, base_item)| !base_item.name.is_empty())
            .map(|(item, _)| item)
            .collect()
    };

    commands.insert_resource(ModelViewerState {
        valid_items: enum_map! {
            equipment_index => get_valid_items(equipment_index.into()),
        },

        npcs: Vec::new(),
        num_npcs: 0,
        max_num_npcs: game_data.npcs.iter().count(),

        characters: Vec::new(),
        num_characters: 10,
        max_num_characters: 500,

        last_effect_entity: None,
    });

    // Reset ambient light
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.1,
    });

    // Open relevant debug windows
    ui_state_debug_windows.debug_ui_open = true;
    ui_state_debug_windows.debug_render_open = false;
    ui_state_debug_windows.npc_list_open = false;
    ui_state_debug_windows.item_list_open = false;

    // Load model
    let zone_list_entry = game_data
        .zone_list
        .get_zone(ZoneId::new(2).unwrap())
        .unwrap();
    let zone_path = zone_list_entry
        .zon_file_path
        .path()
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let zsc_deco = vfs_resource
        .vfs
        .read_file::<ZscFile, _>(&zone_list_entry.zsc_deco_path)
        .unwrap();
    let object_entity = load_block_object(
        &mut commands,
        &asset_server,
        &vfs_resource,
        &mut standard_materials,
        &zsc_deco,
        175,
        Transform::default(),
    );
}

#[allow(clippy::too_many_arguments)]
pub fn model_viewer_system(
    mut commands: Commands,
    mut ui_state: ResMut<ModelViewerState>,
    mut spawn_effect_events: EventWriter<SpawnEffectEvent>,
    query_character_model: Query<(Entity, &CharacterModel)>,
    query_npc_model: Query<(Entity, &NpcModel)>,
    query_effects: Query<Entity, With<Effect>>,
    game_data: Res<GameData>,
    time: Res<Time>,
    query_pbr_ting: Query<Entity, With<PbrObjectTing>>,
    mut egui_context: ResMut<EguiContext>,
) {
    for entity in query_pbr_ting.iter() {
        commands
            .entity(entity)
            .insert(Transform::from_rotation(Quat::from_rotation_y(
                std::f32::consts::PI * time.seconds_since_startup() as f32 / 10.0,
            )));
    }

    egui::Window::new("Model Viewer").show(egui_context.ctx_mut(), |ui| {
        let max_num_npcs = ui_state.max_num_npcs;
        let max_num_characters = ui_state.max_num_characters;
        ui.add(egui::Slider::new(&mut ui_state.num_npcs, 0..=(max_num_npcs - 1)).suffix(" NPCs"));
        ui.add(
            egui::Slider::new(&mut ui_state.num_characters, 0..=(max_num_characters - 1))
                .suffix(" Characters"),
        );

        match ui_state.num_npcs.cmp(&ui_state.npcs.len()) {
            Ordering::Less => {
                // Delete some NPCs
                let num_npcs = ui_state.num_npcs;
                for entity in ui_state.npcs.split_off(num_npcs).iter() {
                    commands.entity(*entity).despawn_recursive();
                }
            }
            Ordering::Greater => {
                // Spawn some NPCs
                for (count, npc) in game_data
                    .npcs
                    .iter()
                    .enumerate()
                    .skip(ui_state.npcs.len())
                    .take(ui_state.num_npcs - ui_state.npcs.len())
                {
                    let entity = commands
                        .spawn_bundle((
                            Npc::new(npc.id, 0),
                            GlobalTransform::default(),
                            Transform::default().with_translation(Vec3::new(
                                2.5 + (count / 30) as f32 * 5.0,
                                0.0,
                                (count % 30) as f32 * -5.0,
                            )),
                        ))
                        .id();

                    ui_state.npcs.push(entity);
                }
            }
            Ordering::Equal => {}
        }

        match ui_state.num_characters.cmp(&ui_state.characters.len()) {
            Ordering::Less => {
                // Delete some characters
                let num_characters = ui_state.num_characters;
                for entity in ui_state.characters.split_off(num_characters).iter() {
                    commands.entity(*entity).despawn_recursive();
                }
            }
            Ordering::Greater => {
                let range = ui_state.characters.len()..ui_state.num_characters;
                for count in range {
                    let mut rng = rand::thread_rng();
                    let genders = [CharacterGender::Male, CharacterGender::Female];
                    let faces = [1u8, 8, 15, 22, 29, 36, 43];
                    let hair = [0u8, 5, 10, 15, 20];

                    let character_info = CharacterInfo {
                        name: format!("Bot {}", count),
                        gender: *genders.choose(&mut rng).unwrap(),
                        race: 0,
                        face: *faces.choose(&mut rng).unwrap(),
                        hair: *hair.choose(&mut rng).unwrap(),
                        birth_stone: 0,
                        job: 0,
                        rank: 0,
                        fame: 0,
                        fame_b: 0,
                        fame_g: 0,
                        revive_zone_id: ZoneId::new(22).unwrap(),
                        revive_position: Vec3::new(5200.0, 1.7, -5200.0),
                        unique_id: 0,
                    };

                    let mut equipment = Equipment::default();
                    for (equipment_index, valid_items) in ui_state.valid_items.iter() {
                        if let Some(item) = valid_items.choose(&mut rng) {
                            equipment.equipped_items[equipment_index] =
                                EquipmentItem::new(*item, 0);
                        }
                    }

                    // If has a two-handed weapon equipped, cannot have a sub weapon equipped
                    if let Some(equipped_weapon) =
                        equipment.equipped_items[EquipmentIndex::Weapon].as_ref()
                    {
                        if let Some(item_data) = game_data.items.get_base_item(equipped_weapon.item)
                        {
                            if item_data.class.is_two_handed_weapon()
                                && equipment.equipped_items[EquipmentIndex::SubWeapon].is_some()
                            {
                                equipment.equipped_items[EquipmentIndex::SubWeapon] = None;
                            }
                        }
                    }

                    let entity = commands
                        .spawn_bundle((
                            character_info,
                            equipment,
                            GlobalTransform::default(),
                            Transform::default().with_translation(Vec3::new(
                                2.5 + (count / 5) as f32 * -5.0,
                                3.0,
                                5.0 + (count % 5) as f32 * -5.0,
                            )),
                        ))
                        .id();

                    ui_state.characters.push(entity);
                }
            }
            Ordering::Equal => {}
        }
    });

    egui::Window::new("Animation").show(egui_context.ctx_mut(), |ui| {
        let mut animation_button =
            |name: &str, character_action: CharacterMotionAction, npc_action: NpcMotionAction| {
                if ui.button(name).clicked() {
                    for (entity, character_model) in query_character_model.iter() {
                        commands.entity(entity).insert(ActiveMotion::new_repeating(
                            character_model.action_motions[character_action].clone(),
                        ));
                    }

                    for (entity, npc_model) in query_npc_model.iter() {
                        commands.entity(entity).insert(ActiveMotion::new_repeating(
                            npc_model.action_motions[npc_action].clone(),
                        ));
                    }
                }
            };

        animation_button("Stop", CharacterMotionAction::Stop1, NpcMotionAction::Stop);
        animation_button("Walk", CharacterMotionAction::Walk, NpcMotionAction::Move);
        animation_button("Run", CharacterMotionAction::Run, NpcMotionAction::Run);
        animation_button(
            "Attack 1",
            CharacterMotionAction::Attack,
            NpcMotionAction::Attack,
        );
        animation_button(
            "Attack 2",
            CharacterMotionAction::Attack2,
            NpcMotionAction::Attack,
        );
        animation_button(
            "Attack 3",
            CharacterMotionAction::Attack3,
            NpcMotionAction::Attack,
        );
        animation_button("Die", CharacterMotionAction::Die, NpcMotionAction::Die);
    });

    egui::Window::new("Effect List")
        .vscroll(true)
        .resizable(true)
        .default_height(300.0)
        .show(egui_context.ctx_mut(), |ui| {
            egui_extras::TableBuilder::new(ui)
                .striped(true)
                .cell_layout(egui::Layout::left_to_right().with_cross_align(egui::Align::Center))
                .column(egui_extras::Size::initial(50.0).at_least(50.0))
                .column(egui_extras::Size::remainder().at_least(50.0))
                .column(egui_extras::Size::initial(60.0).at_least(60.0))
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.heading("ID");
                    });
                    header.col(|ui| {
                        ui.heading("Path");
                    });
                    header.col(|ui| {
                        ui.heading("Action");
                    });
                })
                .body(|mut body| {
                    for (effect_file_id, effect_file_path) in game_data.effect_database.iter_files()
                    {
                        body.row(20.0, |mut row| {
                            row.col(|ui| {
                                ui.label(format!("{}", effect_file_id.get()));
                            });

                            row.col(|ui| {
                                ui.label(effect_file_path.path().to_string_lossy().as_ref());
                            });

                            row.col(|ui| {
                                if ui.button("View").clicked() {
                                    if let Some(last_effect_entity) =
                                        ui_state.last_effect_entity.take()
                                    {
                                        if query_effects.get(last_effect_entity).is_ok() {
                                            commands.entity(last_effect_entity).despawn_recursive();
                                        }
                                    }

                                    let effect_entity = commands
                                        .spawn_bundle((
                                            Transform::default(),
                                            GlobalTransform::default(),
                                            Visibility::default(),
                                            ComputedVisibility::default(),
                                        ))
                                        .id();

                                    spawn_effect_events.send(SpawnEffectEvent::InEntity(
                                        effect_entity,
                                        SpawnEffectData::with_path(effect_file_path.clone())
                                            .manual_despawn(true),
                                    ));

                                    ui_state.last_effect_entity = Some(effect_entity);
                                }
                            });
                        });
                    }
                });
        });
}
