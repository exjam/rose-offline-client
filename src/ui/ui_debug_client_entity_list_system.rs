use bevy::prelude::{Local, Query, Res, ResMut};
use bevy_egui::{egui, EguiContexts};
use rose_game_common::{
    components::{CharacterInfo, DroppedItem, ItemDrop, Npc},
    messages::ClientEntityId,
};

use crate::{
    components::{ClientEntity, ClientEntityType, Command, PlayerCharacter},
    resources::{ClientEntityList, GameData},
    ui::UiStateDebugWindows,
};

pub struct UiStateDebugClientEntityList {
    filter_characters: bool,
    filter_npcs: bool,
    filter_monsters: bool,
    filter_item_drops: bool,
    num_characters: usize,
    num_npcs: usize,
    num_monsters: usize,
    num_item_drops: usize,
}

impl Default for UiStateDebugClientEntityList {
    fn default() -> Self {
        Self {
            filter_characters: true,
            filter_monsters: true,
            filter_npcs: true,
            filter_item_drops: true,
            num_characters: 0,
            num_npcs: 0,
            num_monsters: 0,
            num_item_drops: 0,
        }
    }
}

pub fn ui_debug_client_entity_list_system(
    mut egui_context: EguiContexts,
    mut ui_state_debug_client_entity_list: Local<UiStateDebugClientEntityList>,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
    client_entity_list: Res<ClientEntityList>,
    game_data: Res<GameData>,
    query_client_entity: Query<(
        &ClientEntity,
        Option<&Command>,
        Option<&CharacterInfo>,
        Option<&ItemDrop>,
        Option<&Npc>,
        Option<&PlayerCharacter>,
    )>,
) {
    if !ui_state_debug_windows.debug_ui_open {
        return;
    }

    egui::Window::new("Client Entity List")
        .resizable(true)
        .default_height(400.0)
        .open(&mut ui_state_debug_windows.client_entity_list_open)
        .show(egui_context.ctx_mut(), |ui| {
            egui::Grid::new("client_entity_list_filter")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Filter");
                    ui.label("Count");
                    ui.end_row();

                    ui.checkbox(
                        &mut ui_state_debug_client_entity_list.filter_characters,
                        "Characters",
                    );
                    ui.label(format!(
                        "{}",
                        ui_state_debug_client_entity_list.num_characters
                    ));
                    ui.end_row();

                    ui.checkbox(
                        &mut ui_state_debug_client_entity_list.filter_monsters,
                        "Monsters",
                    );
                    ui.label(format!(
                        "{}",
                        ui_state_debug_client_entity_list.num_monsters
                    ));
                    ui.end_row();

                    ui.checkbox(&mut ui_state_debug_client_entity_list.filter_npcs, "NPCs");
                    ui.label(format!("{}", ui_state_debug_client_entity_list.num_npcs));
                    ui.end_row();

                    ui.checkbox(
                        &mut ui_state_debug_client_entity_list.filter_item_drops,
                        "Item Drops",
                    );
                    ui.label(format!(
                        "{}",
                        ui_state_debug_client_entity_list.num_item_drops
                    ));
                    ui.end_row();
                });

            ui.separator();

            ui_state_debug_client_entity_list.num_characters = 0;
            ui_state_debug_client_entity_list.num_monsters = 0;
            ui_state_debug_client_entity_list.num_npcs = 0;
            ui_state_debug_client_entity_list.num_item_drops = 0;

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                .show(ui, |ui| {
                    egui::Grid::new("client_entity_list_grid")
                        .num_columns(4)
                        .striped(true)
                        .show(ui, |ui| {
                            for (client_entity_id, entity) in client_entity_list
                                .client_entities
                                .iter()
                                .enumerate()
                                .filter(|(_, e)| e.is_some())
                                .map(|(id, entity)| (ClientEntityId(id), entity.unwrap()))
                            {
                                if let Ok((
                                    client_entity,
                                    command,
                                    character_info,
                                    item_drop,
                                    npc,
                                    player_character,
                                )) = query_client_entity.get(entity)
                                {
                                    match client_entity.entity_type {
                                        ClientEntityType::Character => {
                                            ui_state_debug_client_entity_list.num_characters += 1;
                                            if !ui_state_debug_client_entity_list.filter_characters
                                            {
                                                continue;
                                            }
                                        }
                                        ClientEntityType::Monster => {
                                            ui_state_debug_client_entity_list.num_monsters += 1;
                                            if !ui_state_debug_client_entity_list.filter_monsters {
                                                continue;
                                            }
                                        }
                                        ClientEntityType::Npc => {
                                            ui_state_debug_client_entity_list.num_npcs += 1;
                                            if !ui_state_debug_client_entity_list.filter_npcs {
                                                continue;
                                            }
                                        }
                                        ClientEntityType::ItemDrop => {
                                            ui_state_debug_client_entity_list.num_item_drops += 1;
                                            if !ui_state_debug_client_entity_list.filter_item_drops
                                            {
                                                continue;
                                            }
                                        }
                                    }

                                    ui.label(format!("{}", client_entity_id.0));

                                    if let Some(character_info) = character_info {
                                        if player_character.is_some() {
                                            ui.label("Player");
                                        } else {
                                            ui.label("Character");
                                        }
                                        ui.label(&character_info.name);
                                    } else if let Some(npc) = npc {
                                        if client_entity.entity_type == ClientEntityType::Monster {
                                            ui.label("Monster");
                                        } else {
                                            ui.label("NPC");
                                        }

                                        if let Some(npc_data) = game_data.npcs.get_npc(npc.id) {
                                            ui.label(npc_data.name);
                                        } else {
                                            ui.label(format!("? [{}]", npc.id.get()));
                                        }
                                    } else if let Some(item_drop) = item_drop {
                                        ui.label("Item Drop");
                                        match item_drop.item.as_ref() {
                                            Some(DroppedItem::Money(money)) => {
                                                ui.label(format!("Money: {}", money.0))
                                            }
                                            Some(DroppedItem::Item(item)) => ui.label(format!(
                                                "Item: {:?} {}",
                                                item.get_item_type(),
                                                item.get_item_number()
                                            )),
                                            None => ui.label("?"),
                                        };
                                    } else {
                                        ui.label("Unknown");
                                        ui.label(" ");
                                    }

                                    if let Some(command) = command {
                                        match command {
                                            Command::Stop => ui.label("Idle"),
                                            Command::Move(_) => ui.label("Moving"),
                                            Command::Attack(_) => ui.label("Attacking"),
                                            Command::Die => ui.label("Dead"),
                                            Command::PersonalStore => ui.label("Personal Store"),
                                            Command::PickupItem(_) => ui.label("Pickup Item"),
                                            Command::Emote(_) => ui.label("Emote"),
                                            Command::Sit(_) => ui.label("Sitting"),
                                            Command::CastSkill(_) => ui.label("Casting Skill"),
                                        };
                                    } else {
                                        ui.label(" ");
                                    }

                                    ui.end_row();
                                }
                            }
                        });
                });
        });
}
