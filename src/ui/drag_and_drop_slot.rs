use bevy_egui::egui;

use rose_game_common::components::{ItemSlot, SkillSlot};

use crate::resources::UiSprite;

#[derive(Copy, Clone, Debug)]
pub enum DragAndDropId {
    NotDraggable,
    Inventory(ItemSlot),
    Skill(SkillSlot),
    Hotbar(usize, usize),
    NpcStore(usize, usize),
    NpcStoreBuyList(usize),
    NpcStoreSellList(usize),
}

pub struct DragAndDropSlot<'a> {
    dnd_id: DragAndDropId,
    size: egui::Vec2,
    border_width: f32,
    sprite: Option<UiSprite>,
    cooldown_percent: Option<f32>,
    quantity: Option<usize>,
    quantity_margin: f32,
    accepts: fn(&DragAndDropId) -> bool,
    dragged_item: Option<&'a mut Option<DragAndDropId>>,
    dropped_item: Option<&'a mut Option<DragAndDropId>>,
}

impl<'a> DragAndDropSlot<'a> {
    pub fn new(
        dnd_id: DragAndDropId,
        sprite: Option<UiSprite>,
        quantity: Option<usize>,
        cooldown_percent: Option<f32>,
        accepts: fn(&DragAndDropId) -> bool,
        dragged_item: &'a mut Option<DragAndDropId>,
        dropped_item: &'a mut Option<DragAndDropId>,
        size: impl Into<egui::Vec2>,
    ) -> Self {
        Self {
            dnd_id,
            size: size.into(),
            border_width: 1.0,
            sprite,
            cooldown_percent,
            quantity,
            quantity_margin: 2.0,
            accepts,
            dragged_item: Some(dragged_item),
            dropped_item: Some(dropped_item),
        }
    }
}

fn generate_cooldown_mesh(cooldown: f32, content_rect: egui::Rect) -> egui::epaint::Mesh {
    use egui::epaint::*;

    let segment_size = Vec2::new(content_rect.width() / 2.0, content_rect.height() / 2.0);
    let mut mesh = Mesh::default();

    let add_vert = |mesh: &mut Mesh, x, y| {
        let pos = mesh.vertices.len();
        mesh.vertices.push(Vertex {
            pos: Pos2::new(x, y),
            uv: WHITE_UV,
            color: Color32::from_rgba_unmultiplied(50, 50, 50, 150),
        });
        pos as u32
    };

    /*
     * 2 1+9 8
     * 3  0  7
     * 4  5  6
     */
    add_vert(
        &mut mesh,
        content_rect.min.x + segment_size.x,
        content_rect.min.y + segment_size.y,
    );
    add_vert(
        &mut mesh,
        content_rect.min.x + segment_size.x,
        content_rect.min.y,
    );
    add_vert(&mut mesh, content_rect.min.x, content_rect.min.y);
    add_vert(
        &mut mesh,
        content_rect.min.x,
        content_rect.min.y + segment_size.y,
    );
    add_vert(&mut mesh, content_rect.min.x, content_rect.max.y);
    add_vert(
        &mut mesh,
        content_rect.min.x + segment_size.x,
        content_rect.max.y,
    );
    add_vert(&mut mesh, content_rect.max.x, content_rect.max.y);
    add_vert(
        &mut mesh,
        content_rect.max.x,
        content_rect.min.y + segment_size.y,
    );
    add_vert(&mut mesh, content_rect.max.x, content_rect.min.y);
    add_vert(
        &mut mesh,
        content_rect.min.x + segment_size.x,
        content_rect.min.y,
    );

    /*
     * Triangles:
     * _______
     * |\ | /|
     * |_\|/_|
     * | /|\ |
     * |/ | \|
     * -------
     */
    const TRIANGLES_COUNT: f32 = 8.0;
    let segments = cooldown * TRIANGLES_COUNT;
    let num_segments = segments.trunc() as u32;
    for segment_id in 0..num_segments {
        mesh.add_triangle(0, segment_id + 1, segment_id + 2);
    }

    let fract_segments = segments.fract();
    if fract_segments > 0.0 {
        if let (Some(vert_1), Some(vert_2)) = (
            mesh.vertices.get(num_segments as usize + 1).map(|x| x.pos),
            mesh.vertices.get(num_segments as usize + 2).map(|x| x.pos),
        ) {
            let vertex_id = add_vert(
                &mut mesh,
                (vert_2.x - vert_1.x) * fract_segments + vert_1.x,
                (vert_2.y - vert_1.y) * fract_segments + vert_1.y,
            );
            mesh.add_triangle(0, num_segments + 1, vertex_id);
        }
    }

    mesh
}

