use bevy_egui::egui;
use serde::Deserialize;

use crate::resources::{UiResources, UiSprite};

use super::{DataBindings, DrawWidget, LoadWidget};

#[derive(Clone, Default, Deserialize)]
#[serde(rename = "RADIOBUTTON")]
#[serde(default)]
pub struct RadioButton {
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

    #[serde(rename = "RADIOBOXID")]
    pub radio_box_id: i32,
    #[serde(rename = "MODULEID")]
    pub module_id: i32,
    #[serde(rename = "NORMALGID")]
    pub normal_sprite_name: String,
    #[serde(rename = "OVERGID")]
    pub over_sprite_name: String,
    #[serde(rename = "DOWNGID")]
    pub down_sprite_name: String,
    #[serde(rename = "DISABLESID")]
    pub disable_sound_id: i32,

    #[serde(skip)]
    pub normal_sprite: Option<UiSprite>,
    #[serde(skip)]
    pub over_sprite: Option<UiSprite>,
    #[serde(skip)]
    pub down_sprite: Option<UiSprite>,
}

widget_to_rect! { RadioButton }

impl LoadWidget for RadioButton {
    fn load_widget(&mut self, ui_resources: &UiResources) {
        self.normal_sprite = ui_resources.get_sprite(self.module_id, &self.normal_sprite_name);
        self.over_sprite = ui_resources.get_sprite(self.module_id, &self.over_sprite_name);
        self.down_sprite = ui_resources.get_sprite(self.module_id, &self.down_sprite_name);
    }
}

impl DrawWidget for RadioButton {
    fn draw_widget(&self, ui: &mut egui::Ui, bindings: &mut DataBindings) {
        if !bindings.get_visible(self.id) {
            return;
        }

        let mut unbound_selected = 0;
        let selected = bindings
            .get_radio(self.radio_box_id)
            .unwrap_or(&mut unbound_selected);

        let rect = self.widget_rect(ui.min_rect().min);
        let mut response = ui.allocate_rect(rect, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            if response.clicked() {
                *selected = self.id;
                response.mark_changed();
            }

            let sprite = if *selected == self.id || response.is_pointer_button_down_on() {
                self.down_sprite.as_ref()
            } else if response.hovered() || response.has_focus() {
                self.over_sprite.as_ref()
            } else {
                None
            }
            .or(self.normal_sprite.as_ref());

            if let Some(sprite) = sprite {
                sprite.draw(ui, rect.min);
            }
        }

        bindings.set_response(self.id, response);
    }
}
