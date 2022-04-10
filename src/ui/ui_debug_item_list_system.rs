use bevy::prelude::{Local, Query, Res, ResMut, State};
use bevy_egui::{egui, EguiContext};
use rose_data::{EquipmentIndex, EquipmentItem, Item, ItemType};
use rose_data_irose::encode_item_type;
use rose_game_common::{components::Equipment, messages::client::ClientMessage};

use crate::{
    resources::{AppState, GameConnection, GameData, Icons},
    ui::{UiStateDebugWindows, ui_add_item_tooltip},
};

pub struct UiStateDebugItemList {
    item_list_type: ItemType,
    item_name_filter: String,
}

impl Default for UiStateDebugItemList {
    fn default() -> Self {
        Self {
            item_list_type: ItemType::Face,
            item_name_filter: String::new(),
        }
    }
}

pub fn ui_debug_item_list_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_debug_item_list: Local<UiStateDebugItemList>,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
    mut query_equipment: Query<&mut Equipment>,
    app_state: Res<State<AppState>>,
    game_connection: Option<Res<GameConnection>>,
    game_data: Res<GameData>,
    icons: Res<Icons>,
) {
    if !ui_state_debug_windows.debug_ui_open {
        return;
    }

    egui::Window::new("Item List")
        .resizable(true)
        .default_height(300.0)
        .open(&mut ui_state_debug_windows.item_list_open)
        .show(egui_context.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                ui.label("Item Name Filter: ");
                ui.text_edit_singleline(&mut ui_state_debug_item_list.item_name_filter);
            });

            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut ui_state_debug_item_list.item_list_type,
                    ItemType::Face,
                    "Face",
                );
                ui.selectable_value(
                    &mut ui_state_debug_item_list.item_list_type,
                    ItemType::Head,
                    "Head",
                );
                ui.selectable_value(
                    &mut ui_state_debug_item_list.item_list_type,
                    ItemType::Body,
                    "Body",
                );
                ui.selectable_value(
                    &mut ui_state_debug_item_list.item_list_type,
                    ItemType::Hands,
                    "Hands",
                );
                ui.selectable_value(
                    &mut ui_state_debug_item_list.item_list_type,
                    ItemType::Feet,
                    "Feet",
                );
                ui.selectable_value(
                    &mut ui_state_debug_item_list.item_list_type,
                    ItemType::Back,
                    "Back",
                );
                ui.selectable_value(
                    &mut ui_state_debug_item_list.item_list_type,
                    ItemType::Weapon,
                    "Weapon",
                );
                ui.selectable_value(
                    &mut ui_state_debug_item_list.item_list_type,
                    ItemType::SubWeapon,
                    "SubWeapon",
                );
            });

            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut ui_state_debug_item_list.item_list_type,
                    ItemType::Jewellery,
                    "Jewellery",
                );
                ui.selectable_value(
                    &mut ui_state_debug_item_list.item_list_type,
                    ItemType::Consumable,
                    "Consumable",
                );
                ui.selectable_value(
                    &mut ui_state_debug_item_list.item_list_type,
                    ItemType::Gem,
                    "Gem",
                );
                ui.selectable_value(
                    &mut ui_state_debug_item_list.item_list_type,
                    ItemType::Material,
                    "Material",
                );
                ui.selectable_value(
                    &mut ui_state_debug_item_list.item_list_type,
                    ItemType::Quest,
                    "Quest",
                );
                ui.selectable_value(
                    &mut ui_state_debug_item_list.item_list_type,
                    ItemType::Vehicle,
                    "Vehicle",
                );
            });

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .always_show_scroll(true)
                .show(ui, |ui| {
                    egui::Grid::new("item_list_grid")
                        .num_columns(3)
                        .min_row_height(45.0)
                        .striped(true)
                        .show(ui, |ui| {
                            let equipment_index: Option<EquipmentIndex> =
                                ui_state_debug_item_list.item_list_type.try_into().ok();

                            if ui_state_debug_item_list.item_list_type.is_equipment_item() {
                                ui.label(" ");
                                ui.label("0");
                                ui.label("None");

                                if matches!(app_state.current(), AppState::ModelViewer)
                                    && ui.button("Equip").clicked()
                                {
                                    if let Some(equipment_index) = equipment_index {
                                        for mut equipment in query_equipment.iter_mut() {
                                            equipment.equipped_items[equipment_index] = None;
                                        }
                                    }
                                }

                                ui.end_row();
                            }

                            for (item_reference, item_data) in game_data
                                .items
                                .iter_items(ui_state_debug_item_list.item_list_type)
                                .filter_map(|item_reference| {
                                    game_data
                                        .items
                                        .get_base_item(item_reference)
                                        .map(|item_data| (item_reference, item_data))
                                })
                                .filter(|(_, item_data)| {
                                    if ui_state_debug_item_list.item_name_filter.is_empty() {
                                        true
                                    } else {
                                        item_data
                                            .name
                                            .contains(&ui_state_debug_item_list.item_name_filter)
                                    }
                                })
                            {
                                if !item_data.name.is_empty() {
                                    if let Some((icon_texture_id, icon_uv)) =
                                        icons.get_item_icon(item_data.icon_index as usize)
                                    {
                                        ui.add(
                                            egui::Image::new(icon_texture_id, [40.0, 40.0])
                                                .uv(icon_uv),
                                        )
                                        .on_hover_ui(
                                            |ui| {
                                                if let Some(item) = Item::new(&item_reference, 1) {
                                                    ui_add_item_tooltip(ui, &game_data, &item);
                                                }
                                            },
                                        );
                                    } else {
                                        ui.label(" ");
                                    }
                                    ui.label(format!("{}", item_reference.item_number));
                                    ui.label(&item_data.name);

                                    match app_state.current() {
                                        AppState::Game => {
                                            if ui.button("Spawn").clicked() {
                                                if let Some(game_connection) =
                                                    game_connection.as_ref()
                                                {
                                                    if let Some(item_type) = encode_item_type(
                                                        ui_state_debug_item_list.item_list_type,
                                                    ) {
                                                        game_connection
                                                            .client_message_tx
                                                            .send(ClientMessage::Chat(format!(
                                                                "/item {} {}",
                                                                item_type,
                                                                item_reference.item_number,
                                                            )))
                                                            .ok();
                                                    }
                                                }
                                            }
                                        }
                                        AppState::ModelViewer => {
                                            if let Some(equipment_index) = equipment_index {
                                                if ui.button("Equip").clicked() {
                                                    for mut equipment in query_equipment.iter_mut()
                                                    {
                                                        equipment.equipped_items[equipment_index] =
                                                            Some(
                                                                EquipmentItem::new(&item_reference)
                                                                    .unwrap(),
                                                            );

                                                        if item_data.class.is_two_handed_weapon() {
                                                            equipment.equipped_items
                                                                [EquipmentIndex::WeaponLeft] = None;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        _ => {}
                                    }

                                    ui.end_row();
                                }
                            }
                        });
                });
        });
}