impl<'w> DragAndDropSlot<'w> {
    pub fn draw(&self, ui: &mut egui::Ui, accepts_dragged_item: bool) -> (bool, egui::Response) {
        let (rect, response) = ui.allocate_exact_size(
            self.size,
            if self.sprite.is_some() && !matches!(self.dnd_id, DragAndDropId::NotDraggable) {
                egui::Sense::click_and_drag()
            } else {
                egui::Sense::click()
            },
        );
        let mut dropped = false;

        if ui.is_rect_visible(rect) {
            use egui::epaint::*;

            // For some reason, we must do manual implementation of response.hovered
            let is_active = {
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
                    true
                } else {
                    false
                }
            };

            if let Some(sprite) = self.sprite.as_ref() {
                let content_rect = rect;
                let mut mesh = Mesh::with_texture(sprite.texture_id);
                mesh.add_rect_with_uv(content_rect, sprite.uv, egui::Color32::WHITE);
                ui.painter().add(Shape::mesh(mesh));

                if let Some(cooldown_percent) = self.cooldown_percent {
                    ui.painter().add(Shape::mesh(generate_cooldown_mesh(
                        cooldown_percent,
                        content_rect,
                    )));
                }

                if let Some(quantity) = self.quantity {
                    let text_galley = ui.fonts().layout_no_wrap(
                        format!("{}", quantity),
                        FontId::monospace(12.0),
                        Color32::WHITE,
                    );

                    ui.painter().add(egui::Shape::Rect(egui::epaint::RectShape {
                        rect: Rect::from_min_max(
                            egui::Pos2::new(
                                content_rect.max.x
                                    - text_galley.rect.right()
                                    - self.quantity_margin,
                                content_rect.min.y,
                            ),
                            egui::Pos2::new(
                                content_rect.max.x,
                                content_rect.min.y
                                    + self.quantity_margin * 2.0
                                    + text_galley.rect.height(),
                            ),
                        ),
                        rounding: egui::Rounding::none(),
                        fill: Color32::from_rgba_unmultiplied(50, 50, 50, 200),
                        stroke: Stroke::none(),
                    }));

                    ui.painter().add(Shape::galley(
                        egui::Pos2::new(
                            content_rect.max.x - text_galley.rect.right(),
                            content_rect.min.y + self.quantity_margin,
                        ),
                        text_galley,
                    ));
                }

                if response.dragged_by(egui::PointerButton::Primary) {
                    if let Some(pointer_pos) = response.interact_pointer_pos() {
                        if !response.rect.contains(pointer_pos) {
                            let tooltip_painter = ui.ctx().layer_painter(egui::LayerId::new(
                                egui::Order::Tooltip,
                                egui::Id::new("dnd_tooltip"),
                            ));
                            let mut tooltip_mesh =
                                egui::epaint::Mesh::with_texture(sprite.texture_id);
                            tooltip_mesh.add_rect_with_uv(
                                response
                                    .rect
                                    .translate(pointer_pos - response.rect.center()),
                                sprite.uv,
                                egui::Color32::WHITE,
                            );
                            tooltip_painter.add(egui::epaint::Shape::mesh(tooltip_mesh));
                        }
                    }
                }
            }

            if is_active {
                ui.painter().add(egui::Shape::Rect(egui::epaint::RectShape {
                    rect: rect.shrink(self.border_width),
                    rounding: egui::Rounding::none(),
                    fill: Default::default(),
                    stroke: egui::Stroke {
                        width: self.border_width,
                        color: egui::Color32::YELLOW,
                    },
                }));
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
