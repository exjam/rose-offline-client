use bevy_egui::egui;

use rose_game_common::components::ItemSlot;

#[derive(Copy, Clone, Debug)]
pub enum DragAndDropId {
    Inventory(ItemSlot),
}

pub struct DragAndDropSlot<'a> {
    dnd_id: DragAndDropId,
    size: egui::Vec2,
    border_width: f32,
    contents: Option<(egui::TextureId, egui::Rect)>,
    accepts: fn(&DragAndDropId) -> bool,
    dragged_item: Option<&'a mut Option<DragAndDropId>>,
    dropped_item: Option<&'a mut Option<DragAndDropId>>,
}

impl<'a> DragAndDropSlot<'a> {
    pub fn new(
        dnd_id: DragAndDropId,
        contents: Option<(egui::TextureId, egui::Rect)>,
        accepts: fn(&DragAndDropId) -> bool,
        dragged_item: &'a mut Option<DragAndDropId>,
        dropped_item: &'a mut Option<DragAndDropId>,
        size: impl Into<egui::Vec2>,
    ) -> Self {
        Self {
            dnd_id,
            size: size.into(),
            border_width: 1.0,
            contents,
            accepts,
            dragged_item: Some(dragged_item),
            dropped_item: Some(dropped_item),
        }
    }
}

impl<'w> DragAndDropSlot<'w> {
    pub fn draw(&self, ui: &mut egui::Ui, accepts_dragged_item: bool) -> (bool, egui::Response) {
        let (rect, response) = ui.allocate_exact_size(
            self.size + egui::Vec2::splat(self.border_width * 2.0),
            if self.contents.is_some() {
                egui::Sense::click_and_drag()
            } else {
                egui::Sense::click()
            },
        );
        let mut dropped = false;

        if ui.is_rect_visible(rect) {
            use egui::epaint::*;

            // For some reason, we must do manual implementation of response.hovered
            let style = {
                let input = ui.ctx().input();
                let hovered = input
                    .pointer
                    .interact_pos()
                    .map_or(false, |cursor_pos| rect.contains(cursor_pos));

                if accepts_dragged_item && hovered {
                    if input.pointer.any_released()
                        && !input.pointer.button_down(egui::PointerButton::Primary)
                    {
                        dropped = true;
                    }
                    ui.visuals().widgets.active
                } else {
                    ui.visuals().widgets.inactive
                }
            };

            ui.painter().add(egui::Shape::Rect(egui::epaint::RectShape {
                rect,
                rounding: egui::Rounding::none(),
                fill: style.bg_fill,
                stroke: style.bg_stroke,
            }));

            if let Some((texture_id, uv)) = self.contents {
                let content_rect = egui::Rect::from_min_max(
                    rect.min + egui::Vec2::splat(self.border_width),
                    rect.max - egui::Vec2::splat(self.border_width),
                );
                let mut mesh = Mesh::with_texture(texture_id);
                mesh.add_rect_with_uv(content_rect, uv, egui::Color32::WHITE);
                ui.painter().add(Shape::mesh(mesh));

                if response.dragged_by(egui::PointerButton::Primary) {
                    if let Some(pointer_pos) = response.interact_pointer_pos() {
                        if !response.rect.contains(pointer_pos) {
                            let tooltip_painter = ui.ctx().layer_painter(egui::LayerId::new(
                                egui::Order::Tooltip,
                                egui::Id::new("dnd_tooltip"),
                            ));
                            let mut tooltip_mesh = egui::epaint::Mesh::with_texture(texture_id);
                            tooltip_mesh.add_rect_with_uv(
                                response
                                    .rect
                                    .translate(pointer_pos - response.rect.center()),
                                uv,
                                egui::Color32::WHITE,
                            );
                            tooltip_painter.add(egui::epaint::Shape::mesh(tooltip_mesh));
                        }
                    }
                }
            }
        }
        (dropped, response)
    }
}

impl<'w> egui::Widget for DragAndDropSlot<'w> {
    fn ui(mut self, ui: &mut egui::Ui) -> egui::Response {
        let dnd_id = self.dnd_id;
        let dragged_item = self.dragged_item.take().unwrap();
        let dropped_item = self.dropped_item.take().unwrap();
        let accepts_dragged_item = dragged_item
            .as_ref()
            .map(|dnd_id| (self.accepts)(dnd_id))
            .unwrap_or(false);

        let (dropped, mut response) = self.draw(ui, accepts_dragged_item);

        if response.dragged_by(egui::PointerButton::Primary) {
            *dragged_item = Some(dnd_id);
        } else if dropped {
            *dropped_item = Some(dragged_item.unwrap());
            response.mark_changed();
        }

        response
    }
}
