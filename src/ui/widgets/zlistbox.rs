use bevy_egui::egui;
use serde::Deserialize;

use crate::resources::UiResources;

use super::{DataBindings, DrawWidget, LoadWidget};

#[derive(Clone, Default, Deserialize)]
#[serde(rename = "ZLISTBOX")]
#[serde(default)]
pub struct ZListbox {
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
    #[serde(rename = "EXTENT")]
    pub extent: i32,
}

widget_to_rect! { ZListbox }

impl LoadWidget for ZListbox {
    fn load_widget(&mut self, _ui_resources: &UiResources) {}
}

impl DrawWidget for ZListbox {
    fn draw_widget(&self, _ui: &mut egui::Ui, _bindings: &mut DataBindings) {}
}
