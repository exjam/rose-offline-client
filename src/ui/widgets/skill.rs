use bevy_egui::egui;
use serde::Deserialize;

use crate::resources::{UiResources, UiTexture};

use super::{DataBindings, DrawWidget, LoadWidget, Widget};

#[derive(Clone, Default, Deserialize)]
#[serde(rename = "SKILL")]
#[serde(default)]
pub struct Skill {
    #[serde(rename = "INDEX")]
    pub id: i32,
    #[serde(rename = "OFFSETX")]
    pub x: f32,
    #[serde(rename = "OFFSETY")]
    pub y: f32,
    #[serde(rename = "LEVEL")]
    pub level: i32,
    #[serde(rename = "LIMITLEVEL")]
    pub limit_level: i32,
    #[serde(rename = "IMAGE")]
    pub image: String,

    #[serde(rename = "$value", default)]
    pub widgets: Vec<Widget>,

    #[serde(skip)]
    pub ui_texture: Option<UiTexture>,
}

impl LoadWidget for Skill {
    fn load_widget(&mut self, ui_resources: &UiResources) {
        self.widgets.load_widget(ui_resources);
    }
}

impl DrawWidget for Skill {
    fn draw_widget(&self, ui: &mut egui::Ui, bindings: &mut DataBindings) {
        if !bindings.get_visible(self.id) {
            return;
        }

        let ui_texture = if let Some(ui_texture) = self.ui_texture.as_ref() {
            ui_texture
        } else {
            return;
        };
        let size = if let Some(size) = ui_texture.size {
            size
        } else {
            return;
        };

        let rect = egui::Rect::from_min_size(
            ui.min_rect().min + egui::vec2(self.x, self.y),
            egui::vec2(size.x, size.y),
        );
        let response = ui.allocate_rect(rect, egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            let mut mesh = egui::epaint::Mesh::with_texture(ui_texture.texture_id);
            mesh.add_rect_with_uv(
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
            ui.painter().add(egui::epaint::Shape::mesh(mesh));
        }

        bindings.set_response(self.id, response);
    }
}
