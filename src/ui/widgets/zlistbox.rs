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
    fn draw_widget(&self, ui: &mut egui::Ui, bindings: &mut DataBindings) {
        let rect = self.widget_rect(ui.min_rect().min);

        let (scroll_index, scroll_range) = bindings
            .get_scroll(self.id)
            .as_ref()
            .map(|(scroll_index, scroll_range, _)| (**scroll_index, scroll_range.clone()))
            .unwrap_or((0, 0..self.extent));

        ui.allocate_ui_at_rect(rect, |ui| {
            ui.vertical(|ui| {
                if let Some((current_index, draw_list_item)) = bindings.get_zlist(self.id) {
                    for i in 0..self.extent {
                        let index = scroll_index + i;
                        if index >= scroll_range.end {
                            break;
                        }

                        if draw_list_item(ui, index, index == *current_index).clicked() {
                            *current_index = index;
                        }
                    }
                }
            });
        });
    }
}
