use bevy_egui::egui;
use serde::Deserialize;

use crate::resources::{UiResources, UiSprite};

use super::{DataBindings, DrawWidget, LoadWidget};

#[derive(Clone, Default, Deserialize)]
#[serde(rename = "CHECKBOX")]
#[serde(default)]
pub struct Checkbox {
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
    #[serde(rename = "CHECKGID")]
    pub checked_sprite_name: String,
    #[serde(rename = "UNCHECKGID")]
    pub unchecked_sprite_name: String,

    #[serde(skip)]
    pub checked_sprite: Option<UiSprite>,
    #[serde(skip)]
    pub unchecked_sprite: Option<UiSprite>,
}

widget_to_rect! { Checkbox }

impl LoadWidget for Checkbox {
    fn load_widget(&mut self, ui_resources: &UiResources) {
        self.checked_sprite = ui_resources.get_sprite(self.module_id, &self.checked_sprite_name);
        self.unchecked_sprite =
            ui_resources.get_sprite(self.module_id, &self.unchecked_sprite_name);
    }
}

impl DrawWidget for Checkbox {
    fn draw_widget(&self, ui: &mut egui::Ui, bindings: &mut DataBindings) {
        if !bindings.get_visible(self.id) {
            return;
        }

        let mut unbound_checked = false;
        let checked = bindings
            .checked
            .iter_mut()
            .find(|(id, _)| *id == self.id)
            .map(|(_, buffer)| &mut **buffer)
            .unwrap_or(&mut unbound_checked);

        let rect = self.widget_rect(ui.min_rect().min);
        let mut response = ui.allocate_rect(rect, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            if response.clicked() {
                *checked = !*checked;
                response.mark_changed();
            }

            let sprite = if *checked {
                self.checked_sprite.as_ref()
            } else {
                self.unchecked_sprite.as_ref()
            };

            if let Some(sprite) = sprite {
                sprite.draw(ui, rect.min);
            }
        }

        bindings.set_response(self.id, response);
    }
}
