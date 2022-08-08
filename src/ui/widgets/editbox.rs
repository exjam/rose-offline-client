use bevy_egui::egui;
use serde::Deserialize;

use crate::resources::UiResources;

use super::{DataBindings, DrawWidget, LoadWidget};

#[derive(Clone, Default, Deserialize)]
#[serde(rename = "EDITBOX")]
#[serde(default)]
pub struct Editbox {
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
    #[serde(rename = "CHARWIDTH")]
    pub char_width: i32,
    #[serde(rename = "CHARHEIGHT")]
    pub char_height: i32,
    #[serde(rename = "NUMBER")]
    pub number: i32,
    #[serde(rename = "LIMITTEXT")]
    pub limit_text: i32,
    #[serde(rename = "PASSWORD")]
    pub password: i32,
    #[serde(rename = "HIDECURSOR")]
    pub hide_cursor: i32,
    #[serde(rename = "TEXTALIGN")]
    pub text_align: i32,
    #[serde(rename = "MULTILINE")]
    pub multiline: i32,
    #[serde(rename = "TEXTCOLOR")]
    pub textcolor: i32,
}

widget_to_rect! { Editbox }

impl LoadWidget for Editbox {
    fn load_widget(&mut self, _ui_resources: &UiResources) {}
}

impl DrawWidget for Editbox {
    fn draw_widget(&self, ui: &mut egui::Ui, bindings: &mut DataBindings) {
        if !bindings.get_visible(self.id) {
            return;
        }

        let mut unbound_buffer = format!("<{} unbound>", self.id);
        let enabled = bindings.get_enabled(self.id);
        let buffer = bindings.get_text(self.id).unwrap_or(&mut unbound_buffer);

        let rect = self.widget_rect(ui.min_rect().min);
        let response = ui
            .allocate_ui_at_rect(rect, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.add_enabled(
                        enabled,
                        egui::TextEdit::singleline(buffer)
                            .frame(false)
                            .margin(egui::vec2(0.0, 0.0))
                            .password(self.password != 0),
                    )
                })
                .inner
            })
            .inner;

        bindings.set_response(self.id, response);
    }
}
