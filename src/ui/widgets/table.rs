use bevy_egui::egui;
use serde::Deserialize;

use crate::resources::UiResources;

use super::{DataBindings, DrawWidget, LoadWidget};

#[derive(Clone, Default, Deserialize)]
#[serde(rename = "TABLE")]
#[serde(default)]
pub struct Table {
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
    #[serde(rename = "CELLWIDTH")]
    pub cell_width: f32,
    #[serde(rename = "CELLHEIGHT")]
    pub cell_height: f32,
    #[serde(rename = "COLUMNCOUNT")]
    pub column_count: i32,
    #[serde(rename = "COLMARGIN")]
    pub column_margin: f32,
    #[serde(rename = "ROWMARGIN")]
    pub row_margin: f32,
}

widget_to_rect! { Table }

impl LoadWidget for Table {
    fn load_widget(&mut self, _ui_resources: &UiResources) {}
}

impl DrawWidget for Table {
    fn draw_widget(&self, ui: &mut egui::Ui, bindings: &mut DataBindings) {
        if !bindings.get_visible(self.id) {
            return;
        }

        let (mut current_index, draw_table_item) =
            if let Some((current_index, draw_table_item)) = bindings.get_table(self.id) {
                (Some(current_index), Some(draw_table_item))
            } else {
                (None, None)
            };
        let rect = self.widget_rect(ui.min_rect().min);
        ui.allocate_ui_at_rect(rect, |ui| {
            egui::Grid::new(self.id)
                .num_columns(self.column_count as usize)
                .max_col_width(self.cell_width)
                .min_col_width(self.cell_width)
                .min_row_height(self.cell_height)
                .spacing(egui::vec2(self.column_margin, self.row_margin))
                .show(ui, |ui| {
                    for y in 0..self.extent {
                        for x in 0..self.column_count {
                            let index = x + y * self.column_count;

                            if let Some(draw_table_item) = draw_table_item {
                                if draw_table_item(ui, index, x, y).clicked() {
                                    if let Some(current_index) = current_index.as_mut() {
                                        **current_index = index;
                                    }
                                }
                            }
                        }

                        ui.end_row();
                    }
                });
        });
    }
}
