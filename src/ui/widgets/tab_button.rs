use bevy_egui::egui;
use serde::Deserialize;

use rose_data::SoundId;

use crate::resources::{UiResources, UiSprite};

use super::{dialog::deserialize_sound_id, DataBindings, DrawWidget, LoadWidget};

#[derive(Clone, Default, Deserialize)]
#[serde(rename = "TABBUTTON")]
#[serde(default)]
pub struct TabButton {
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
    #[serde(deserialize_with = "deserialize_sound_id")]
    #[serde(rename = "DISABLESID")]
    pub disable_sound_id: Option<SoundId>,

    #[serde(skip)]
    pub tab_id: i32,
    #[serde(skip)]
    pub tabbed_pane_id: i32,
    #[serde(skip)]
    pub normal_sprite: Option<UiSprite>,
    #[serde(skip)]
    pub over_sprite: Option<UiSprite>,
    #[serde(skip)]
    pub down_sprite: Option<UiSprite>,
}

widget_to_rect! { TabButton }

impl LoadWidget for TabButton {
    fn load_widget(&mut self, ui_resources: &UiResources) {
        self.normal_sprite = ui_resources.get_sprite(self.module_id, &self.normal_sprite_name);
        self.over_sprite = ui_resources.get_sprite(self.module_id, &self.over_sprite_name);
        self.down_sprite = ui_resources.get_sprite(self.module_id, &self.down_sprite_name);
    }
}

impl DrawWidget for TabButton {
    fn draw_widget(&self, ui: &mut egui::Ui, bindings: &mut DataBindings) {
        if !bindings.get_visible(self.id) {
            return;
        }

        let enabled = bindings.get_enabled(self.id);
        let mut current_tab = bindings.get_tab(self.tabbed_pane_id);
        let selected = current_tab.as_mut().map_or(0, |x| **x) == self.tab_id;

        let rect = self.widget_rect(ui.min_rect().min);
        let response = ui.allocate_rect(rect, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let sprite = if !enabled {
                self.normal_sprite.as_ref()
            } else if selected || response.is_pointer_button_down_on() {
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

        if response.clicked() {
            if enabled {
                if let Some(current_tab) = current_tab.as_mut() {
                    **current_tab = self.tab_id;
                }
            } else if let Some(disable_sound_id) = self.disable_sound_id {
                bindings.emit_sound(disable_sound_id);
            }
        }

        bindings.set_response(self.id, response);
    }
}
