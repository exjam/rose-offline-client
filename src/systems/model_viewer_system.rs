use std::cmp::Ordering;

use bevy::{
    hierarchy::DespawnRecursiveExt,
    math::Vec3,
    prelude::{
        Commands, Entity, GlobalTransform, PerspectiveCameraBundle, Query, Res, ResMut, Transform,
        With,
    },
    render::camera::Camera3d,
};
use bevy_egui::{egui, EguiContext};
use enum_map::{enum_map, EnumMap};
use rand::prelude::SliceRandom;

use rose_data::{
    CharacterMotionAction, EquipmentIndex, EquipmentItem, ItemReference, ItemType, NpcMotionAction,
    ZoneId,
};
use rose_game_common::components::{CharacterGender, CharacterInfo, Equipment, Npc};

use crate::{
    components::{
        ActiveMotion, CharacterModel, DebugRenderCollider, DebugRenderSkeleton, NpcModel,
    },
    fly_camera::{FlyCameraBundle, FlyCameraController},
    follow_camera::FollowCameraController,
    resources::{GameData, Icons},
};

pub struct ModelViewerState {
    debug_skeletons: bool,
    debug_colliders: bool,

    item_list_type: ItemType,
    valid_items: EnumMap<EquipmentIndex, Vec<ItemReference>>,

    npcs: Vec<Entity>,
    num_npcs: usize,
    max_num_npcs: usize,

    characters: Vec<Entity>,
    num_characters: usize,
    max_num_characters: usize,
}

#[allow(clippy::too_many_arguments)]
pub fn model_viewer_enter_system(
    mut commands: Commands,
    query_cameras: Query<Entity, With<Camera3d>>,
    game_data: Res<GameData>,
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
        debug_skeletons: false,
        debug_colliders: false,

        item_list_type: ItemType::Weapon,
        valid_items: enum_map! {
            equipment_index => get_valid_items(equipment_index.into()),
        },

        npcs: Vec::new(),
        num_npcs: 1,
        max_num_npcs: game_data.npcs.iter().count(),

        characters: Vec::new(),
        num_characters: 0,
        max_num_characters: 500,
    });
}

