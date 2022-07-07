use bevy::{
    input::Input,
    prelude::{EventWriter, KeyCode, Local, Mut, Query, Res, ResMut, With},
};
use bevy_egui::{egui, EguiContext};

use rose_data::{Item, SkillCooldown};
use rose_game_common::components::{
    Equipment, Hotbar, HotbarSlot, Inventory, SkillList, HOTBAR_NUM_PAGES, HOTBAR_PAGE_SIZE,
};

use crate::{
    components::{ConsumableCooldownGroup, Cooldowns, PlayerCharacter},
    events::PlayerCommandEvent,
    resources::{GameData, Icons},
    ui::{
        ui_add_item_tooltip, ui_add_skill_tooltip, ui_inventory_system::GetItem, DragAndDropId,
        DragAndDropSlot, UiStateDragAndDrop,
    },
};

#[derive(Default)]
pub struct UiStateHotBar {
    current_page: usize,
}

fn hotbar_drag_accepts(drag_source: &DragAndDropId) -> bool {
    matches!(
        drag_source,
        DragAndDropId::Inventory(_) | DragAndDropId::Skill(_) | DragAndDropId::Hotbar(_, _)
    )
}

fn ui_add_hotbar_slot(
    ui: &mut egui::Ui,
    hotbar_index: (usize, usize),
    player_hotbar: &mut Mut<Hotbar>,
    player_cooldowns: &Cooldowns,
    player_equipment: &Equipment,
    player_inventory: &Inventory,
    player_skill_list: &SkillList,
    game_data: &GameData,
    icons: &Icons,
    ui_state_dnd: &mut UiStateDragAndDrop,
    use_slot: bool,
    player_command_events: &mut EventWriter<PlayerCommandEvent>,
) {
    let hotbar_slot = player_hotbar.pages[hotbar_index.0][hotbar_index.1].as_ref();
    let (contents, quantity, cooldown_percent) = match hotbar_slot {
        Some(HotbarSlot::Skill(skill_slot)) => {
            let skill = player_skill_list.get_skill(*skill_slot);
            let skill_data = skill
                .as_ref()
                .and_then(|skill| game_data.skills.get_skill(*skill));
            (
                skill_data
                    .and_then(|skill_data| icons.get_skill_icon(skill_data.icon_number as usize)),
                None,
                skill_data.and_then(|skill_data| match &skill_data.cooldown {
                    SkillCooldown::Skill(_) => {
                        player_cooldowns.get_skill_cooldown_percent(skill_data.id)
                    }
                    SkillCooldown::Group(group, _) => {
                        player_cooldowns.get_skill_group_cooldown_percent(*group)
                    }
                }),
            )
        }
        Some(HotbarSlot::Inventory(item_slot)) => {
            let item = (player_equipment, player_inventory).get_item(*item_slot);
            let item_data = item
                .as_ref()
                .and_then(|item| game_data.items.get_base_item(item.get_item_reference()));
            (
                item_data.and_then(|item_data| icons.get_item_icon(item_data.icon_index as usize)),
                match item.as_ref() {
                    Some(Item::Stackable(stackable_item)) => Some(stackable_item.quantity as usize),
                    _ => None,
                },
                item.as_ref()
                    .and_then(|item| {
                        ConsumableCooldownGroup::from_item(&item.get_item_reference(), game_data)
                    })
                    .and_then(|group| player_cooldowns.get_consumable_cooldown_percent(group)),
            )
        }
        _ => (None, None, None),
    };

    let mut dropped_item = None;
    let response = ui.add(DragAndDropSlot::new(
        DragAndDropId::Hotbar(hotbar_index.0, hotbar_index.1),
        contents,
        quantity,
        cooldown_percent,
        hotbar_drag_accepts,
        &mut ui_state_dnd.dragged_item,
        &mut dropped_item,
        [40.0, 40.0],
    ));

    if use_slot || response.double_clicked() {
        player_command_events.send(PlayerCommandEvent::UseHotbar(
            hotbar_index.0,
            hotbar_index.1,
        ));
    }

    response.on_hover_ui(|ui| match hotbar_slot {
        Some(HotbarSlot::Inventory(item_slot)) => {
            if let Some(item) = (player_equipment, player_inventory).get_item(*item_slot) {
                ui_add_item_tooltip(ui, game_data, &item);
            }
        }
        Some(HotbarSlot::Skill(skill_slot)) => {
            if let Some(skill) = player_skill_list.get_skill(*skill_slot) {
                ui_add_skill_tooltip(ui, false, game_data, skill);
            }
        }
        _ => {}
    });

    match dropped_item {
        Some(DragAndDropId::Hotbar(page, index)) => {
            if page != hotbar_index.0 || index != hotbar_index.1 {
                let slot_a = player_hotbar.pages[hotbar_index.0][hotbar_index.1].take();
                let slot_b = player_hotbar.pages[page][index].take();

                player_command_events.send(PlayerCommandEvent::SetHotbar(page, index, slot_a));
                player_command_events.send(PlayerCommandEvent::SetHotbar(
                    hotbar_index.0,
                    hotbar_index.1,
                    slot_b,
                ));
            }
        }
        Some(DragAndDropId::Inventory(item_slot)) => {
            player_command_events.send(PlayerCommandEvent::SetHotbar(
                hotbar_index.0,
                hotbar_index.1,
                Some(HotbarSlot::Inventory(item_slot)),
            ));
        }
        Some(DragAndDropId::Skill(skill_slot)) => {
            player_command_events.send(PlayerCommandEvent::SetHotbar(
                hotbar_index.0,
                hotbar_index.1,
                Some(HotbarSlot::Skill(skill_slot)),
            ));
        }
        _ => {}
    }
}

