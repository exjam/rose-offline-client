use bevy::{
    math::Vec3,
    prelude::{
        Commands, Entity, GlobalTransform, Local, PerspectiveCameraBundle, Query, Res, ResMut,
        Transform, With,
    },
    render::camera::Camera3d,
};
use bevy_egui::{egui, EguiContext};

use rose_data::{EquipmentIndex, EquipmentItem, ItemReference, ItemType, NpcId, ZoneId};
use rose_game_common::components::{CharacterGender, CharacterInfo, Equipment, Npc};

use crate::{
    components::DebugModelSkeleton,
    fly_camera::FlyCameraController,
    follow_camera::{FollowCameraBundle, FollowCameraController},
    resources::GameData,
};

pub struct ModelViewerUiState {
    item_list_open: bool,
    npc_list_open: bool,
}

impl Default for ModelViewerUiState {
    fn default() -> Self {
        Self {
            item_list_open: true,
            npc_list_open: true,
        }
    }
}

pub struct ModelViewerUiItemListState {
    item_list_type: ItemType,
}

impl Default for ModelViewerUiItemListState {
    fn default() -> Self {
        Self {
            item_list_type: ItemType::Face,
        }
    }
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
            .remove::<FlyCameraController>()
            .insert_bundle(FollowCameraBundle::new(
                FollowCameraController::default(),
                PerspectiveCameraBundle::default(),
                Vec3::new(10.0, 10.0, 10.0),
                Vec3::new(0.0, 0.0, 0.0),
            ));
    }

    // Spawn our character model
    let character_info = CharacterInfo {
        name: "Bot 1".into(),
        gender: CharacterGender::Male,
        race: 0,
        face: 8,
        hair: 10,
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
    equipment
        .equip_item(EquipmentItem::new(&ItemReference::new(ItemType::Weapon, 1)).unwrap())
        .ok();
    equipment
        .equip_item(EquipmentItem::new(&ItemReference::new(ItemType::SubWeapon, 1)).unwrap())
        .ok();

    commands.spawn_bundle((
        character_info,
        equipment,
        GlobalTransform::default(),
        Transform::default().with_translation(Vec3::new(-2.0, 0.0, 0.0)),
        DebugModelSkeleton::default(),
    ));

    // Spawn our NPC model
    if false {
        for (count, npc) in game_data.npcs.iter().enumerate() {
            commands.spawn_bundle((
                Npc::new(npc.id, 0),
                GlobalTransform::default(),
                Transform::default().with_translation(Vec3::new(
                    (count / 30) as f32 * 5.0,
                    0.0,
                    (count % 30) as f32 * 5.0,
                )),
            ));
        }
    } else {
        commands.spawn_bundle((
            Npc::new(NpcId::new(1).unwrap(), 0),
            GlobalTransform::default(),
            Transform::default().with_translation(Vec3::new(-2.0, 0.0, 0.0)),
            DebugModelSkeleton::default(),
        ));
    }
}

