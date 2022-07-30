use bevy_egui::egui;
use serde::Deserialize;

use crate::resources::UiResources;

use super::{DataBindings, DrawWidget, LoadWidget};

#[derive(Clone, Default, Deserialize)]
#[serde(rename = "CAPTION")]
#[serde(default)]
pub struct Caption {
    #[serde(rename = "ID")]
    pub id: i32,
    #[serde(rename = "NAME")]
    pub name: String,
    #[serde(rename = "X")]
    pub x: f32,
    #[serde(rename = "Y")]
    pub y: f32,
    #[serde(rename = "WIDTH")]
    pub width: f32,
    #[serde(rename = "HEIGHT")]
    pub height: f32,
}

impl LoadWidget for Caption {
    fn load_widget(&mut self, _ui_resources: &UiResources) {}
}

impl DrawWidget for Caption {
    fn draw_widget(&self, _ui: &mut egui::Ui, _bindings: &mut DataBindings) {}
}