pub fn ui_hotbar_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_hot_bar: Local<UiStateHotBar>,
    mut ui_state_dnd: ResMut<UiStateDragAndDrop>,
    mut query_player: Query<
        (&mut Hotbar, &Cooldowns, &Equipment, &Inventory, &SkillList),
        With<PlayerCharacter>,
    >,
    mut player_command_events: EventWriter<PlayerCommandEvent>,
    keyboard_input: Res<Input<KeyCode>>,
    game_data: Res<GameData>,
    icons: Res<Icons>,
) {
    let (
        mut player_hotbar,
        player_cooldowns,
        player_equipment,
        player_inventory,
        player_skill_list,
    ) = query_player.single_mut();

    let use_hotbar_index = if !egui_context.ctx_mut().wants_keyboard_input() {
        if keyboard_input.just_pressed(KeyCode::F1) {
            Some(0)
        } else if keyboard_input.just_pressed(KeyCode::F2) {
            Some(1)
        } else if keyboard_input.just_pressed(KeyCode::F3) {
            Some(2)
        } else if keyboard_input.just_pressed(KeyCode::F4) {
            Some(3)
        } else if keyboard_input.just_pressed(KeyCode::F5) {
            Some(4)
        } else if keyboard_input.just_pressed(KeyCode::F6) {
            Some(5)
        } else if keyboard_input.just_pressed(KeyCode::F7) {
            Some(6)
        } else if keyboard_input.just_pressed(KeyCode::F8) {
            Some(7)
        } else {
            None
        }
    } else {
        None
    };

    egui::Window::new("Hot Bar")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -10.0])
        .show(egui_context.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                egui::Grid::new("my_grid")
                    .spacing([4.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        let current_page = ui_state_hot_bar.current_page;

                        for i in 0..HOTBAR_PAGE_SIZE {
                            let hotbar_index = (current_page, i);
                            ui_add_hotbar_slot(
                                ui,
                                hotbar_index,
                                &mut player_hotbar,
                                player_cooldowns,
                                player_equipment,
                                player_inventory,
                                player_skill_list,
                                &game_data,
                                &icons,
                                &mut ui_state_dnd,
                                use_hotbar_index.map_or(false, |use_index| use_index == i),
                                &mut player_command_events,
                            );
                        }
                        ui.end_row();

                        for i in 1..=HOTBAR_PAGE_SIZE {
                            ui.vertical_centered_justified(|ui| {
                                ui.label(format!("{}", i));
                            });
                        }
                        ui.end_row();
                    });

                ui.vertical_centered(|ui| {
                    if ui.button("⬆").clicked() {
                        if ui_state_hot_bar.current_page == 0 {
                            ui_state_hot_bar.current_page = HOTBAR_NUM_PAGES - 1;
                        } else {
                            ui_state_hot_bar.current_page =
                                (ui_state_hot_bar.current_page - 1) % HOTBAR_NUM_PAGES;
                        }
                    }

                    ui.label(format!("{}", ui_state_hot_bar.current_page));

                    if ui.button("⬇").clicked() {
                        ui_state_hot_bar.current_page =
                            (ui_state_hot_bar.current_page + 1) % HOTBAR_NUM_PAGES;
                    }
                });
            });
        });
}
