use bevy_egui::egui;
use serde::Deserialize;

use crate::resources::UiResources;

use super::{DataBindings, DrawWidget, LoadWidget};

#[derive(Clone, Default, Deserialize)]
#[serde(rename = "LISTBOX")]
#[serde(default)]
pub struct Listbox {
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
    #[serde(rename = "LINESPACE")]
    pub line_space: i32,
    #[serde(rename = "SELECTABLE")]
    pub selectable: i32,
    #[serde(rename = "CHARWIDTH")]
    pub char_width: i32,
    #[serde(rename = "CHARHEIGHT")]
    pub char_height: i32,
    #[serde(rename = "MAXSIZE")]
    pub max_size: i32,
    #[serde(rename = "OWNERDRAW")]
    pub owner_draw: i32,
    #[serde(rename = "FONT")]
    pub font: i32,
}

widget_to_rect! { Listbox }

impl LoadWidget for Listbox {
    fn load_widget(&mut self, _ui_resources: &UiResources) {}
}

impl DrawWidget for Listbox {
    fn draw_widget(&self, ui: &mut egui::Ui, bindings: &mut DataBindings) {
        if !bindings.get_visible(self.id) {
            return;
        }

        let rect = self.widget_rect(ui.min_rect().min);
        let response = ui.allocate_rect(rect, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            // TODO: Implement list box... somehow...
        }

        bindings.set_response(self.id, response);
    }
}
