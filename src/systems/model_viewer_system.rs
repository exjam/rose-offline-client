use std::cmp::Ordering;

use bevy::{
    hierarchy::DespawnRecursiveExt,
    math::Vec3,
    pbr::AmbientLight,
    prelude::{
        Camera3d, Color, Commands, ComputedVisibility, Entity, EventWriter, GlobalTransform, Query,
        Res, ResMut, Transform, Visibility, With,
    },
};
use bevy_egui::{egui, EguiContext};
use enum_map::{enum_map, EnumMap};
use rand::prelude::SliceRandom;

use rose_data::{
    CharacterMotionAction, EquipmentIndex, EquipmentItem, ItemReference, NpcMotionAction, ZoneId,
};
use rose_game_common::components::{CharacterGender, CharacterInfo, Equipment, Npc};

use crate::{
    components::{ActiveMotion, CharacterModel, Effect, NpcModel},
    events::{SpawnEffectData, SpawnEffectEvent},
    free_camera::FreeCamera,
    orbit_camera::OrbitCamera,
    resources::GameData,
    ui::UiStateDebugWindows,
};

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

#[allow(clippy::too_many_arguments)]
pub fn model_viewer_enter_system(
    mut commands: Commands,
    query_cameras: Query<Entity, With<Camera3d>>,
    game_data: Res<GameData>,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
) {
    // Reset camera
    for entity in query_cameras.iter() {
        commands
            .entity(entity)
            .remove::<FreeCamera>()
            .remove::<OrbitCamera>()
            .remove::<ActiveMotion>()
            .insert(FreeCamera::new(Vec3::new(0.0, 10.0, 15.0), 0.0, -20.0));
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
        num_npcs: 1,
        max_num_npcs: game_data.npcs.iter().count(),

        characters: Vec::new(),
        num_characters: 0,
        max_num_characters: 500,

        last_effect_entity: None,
    });

    // Reset ambient light
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 1.0,
    });

    // Open relevant debug windows
    ui_state_debug_windows.debug_ui_open = true;
    ui_state_debug_windows.debug_render_open = true;
    ui_state_debug_windows.npc_list_open = true;
    ui_state_debug_windows.item_list_open = true;
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
    mut egui_context: ResMut<EguiContext>,
) {
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
                                -2.5 + (count / 25) as f32 * -5.0,
                                0.0,
                                (count % 25) as f32 * -5.0,
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
