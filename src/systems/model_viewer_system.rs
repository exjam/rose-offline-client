use bevy::{
    math::Vec3,
    pbr::{AlphaMode, StandardMaterial},
    prelude::{
        shape, AssetServer, Assets, BuildChildren, Camera, Color, Commands, Entity,
        GlobalTransform, Local, Mesh, PerspectiveCameraBundle, PerspectiveProjection, Query, Res,
        ResMut, Transform, With,
    },
};
use bevy_egui::{egui, EguiContext};
use nalgebra::Point3;

use rose_data::{EquipmentIndex, EquipmentItem, ItemDatabase, ItemReference, ItemType, ZoneId};
use rose_game_common::components::{CharacterGender, CharacterInfo, Equipment};

use crate::{
    character_model::{spawn_character_model, CharacterModelList},
    follow_camera::{FollowCameraBundle, FollowCameraController},
    render::StaticMeshMaterial,
};

pub struct ModelViewerUiState {
    item_list_open: bool,
}

impl Default for ModelViewerUiState {
    fn default() -> Self {
        Self {
            item_list_open: true,
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

pub fn model_viewer_enter_system(
    mut commands: Commands,
    query_cameras: Query<Entity, (With<Camera>, With<PerspectiveProjection>)>,
) {
    // Remove any other cameras
    for entity in query_cameras.iter() {
        commands.entity(entity).despawn();
    }

    commands.spawn_bundle(FollowCameraBundle::new(
        FollowCameraController::default(),
        PerspectiveCameraBundle::default(),
        Vec3::new(10.0, 10.0, 10.0),
        Vec3::new(0.0, 0.0, 0.0),
    ));
}

#[allow(clippy::too_many_arguments)]
pub fn model_viewer_system(
    mut commands: Commands,
    mut ui_state: Local<ModelViewerUiState>,
    mut ui_item_list_state: Local<ModelViewerUiItemListState>,
    mut query_character: Query<(&mut Equipment,)>,
    asset_server: Res<AssetServer>,
    character_model_list: Res<CharacterModelList>,
    item_database: Res<ItemDatabase>,
    mut egui_context: ResMut<EguiContext>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut static_mesh_materials: ResMut<Assets<StaticMeshMaterial>>,
) {
    if query_character.is_empty() {
        // Create a character
        let bone_mesh = meshes.add(Mesh::from(shape::Cube { size: 0.1 }));
        let bone_material = materials.add(StandardMaterial {
            base_color: Color::rgba(1.0, 0.08, 0.58, 0.75),
            alpha_mode: AlphaMode::Blend,
            ..Default::default()
        });

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
            revive_position: Point3::new(5200.0, 0.0, -5200.0),
            unique_id: 0,
        };
        let mut equipment = Equipment::default();
        equipment
            .equip_item(EquipmentItem::new(&ItemReference::new(ItemType::Weapon, 1)).unwrap())
            .ok();
        equipment
            .equip_item(EquipmentItem::new(&ItemReference::new(ItemType::SubWeapon, 1)).unwrap())
            .ok();

        let character_model = spawn_character_model(
            &mut commands,
            &asset_server,
            &mut static_mesh_materials,
            &character_model_list,
            &character_info,
            &equipment,
            Some((bone_mesh, bone_material)),
        );
        let root_bone = character_model.skeleton.root;
        commands
            .spawn_bundle((
                character_info,
                equipment,
                character_model,
                GlobalTransform::default(),
                Transform::default(),
            ))
            .add_child(root_bone);
    } else {
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
                    ui.selectable_value(
                        &mut ui_item_list_state.item_list_type,
                        ItemType::Gem,
                        "Gem",
                    );
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

                    for item_reference in
                        item_database.iter_items(ui_item_list_state.item_list_type)
                    {
                        if let Some(item_data) = item_database.get_base_item(item_reference) {
                            if !item_data.name.is_empty() {
                                ui.label(format!("{}", item_reference.item_number));
                                ui.label(&item_data.name);

                                if item_reference.item_type.is_equipment_item()
                                    && ui.button("Equip").clicked()
                                {
                                    for (mut equipment,) in query_character.iter_mut() {
                                        match item_reference.item_type {
                                            ItemType::Face => {
                                                equipment.equipped_items[EquipmentIndex::Face] =
                                                    Some(
                                                        EquipmentItem::new(&item_reference)
                                                            .unwrap(),
                                                    )
                                            }
                                            ItemType::Head => {
                                                equipment.equipped_items[EquipmentIndex::Head] =
                                                    Some(
                                                        EquipmentItem::new(&item_reference)
                                                            .unwrap(),
                                                    )
                                            }
                                            ItemType::Body => {
                                                equipment.equipped_items[EquipmentIndex::Body] =
                                                    Some(
                                                        EquipmentItem::new(&item_reference)
                                                            .unwrap(),
                                                    )
                                            }
                                            ItemType::Hands => {
                                                equipment.equipped_items[EquipmentIndex::Hands] =
                                                    Some(
                                                        EquipmentItem::new(&item_reference)
                                                            .unwrap(),
                                                    )
                                            }
                                            ItemType::Feet => {
                                                equipment.equipped_items[EquipmentIndex::Feet] =
                                                    Some(
                                                        EquipmentItem::new(&item_reference)
                                                            .unwrap(),
                                                    )
                                            }
                                            ItemType::Back => {
                                                equipment.equipped_items[EquipmentIndex::Back] =
                                                    Some(
                                                        EquipmentItem::new(&item_reference)
                                                            .unwrap(),
                                                    )
                                            }
                                            ItemType::Weapon => {
                                                equipment.equipped_items
                                                    [EquipmentIndex::WeaponRight] = Some(
                                                    EquipmentItem::new(&item_reference).unwrap(),
                                                )
                                            }
                                            ItemType::SubWeapon => {
                                                equipment.equipped_items
                                                    [EquipmentIndex::WeaponLeft] = Some(
                                                    EquipmentItem::new(&item_reference).unwrap(),
                                                )
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
    }
}
