use bevy_egui::egui;
use serde::Deserialize;

use crate::resources::{UiResources, UiSprite};

use super::{DataBindings, DrawWidget, LoadWidget};

#[derive(Clone, Default, Deserialize)]
#[serde(rename = "BUTTON")]
#[serde(default)]
pub struct Button {
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
    #[serde(rename = "MODULEID")]
    pub module_id: i32,
    #[serde(rename = "NORMALGID")]
    pub normal_sprite_name: String,
    #[serde(rename = "OVERGID")]
    pub over_sprite_name: String,
    #[serde(rename = "DOWNGID")]
    pub down_sprite_name: String,
    #[serde(rename = "BLINKGID")]
    pub blink_sprite_name: String,
    #[serde(rename = "DISABLEGID")]
    pub disable_sprite_name: String,
    #[serde(rename = "OVERSID")]
    pub over_sound_id: i32,
    #[serde(rename = "CLICKSID")]
    pub click_sound_id: i32,
    #[serde(rename = "NOIMAGE")]
    pub no_image: i32,

    #[serde(skip)]
    pub normal_sprite: Option<UiSprite>,
    #[serde(skip)]
    pub over_sprite: Option<UiSprite>,
    #[serde(skip)]
    pub down_sprite: Option<UiSprite>,
    #[serde(skip)]
    pub blink_sprite: Option<UiSprite>,
    #[serde(skip)]
    pub disable_sprite: Option<UiSprite>,
}

widget_to_rect! { Button }

impl LoadWidget for Button {
    fn load_widget(&mut self, ui_resources: &UiResources) {
        self.normal_sprite = ui_resources.get_sprite(self.module_id, &self.normal_sprite_name);
        self.over_sprite = ui_resources.get_sprite(self.module_id, &self.over_sprite_name);
        self.blink_sprite = ui_resources.get_sprite(self.module_id, &self.blink_sprite_name);
        self.down_sprite = ui_resources.get_sprite(self.module_id, &self.down_sprite_name);
        self.disable_sprite = ui_resources.get_sprite(self.module_id, &self.disable_sprite_name);
    }
}

impl DrawWidget for Button {
    fn draw_widget(&self, ui: &mut egui::Ui, bindings: &mut DataBindings) {
        if !bindings.get_visible(self.id) {
            return;
        }

        let rect = self.widget_rect(ui.min_rect().min);
        let response = ui.allocate_rect(rect, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let sprite = if !response.sense.interactive() {
                self.disable_sprite.as_ref()
            } else if response.is_pointer_button_down_on() {
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
