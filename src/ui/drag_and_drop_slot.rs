use bevy_egui::egui;

use rose_data::Item;
use rose_game_common::components::{Equipment, Inventory, ItemSlot};

use crate::{
    resources::{GameData, Icons},
    ui::UiStateDragAndDrop,
};

#[derive(Copy, Clone, Debug)]
pub enum DragAndDropId {
    Inventory(ItemSlot),
}

pub struct DragAndDropSlot<'w> {
    dnd_id: DragAndDropId,
    size: egui::Vec2,
    id: egui::Id,
    game_data: &'w GameData,
    icons: &'w Icons,
    equipment: &'w Equipment,
    inventory: &'w Inventory,
    ui_state_dnd: Option<&'w mut UiStateDragAndDrop>,
}

impl<'w> DragAndDropSlot<'w> {
    pub fn new(
        dnd_id: DragAndDropId,
        game_data: &'w GameData,
        icons: &'w Icons,
        equipment: &'w Equipment,
        inventory: &'w Inventory,
        ui_state_dnd: &'w mut UiStateDragAndDrop,
        size: impl Into<egui::Vec2>,
    ) -> Self {
        let id = match dnd_id {
            DragAndDropId::Inventory(slot) => egui::Id::new("slot_inventory").with(slot),
        };

        Self {
            dnd_id,
            size: size.into(),
            id,
            game_data,
            icons,
            equipment,
            inventory,
            ui_state_dnd: Some(ui_state_dnd),
        }
    }
}

impl<'w> DragAndDropSlot<'w> {
    fn get_item(&self) -> Option<Item> {
        match self.dnd_id {
            DragAndDropId::Inventory(ItemSlot::Equipment(equipment_index)) => self
                .equipment
                .get_equipment_item(equipment_index)
                .cloned()
                .map(Item::Equipment),
            DragAndDropId::Inventory(ItemSlot::Ammo(ammo_index)) => self
                .equipment
                .get_ammo_item(ammo_index)
                .cloned()
                .map(Item::Stackable),
            DragAndDropId::Inventory(ItemSlot::Vehicle(vehicle_part_index)) => self
                .equipment
                .get_vehicle_item(vehicle_part_index)
                .cloned()
                .map(Item::Equipment),
            DragAndDropId::Inventory(item_slot) => self.inventory.get_item(item_slot).cloned(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self.dnd_id {
            DragAndDropId::Inventory(ItemSlot::Equipment(equipment_index)) => {
                self.equipment.get_equipment_item(equipment_index).is_none()
            }
            DragAndDropId::Inventory(item_slot) => self.inventory.get_item(item_slot).is_none(),
        }
    }

    pub fn slot_ui(&self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(self.size, egui::Sense::hover());
        if ui.is_rect_visible(rect) {
            use egui::epaint::*;
            match self.dnd_id {
                DragAndDropId::Inventory(_) => {
                    if let Some(item) = self.get_item() {
                        let item_data = self
                            .game_data
                            .items
                            .get_base_item(item.get_item_reference());

                        if let Some((texture_id, uv)) = self.icons.get_item_icon(
                            item_data
                                .map(|item_data| item_data.icon_index as usize)
                                .unwrap_or(0),
                        ) {
                            let mut mesh = Mesh::with_texture(texture_id);
                            mesh.add_rect_with_uv(rect, uv, egui::Color32::WHITE);
                            ui.painter().add(Shape::mesh(mesh));

                            if let Some(item_data) = item_data {
                                response.on_hover_text(format!(
                                    "{}\nItem Type: {:?}, Item Id: {}",
                                    item_data.name,
                                    item.get_item_type(),
                                    item.get_item_number()
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
}

impl<'w> egui::Widget for DragAndDropSlot<'w> {
    fn ui(mut self, ui: &mut egui::Ui) -> egui::Response {
        let can_accept_what_is_being_dragged = true; // TODO
        let slot_id = self.id;
        let dnd_id = self.dnd_id;
        let drag_slot = self.ui_state_dnd.take().unwrap();

        let response = drop_target(ui, can_accept_what_is_being_dragged, |ui| {
            if self.is_empty() {
                self.slot_ui(ui);
            } else {
                drag_source(ui, self.id, |ui| {
                    self.slot_ui(ui);
                });
            }
        })
        .response;

        if ui.memory().is_being_dragged(slot_id) {
            drag_slot.source = Some(dnd_id);
        }

        let can_accept_what_is_being_dragged = true;
        if ui.memory().is_anything_being_dragged()
            && can_accept_what_is_being_dragged
            && response.hovered()
        {
            drag_slot.destination = Some(dnd_id);
        }

        response
    }
}

fn drag_source(ui: &mut egui::Ui, id: egui::Id, body: impl FnOnce(&mut egui::Ui)) {
    let is_being_dragged = ui.memory().is_being_dragged(id);

    if !is_being_dragged {
        let response = ui.scope(body).response;

        // Check for drags:
        let response = ui.interact(response.rect, id, egui::Sense::drag());
        if response.hovered() {
            ui.output().cursor_icon = egui::CursorIcon::Grab;
        }
    } else {
        ui.output().cursor_icon = egui::CursorIcon::Grabbing;

        // Paint the body to a new layer:
        let layer_id = egui::LayerId::new(egui::Order::Tooltip, id);
        let response = ui.with_layer_id(layer_id, body).response;

        // Now we move the visuals of the body to where the mouse is.
        // Normally you need to decide a location for a widget first,
        // because otherwise that widget cannot interact with the mouse.
        // However, a dragged component cannot be interacted with anyway
        // (anything with `Order::Tooltip` always gets an empty `Response`)
        // So this is fine!

        if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
            let delta = pointer_pos - response.rect.center();
            ui.ctx().translate_layer(layer_id, delta);
        }
    }
}

fn drop_target<R>(
    ui: &mut egui::Ui,
    can_accept_what_is_being_dragged: bool,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let is_being_dragged = ui.memory().is_anything_being_dragged();

    let margin = egui::Vec2::splat(1.0);

    let outer_rect_bounds = ui.available_rect_before_wrap();
    let inner_rect = outer_rect_bounds.shrink2(margin);
    let where_to_put_background = ui.painter().add(egui::Shape::Noop);
    let mut content_ui = ui.child_ui(inner_rect, *ui.layout());
    let ret = body(&mut content_ui);
    let outer_rect =
        egui::Rect::from_min_max(outer_rect_bounds.min, content_ui.min_rect().max + margin);
    let (rect, response) = ui.allocate_at_least(outer_rect.size(), egui::Sense::hover());

    let style = if is_being_dragged && can_accept_what_is_being_dragged && response.hovered() {
        ui.visuals().widgets.active
    } else {
        ui.visuals().widgets.inactive
    };

    let mut fill = style.bg_fill;
    let mut stroke = style.bg_stroke;
    stroke.width = 1.0;
    if is_being_dragged && !can_accept_what_is_being_dragged {
        // gray out:
        fill = egui::color::tint_color_towards(fill, ui.visuals().window_fill());
        stroke.color = egui::color::tint_color_towards(stroke.color, ui.visuals().window_fill());
    }

    ui.painter().set(
        where_to_put_background,
        egui::epaint::RectShape {
            rounding: egui::Rounding::none(),
            fill,
            stroke,
            rect,
        },
    );

    egui::InnerResponse::new(ret, response)
}
