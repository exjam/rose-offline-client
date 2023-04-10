use std::sync::Arc;

use bevy::prelude::{Camera, Camera3d, GlobalTransform, Local, Query, Res, Vec2, With};
use bevy_egui::{egui, EguiContexts};

use rose_data::Item;
use rose_game_common::components::{DroppedItem, ItemDrop};

use crate::{resources::GameData, ui::get_item_name_color};

pub struct ItemDropName {
    screen_z: f32,
    pos: egui::Pos2,
    galley: Arc<egui::Galley>,
    colour: egui::Color32,
}

pub fn ui_item_drop_name_system(
    mut egui_context: EguiContexts,
    query_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    query_item_drop: Query<(&ItemDrop, &GlobalTransform)>,
    game_data: Res<GameData>,
    mut visible_names: Local<Vec<ItemDropName>>,
) {
    let ctx = egui_context.ctx_mut();
    let style = ctx.style();
    let screen_size = ctx.input(|input| input.screen_rect().size());
    let tooltip_painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Background,
        egui::Id::new("item_drop_tooltips"),
    ));
    let (camera, camera_transform) = query_camera.single();

    visible_names.clear();
    visible_names.reserve(32);

    for (item_drop, global_transform) in query_item_drop.iter() {
        let Some(dropped_item) = &item_drop.item else {
            continue;
        };
        let Some(ndc_space_coords) = camera.world_to_ndc(camera_transform, global_transform.translation()) else { continue; };
        if ndc_space_coords.z < 0.0 || ndc_space_coords.z > 1.0 {
            // Outside near / far plane
            continue;
        }

        let screen_pos = (ndc_space_coords.truncate() + Vec2::ONE) / 2.0
            * Vec2::new(screen_size.x, screen_size.y);
        let screen_z = ndc_space_coords.z;

        let (text, colour) = match dropped_item {
            DroppedItem::Item(item) => {
                let item_data = game_data
                    .items
                    .get_base_item(item.get_item_reference())
                    .unwrap();

                match item {
                    Item::Equipment(_) => (
                        item_data.name.to_string(),
                        get_item_name_color(item.get_item_type(), item_data),
                    ),
                    Item::Stackable(stackable_item) => (
                        format!("{} [{}]", item_data.name, stackable_item.quantity),
                        egui::Color32::YELLOW,
                    ),
                }
            }
            DroppedItem::Money(money) => (format!("{} Zuly", money.0), egui::Color32::YELLOW),
        };

        let galley = ctx.fonts(|fonts| {
            fonts.layout_no_wrap(text, egui::FontSelection::Default.resolve(&style), colour)
        });
        let pos = egui::pos2(
            screen_pos.x - galley.rect.width() / 2.0,
            screen_size.y - screen_pos.y,
        );
        visible_names.push(ItemDropName {
            screen_z,
            pos,
            galley,
            colour,
        });
    }

    // Sort by distance to camera
    visible_names.sort_by(|a, b| a.screen_z.partial_cmp(&b.screen_z).unwrap());

    for visible_name in visible_names.drain(..) {
        tooltip_painter.add(egui::epaint::RectShape {
            rect: visible_name
                .galley
                .rect
                .translate(egui::vec2(visible_name.pos.x, visible_name.pos.y))
                .expand(2.0),
            rounding: egui::Rounding::none(),
            fill: style.visuals.window_fill,
            stroke: style.visuals.window_stroke,
        });
        tooltip_painter.add(egui::epaint::TextShape {
            pos: visible_name.pos,
            galley: visible_name.galley,
            underline: egui::Stroke::NONE,
            override_text_color: Some(visible_name.colour),
            angle: 0.0,
        });
    }
}
