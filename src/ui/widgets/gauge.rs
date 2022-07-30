use bevy_egui::egui;
use serde::Deserialize;

use crate::resources::{UiResources, UiSprite};

use super::{DataBindings, DrawWidget, LoadWidget};

#[derive(Clone, Default, Deserialize)]
#[serde(rename = "GUAGE")] // Intentionally incorrect spelling
#[serde(default)]
pub struct Gauge {
    #[serde(rename = "ID")]
    pub id: i32,
    #[serde(rename = "NAME")]
    pub name: String,
    #[serde(rename = "X")]
    pub x: f32,
    #[serde(rename = "Y")]
    pub y: f32,
    #[serde(rename = "OFFSETX")]
    pub offset_x: f32,
    #[serde(rename = "OFFSETY")]
    pub offset_y: f32,
    #[serde(rename = "WIDTH")]
    pub width: f32,
    #[serde(rename = "HEIGHT")]
    pub height: f32,
    #[serde(rename = "MODULEID")]
    pub module_id: i32,
    #[serde(rename = "GID")]
    pub foreground_sprite_name: String,
    #[serde(rename = "BGID")]
    pub background_sprite_name: String,

    #[serde(skip)]
    pub foreground_sprite: Option<UiSprite>,
    #[serde(skip)]
    pub background_sprite: Option<UiSprite>,
}

widget_to_rect! { Gauge }

impl LoadWidget for Gauge {
    fn load_widget(&mut self, ui_resources: &UiResources) {
        self.foreground_sprite =
            ui_resources.get_sprite(self.module_id, &self.foreground_sprite_name);
        self.background_sprite =
            ui_resources.get_sprite(self.module_id, &self.background_sprite_name);
    }
}

impl DrawWidget for Gauge {
    fn draw_widget(&self, ui: &mut egui::Ui, bindings: &mut DataBindings) {
        if !bindings.get_visible(self.id) {
            return;
        }

        let (value, text) = bindings
            .gauge
            .iter()
            .find(|(id, _, _)| *id == self.id)
            .map_or((0.5, ""), |(_, value, text)| (**value, &**text));

        let rect = self.widget_rect(ui.min_rect().min);
        let response = ui.allocate_rect(rect, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            if let Some(sprite) = self.background_sprite.as_ref() {
                sprite.draw_stretched(ui, rect);
            }

            if value * self.width > 0.5 {
                if let Some(sprite) = self.foreground_sprite.as_ref() {
                    let mut stretched_rect = rect;
                    stretched_rect.set_width(value * self.width);
                    sprite.draw_stretched(ui, stretched_rect);
                }
            }

            if !text.is_empty() {
                ui.put(
                    rect.translate(egui::vec2(1.0, 1.0)),
                    egui::Label::new(egui::RichText::new(text).color(egui::Color32::BLACK)),
                );

                ui.put(rect, egui::Label::new(text));
            }
        }

        bindings.set_response(self.id, response);
    }
}
