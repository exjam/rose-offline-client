use bevy_egui::egui;
use serde::Deserialize;

use crate::resources::UiResources;

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

    #[serde(rename = "$value")]
    pub widgets: Vec<Widget>,
}

impl LoadWidget for Skill {
    fn load_widget(&mut self, ui_resources: &UiResources) {
        self.widgets.load_widget(ui_resources);
    }
}

impl DrawWidget for Skill {
    fn draw_widget(&self, _ui: &mut egui::Ui, _bindings: &mut DataBindings) {}
}
