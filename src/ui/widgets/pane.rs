use bevy_egui::egui;
use serde::Deserialize;

use crate::resources::UiResources;

use super::{DataBindings, DrawWidget, LoadWidget, Widget};

#[derive(Clone, Default, Deserialize)]
#[serde(rename = "PANE")]
#[serde(default)]
pub struct Pane {
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

    #[serde(rename = "$value")]
    pub widgets: Vec<Widget>,
}

widget_to_rect! { Pane }

impl LoadWidget for Pane {
    fn load_widget(&mut self, ui_resources: &UiResources) {
        self.widgets.load_widget(ui_resources);
    }
}

impl DrawWidget for Pane {
    fn draw_widget(&self, ui: &mut egui::Ui, bindings: &mut DataBindings) {
        if !bindings.get_visible(self.id) {
            return;
        }

        ui.allocate_ui_at_rect(self.widget_rect(ui.min_rect().min), |ui| {
            self.widgets.draw_widget(ui, bindings)
        });
    }
}
