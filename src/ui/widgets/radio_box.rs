use bevy_egui::egui;
use serde::Deserialize;

use crate::resources::UiResources;

use super::{DataBindings, DrawWidget, LoadWidget};

#[derive(Clone, Default, Deserialize)]
#[serde(rename = "RADIOBOX")]
#[serde(default)]
pub struct RadioBox {
    #[serde(rename = "ID")]
    pub id: i32,
    #[serde(rename = "NAME")]
    pub name: String,
}

impl LoadWidget for RadioBox {
    fn load_widget(&mut self, _ui_resources: &UiResources) {}
}

impl DrawWidget for RadioBox {
    fn draw_widget(&self, _ui: &mut egui::Ui, _bindings: &mut DataBindings) {}
}
