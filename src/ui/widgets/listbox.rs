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
        let (scroll_index, scroll_range) = bindings
            .get_scroll(self.id)
            .as_ref()
            .map(|(scroll_index, scroll_range, _)| (**scroll_index, scroll_range.clone()))
            .unwrap_or((0, 0..self.extent));
        let mut listbox_response = None;

        ui.allocate_ui_at_rect(rect, |ui| {
            let rect_min = ui.min_rect().min;

            if let Some((current_index, get_item_text)) = bindings.get_list(self.id) {
                for i in 0..self.extent {
                    let index = scroll_index + i;
                    if index >= scroll_range.end {
                        break;
                    }
                    let Some(text) = get_item_text(i) else {
                        break;
                    };

                    let y = i * (self.char_height + self.line_space);
                    let color = if index == *current_index {
                        egui::Color32::from_rgb(255, 255, 128)
                    } else {
                        egui::Color32::WHITE
                    };

                    let response = ui
                        .allocate_ui_at_rect(
                            egui::Rect::from_min_size(
                                egui::pos2(rect_min.x, rect_min.y + y as f32),
                                egui::vec2(self.width, self.char_height as f32),
                            ),
                            |ui| {
                                ui.add(
                                    egui::Label::new(egui::RichText::new(text).color(color))
                                        .wrap(true)
                                        .sense(egui::Sense::click()),
                                )
                            },
                        )
                        .inner;
                    if response.clicked() {
                        *current_index = index;
                        listbox_response = Some(response);
                    }
                }
            }
        });

        if let Some(response) = listbox_response {
            bindings.set_response(self.id, response);
        }
    }
}
