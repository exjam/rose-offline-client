use bevy_egui::{egui, egui::Widget};
use serde::Deserialize;

use rose_data::SoundId;

use crate::resources::{UiResources, UiSprite};

use super::{dialog::deserialize_sound_id, DataBindings, DrawWidget, LoadWidget};

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
    #[serde(deserialize_with = "deserialize_sound_id")]
    #[serde(rename = "OVERSID")]
    pub over_sound_id: Option<SoundId>,
    #[serde(deserialize_with = "deserialize_sound_id")]
    #[serde(rename = "CLICKSID")]
    pub click_sound_id: Option<SoundId>,
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
        let enabled = bindings.get_enabled(self.id);
        let mut response = ui.allocate_rect(
            rect,
            if enabled {
                egui::Sense::click()
            } else {
                egui::Sense::hover()
            },
        );

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

            if response.clicked() {
                if let Some(click_sound_id) = self.click_sound_id {
                    bindings.emit_sound(click_sound_id);
                }
            }

            let label = bindings.get_label(self.id);
            if let Some(label) = label {
                let rect = rect.shrink(2.0);
                let mut child_ui =
                    ui.child_ui(rect, egui::Layout::top_down_justified(egui::Align::Center));
                let style = ui.style();
                let mut font_id = style.override_text_style.clone().map_or_else(
                    || egui::FontSelection::Default.resolve(style),
                    |text_style| text_style.resolve(style),
                );
                font_id.size = 12.0;

                let mut layout_job = egui::epaint::text::LayoutJob::single_section(
                    label.to_string(),
                    egui::TextFormat::simple(font_id, egui::Color32::BLACK),
                );
                layout_job.wrap = egui::epaint::text::TextWrapping {
                    max_width: rect.width(),
                    max_rows: 1,
                    break_anywhere: true,
                    overflow_character: Some('…'),
                };

                let galley = ui.fonts(|fonts| fonts.layout_job(layout_job));
                let was_truncated = galley
                    .rows
                    .last()
                    .and_then(|row| row.glyphs.last())
                    .map_or(false, |glyph| glyph.chr == '…');
                egui::Label::new(galley).wrap(true).ui(&mut child_ui);

                if was_truncated {
                    response = response.on_hover_text(label);
                }
            }
        }

        bindings.set_response(self.id, response);
    }
}
