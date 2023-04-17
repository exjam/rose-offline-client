use bevy_egui::egui;
use serde::Deserialize;

use crate::resources::UiResources;

use super::{DataBindings, DrawWidget, LoadWidget, Scrollbox};

#[derive(Clone, Default, Deserialize)]
#[serde(rename = "SCROLLBAR")]
#[serde(default)]
pub struct Scrollbar {
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

    #[serde(rename = "LISTBOXID")]
    pub listbox_id: i32,
    #[serde(rename = "TYPE")]
    pub scrollbar_type: i32,

    #[serde(rename = "$value")]
    pub scrollbox: Option<Scrollbox>,
}

widget_to_rect! { Scrollbar }

impl LoadWidget for Scrollbar {
    fn load_widget(&mut self, ui_resources: &UiResources) {
        if let Some(scrollbox) = self.scrollbox.as_mut() {
            scrollbox.load_widget(ui_resources);
        }
    }
}

impl DrawWidget for Scrollbar {
    fn draw_widget(&self, ui: &mut egui::Ui, bindings: &mut DataBindings) {
        if !bindings.get_visible(self.id) {
            return;
        }

        if let Some((current, range, extent)) = bindings.get_scroll(self.listbox_id) {
            let rect = self.widget_rect(ui.min_rect().min);
            let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());
            let range = range.start..(range.end - extent).max(range.start);

            if !range.is_empty() {
                if let Some(scrollbox) = self.scrollbox.as_ref() {
                    let start = rect.min.y + scrollbox.height / 2.0;
                    let end = rect.max.y - scrollbox.height / 2.0;
                    let range_size = (range.end - range.start) as f32;

                    if let Some(pointer_position_2d) = response.interact_pointer_pos() {
                        // Calculate value from position
                        let pos = pointer_position_2d.y.clamp(start, end);
                        let value =
                            range.start + (range_size * (pos - start) / (end - start)) as i32;
                        *current = value;
                    }

                    if ui.is_rect_visible(rect) {
                        // Calculate position from value
                        let pos = rect.min.y + *current as f32 * ((end - start) / range_size);

                        if let Some(sprite) = scrollbox.sprite.as_ref() {
                            sprite.draw(ui, egui::pos2(rect.min.x, pos));
                        }
                    }
                }
            }

            bindings.set_response(self.id, response);

            if ui.rect_contains_pointer(rect) {
                let scroll_delta = ui.input(|input| input.scroll_delta);
                if let Some((scroll_index, scroll_range, extent)) = bindings
                    .get_scroll(self.listbox_id)
                    .as_mut()
                    .map(|(scroll_index, scroll_range, extent)| {
                        (scroll_index, scroll_range.clone(), *extent)
                    })
                {
                    if scroll_delta.y > 0.0 && **scroll_index > scroll_range.start {
                        **scroll_index -= 1;
                    } else if scroll_delta.y < 0.0 && **scroll_index < (scroll_range.end - extent) {
                        **scroll_index += 1;
                    }
                }
            }
        }
    }
}