#[allow(clippy::too_many_arguments)]
pub fn model_viewer_system(
    mut commands: Commands,
    mut ui_state: ResMut<ModelViewerState>,
    mut query_character: Query<(Entity, &mut Equipment)>,
    mut query_npc: Query<(Entity, &mut Npc)>,
    query_character_model: Query<(Entity, &CharacterModel)>,
    query_npc_model: Query<(Entity, &NpcModel)>,
    query_debug_colliders: Query<Entity, With<DebugRenderCollider>>,
    query_debug_skeletons: Query<Entity, With<DebugRenderSkeleton>>,
    game_data: Res<GameData>,
    icons: Res<Icons>,
    mut egui_context: ResMut<EguiContext>,
) {
    egui::Window::new("Model Viewer").show(egui_context.ctx_mut(), |ui| {
        if ui
            .checkbox(&mut ui_state.debug_colliders, "Show Debug Colliders")
            .clicked()
        {
            if ui_state.debug_colliders {
                for (entity, _) in query_character.iter() {
                    commands
                        .entity(entity)
                        .insert(DebugRenderCollider::default());
                }

                for (entity, _) in query_npc.iter() {
                    commands
                        .entity(entity)
                        .insert(DebugRenderCollider::default());
                }
            } else {
                for entity in query_debug_colliders.iter() {
                    commands.entity(entity).remove::<DebugRenderCollider>();
                }
            }
        }

        if ui
            .checkbox(&mut ui_state.debug_skeletons, "Show Debug Skeletons")
            .clicked()
        {
            if ui_state.debug_skeletons {
                for (entity, _) in query_character.iter() {
                    commands
                        .entity(entity)
                        .insert(DebugRenderSkeleton::default());
                }

                for (entity, _) in query_npc.iter() {
                    commands
                        .entity(entity)
                        .insert(DebugRenderSkeleton::default());
                }
            } else {
                for entity in query_debug_skeletons.iter() {
                    commands.entity(entity).remove::<DebugRenderSkeleton>();
                }
            }
        }

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
                    let mut entity_commands = commands.spawn_bundle((
                        Npc::new(npc.id, 0),
                        GlobalTransform::default(),
                        Transform::default().with_translation(Vec3::new(
                            2.5 + (count / 30) as f32 * 5.0,
                            0.0,
                            (count % 30) as f32 * -5.0,
                        )),
                    ));

                    if ui_state.debug_colliders {
                        entity_commands.insert(DebugRenderCollider::default());
                    }

                    if ui_state.debug_skeletons {
                        entity_commands.insert(DebugRenderSkeleton::default());
                    }

                    ui_state.npcs.push(entity_commands.id());
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
                            equipment.equipped_items[equipment_index] = EquipmentItem::new(item);
                        }
                    }

                    // If has a two-handed weapon equipped, cannot have a sub weapon equipped
                    if let Some(equipped_weapon) =
                        equipment.equipped_items[EquipmentIndex::WeaponRight].as_ref()
                    {
                        if let Some(item_data) = game_data.items.get_base_item(equipped_weapon.item)
                        {
                            if item_data.class.is_two_handed_weapon()
                                && equipment.equipped_items[EquipmentIndex::WeaponLeft].is_some()
                            {
                                equipment.equipped_items[EquipmentIndex::WeaponLeft] = None;
                            }
                        }
                    }

                    let mut entity_commands = commands.spawn_bundle((
                        character_info,
                        equipment,
                        GlobalTransform::default(),
                        Transform::default().with_translation(Vec3::new(
                            -2.5 + (count / 25) as f32 * -5.0,
                            0.0,
                            (count % 25) as f32 * -5.0,
                        )),
                    ));

                    if ui_state.debug_colliders {
                        entity_commands.insert(DebugRenderCollider::default());
                    }

                    if ui_state.debug_skeletons {
                        entity_commands.insert(DebugRenderSkeleton::default());
                    }

                    ui_state.characters.push(entity_commands.id());
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
            "Attack",
            CharacterMotionAction::Attack,
            NpcMotionAction::Attack,
        );
        animation_button("Die", CharacterMotionAction::Die, NpcMotionAction::Die);
    });

    egui::Window::new("Item List")
        .vscroll(true)
        .resizable(true)
        .default_height(300.0)
        .show(egui_context.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut ui_state.item_list_type, ItemType::Face, "Face");
                ui.selectable_value(&mut ui_state.item_list_type, ItemType::Head, "Head");
                ui.selectable_value(&mut ui_state.item_list_type, ItemType::Body, "Body");
                ui.selectable_value(&mut ui_state.item_list_type, ItemType::Hands, "Hands");
                ui.selectable_value(&mut ui_state.item_list_type, ItemType::Feet, "Feet");
                ui.selectable_value(&mut ui_state.item_list_type, ItemType::Back, "Back");
                ui.selectable_value(&mut ui_state.item_list_type, ItemType::Weapon, "Weapon");
                ui.selectable_value(
                    &mut ui_state.item_list_type,
                    ItemType::SubWeapon,
                    "SubWeapon",
                );
            });
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut ui_state.item_list_type,
                    ItemType::Jewellery,
                    "Jewellery",
                );
                ui.selectable_value(
                    &mut ui_state.item_list_type,
                    ItemType::Consumable,
                    "Consumable",
                );
                ui.selectable_value(&mut ui_state.item_list_type, ItemType::Gem, "Gem");
                ui.selectable_value(&mut ui_state.item_list_type, ItemType::Material, "Material");
                ui.selectable_value(&mut ui_state.item_list_type, ItemType::Quest, "Quest");
                ui.selectable_value(&mut ui_state.item_list_type, ItemType::Vehicle, "Vehicle");
            });

            egui::Grid::new("item_list_grid")
                .num_columns(3)
                .show(ui, |ui| {
                    ui.label("icon");
                    ui.label("id");
                    ui.label("name");
                    ui.end_row();

                    let equipment_index = ui_state.item_list_type.try_into().ok();

                    if ui_state.item_list_type.is_equipment_item() {
                        ui.label(" ");
                        ui.label("0");
                        ui.label("None");
                        if ui.button("Equip").clicked() {
                            if let Some(equipment_index) = equipment_index {
                                for (_, mut equipment) in query_character.iter_mut() {
                                    equipment.equipped_items[equipment_index] = None;
                                }
                            }
                        }
                        ui.end_row();
                    }

                    for item_reference in game_data.items.iter_items(ui_state.item_list_type) {
                        if let Some(item_data) = game_data.items.get_base_item(item_reference) {
                            if !item_data.name.is_empty() {
                                if let Some((icon_texture_id, icon_uv)) =
                                    icons.get_item_icon(item_data.icon_index as usize)
                                {
                                    ui.add(
                                        egui::Image::new(icon_texture_id, [40.0, 40.0]).uv(icon_uv),
                                    );
                                } else {
                                    ui.label(" ");
                                }
                                ui.label(format!("{}", item_reference.item_number));
                                ui.label(&item_data.name);

                                if let Some(equipment_index) = equipment_index {
                                    if ui.button("Equip").clicked() {
                                        for (_, mut equipment) in query_character.iter_mut() {
                                            equipment.equipped_items[equipment_index] =
                                                Some(EquipmentItem::new(&item_reference).unwrap());

                                            if item_data.class.is_two_handed_weapon() {
                                                equipment.equipped_items
                                                    [EquipmentIndex::WeaponLeft] = None;
                                            }
                                        }
                                    }
                                }
                                ui.end_row();
                            }
                        }
                    }
                });
        });

    egui::Window::new("NPC List")
        .vscroll(true)
        .resizable(true)
        .default_height(300.0)
        .show(egui_context.ctx_mut(), |ui| {
            egui::Grid::new("npc_list_grid")
                .num_columns(3)
                .show(ui, |ui| {
                    ui.label("id");
                    ui.label("name");
                    ui.end_row();

                    for npc_data in game_data.npcs.iter() {
                        ui.label(format!("{}", npc_data.id.get()));
                        ui.label(&npc_data.name);
                        if ui.button("View").clicked() {
                            for (_, mut npc) in query_npc.iter_mut() {
                                npc.id = npc_data.id;
                            }
                        }
                        ui.end_row();
                    }
                });
        });
}