#[allow(clippy::too_many_arguments)]
pub fn model_viewer_system(
    mut ui_state: Local<ModelViewerUiState>,
    mut ui_item_list_state: Local<ModelViewerUiItemListState>,
    mut query_character: Query<&mut Equipment>,
    mut query_npc: Query<&mut Npc>,
    game_data: Res<GameData>,
    mut egui_context: ResMut<EguiContext>,
) {
    egui::Window::new("Item List")
        .vscroll(true)
        .resizable(true)
        .default_height(300.0)
        .open(&mut ui_state.item_list_open)
        .show(egui_context.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut ui_item_list_state.item_list_type,
                    ItemType::Face,
                    "Face",
                );
                ui.selectable_value(
                    &mut ui_item_list_state.item_list_type,
                    ItemType::Head,
                    "Head",
                );
                ui.selectable_value(
                    &mut ui_item_list_state.item_list_type,
                    ItemType::Body,
                    "Body",
                );
                ui.selectable_value(
                    &mut ui_item_list_state.item_list_type,
                    ItemType::Hands,
                    "Hands",
                );
                ui.selectable_value(
                    &mut ui_item_list_state.item_list_type,
                    ItemType::Feet,
                    "Feet",
                );
                ui.selectable_value(
                    &mut ui_item_list_state.item_list_type,
                    ItemType::Back,
                    "Back",
                );
                ui.selectable_value(
                    &mut ui_item_list_state.item_list_type,
                    ItemType::Weapon,
                    "Weapon",
                );
                ui.selectable_value(
                    &mut ui_item_list_state.item_list_type,
                    ItemType::SubWeapon,
                    "SubWeapon",
                );
            });
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut ui_item_list_state.item_list_type,
                    ItemType::Jewellery,
                    "Jewellery",
                );
                ui.selectable_value(
                    &mut ui_item_list_state.item_list_type,
                    ItemType::Consumable,
                    "Consumable",
                );
                ui.selectable_value(&mut ui_item_list_state.item_list_type, ItemType::Gem, "Gem");
                ui.selectable_value(
                    &mut ui_item_list_state.item_list_type,
                    ItemType::Material,
                    "Material",
                );
                ui.selectable_value(
                    &mut ui_item_list_state.item_list_type,
                    ItemType::Quest,
                    "Quest",
                );
                ui.selectable_value(
                    &mut ui_item_list_state.item_list_type,
                    ItemType::Vehicle,
                    "Vehicle",
                );
            });

            egui::Grid::new("item_list_grid").show(ui, |ui| {
                ui.label("id");
                ui.label("name");
                ui.end_row();

                for item_reference in game_data
                    .items
                    .iter_items(ui_item_list_state.item_list_type)
                {
                    if let Some(item_data) = game_data.items.get_base_item(item_reference) {
                        if !item_data.name.is_empty() {
                            ui.label(format!("{}", item_reference.item_number));
                            ui.label(&item_data.name);

                            if item_reference.item_type.is_equipment_item()
                                && ui.button("Equip").clicked()
                            {
                                for mut equipment in query_character.iter_mut() {
                                    match item_reference.item_type {
                                        ItemType::Face => {
                                            equipment.equipped_items[EquipmentIndex::Face] =
                                                Some(EquipmentItem::new(&item_reference).unwrap())
                                        }
                                        ItemType::Head => {
                                            equipment.equipped_items[EquipmentIndex::Head] =
                                                Some(EquipmentItem::new(&item_reference).unwrap())
                                        }
                                        ItemType::Body => {
                                            equipment.equipped_items[EquipmentIndex::Body] =
                                                Some(EquipmentItem::new(&item_reference).unwrap())
                                        }
                                        ItemType::Hands => {
                                            equipment.equipped_items[EquipmentIndex::Hands] =
                                                Some(EquipmentItem::new(&item_reference).unwrap())
                                        }
                                        ItemType::Feet => {
                                            equipment.equipped_items[EquipmentIndex::Feet] =
                                                Some(EquipmentItem::new(&item_reference).unwrap())
                                        }
                                        ItemType::Back => {
                                            equipment.equipped_items[EquipmentIndex::Back] =
                                                Some(EquipmentItem::new(&item_reference).unwrap())
                                        }
                                        ItemType::Weapon => {
                                            equipment.equipped_items[EquipmentIndex::WeaponRight] =
                                                Some(EquipmentItem::new(&item_reference).unwrap())
                                        }
                                        ItemType::SubWeapon => {
                                            equipment.equipped_items[EquipmentIndex::WeaponLeft] =
                                                Some(EquipmentItem::new(&item_reference).unwrap())
                                        }
                                        _ => {}
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
        .open(&mut ui_state.npc_list_open)
        .show(egui_context.ctx_mut(), |ui| {
            egui::Grid::new("npc_list_grid").show(ui, |ui| {
                ui.label("id");
                ui.label("name");
                ui.end_row();

                for npc_data in game_data.npcs.iter() {
                    ui.label(format!("{}", npc_data.id.get()));
                    ui.label(&npc_data.name);
                    if ui.button("View").clicked() {
                        for mut npc in query_npc.iter_mut() {
                            npc.id = npc_data.id;
                        }
                    }
                    ui.end_row();
                }
            });
        });
}
