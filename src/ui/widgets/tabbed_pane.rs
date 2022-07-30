use bevy_egui::egui;
use serde::Deserialize;

use crate::resources::UiResources;

use super::{DataBindings, DrawWidget, LoadWidget, Tab, TabButton, Widget};

#[derive(Clone, Default, Deserialize)]
#[serde(rename = "TABBEDPANE")]
#[serde(default)]
pub struct TabbedPane {
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

    #[serde(rename = "TAB")]
    pub tabs: Vec<Tab>,
}

impl LoadWidget for TabbedPane {
    fn load_widget(&mut self, ui_resources: &UiResources) {
        for tab in self.tabs.iter_mut() {
            tab.load_widget(ui_resources);

            // Assign tab buttons to tabs
            for (index, widget) in tab.widgets.iter_mut().enumerate() {
                if let Widget::TabButton(tab_button) = widget {
                    tab_button.tab_id = tab.id;
                    tab_button.tabbed_pane_id = self.id;
                    tab.tab_button_widget_index = Some(index);
                }
            }
        }
    }
}

struct DrawTabButton<'a> {
    tab_button: &'a TabButton,
    selected: bool,
}

impl<'a> egui::Widget for DrawTabButton<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let rect = self.tab_button.widget_rect(ui.min_rect().min);
        let response = ui.allocate_rect(rect, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let sprite = if self.selected || response.is_pointer_button_down_on() {
                self.tab_button.down_sprite.as_ref()
            } else if response.hovered() || response.has_focus() {
                self.tab_button.over_sprite.as_ref()
            } else {
                None
            }
            .or(self.tab_button.normal_sprite.as_ref());

            if let Some(sprite) = sprite {
                sprite.draw(ui, rect.min);
            }
        }

        response
    }
}

impl DrawWidget for TabbedPane {
    fn draw_widget(&self, ui: &mut egui::Ui, bindings: &mut DataBindings) {
        if !bindings.get_visible(self.id) || self.tabs.is_empty() {
            return;
        }

        let mut rect = ui.min_rect();
        rect.min.x += self.x;
        rect.min.y += self.y;

        ui.allocate_ui_at_rect(rect, |ui| {
            let current_tab = bindings
                .get_tab(self.id)
                .map(|x| *x)
                .unwrap_or(self.tabs[0].id);

            // Draw active tab
            for tab in self.tabs.iter() {
                if tab.id == current_tab {
                    tab.widgets.draw_widget(ui, bindings);
                }
            }

            // Draw inactive tab buttons
            for tab in self.tabs.iter() {
                if tab.id != current_tab {
                    if let Some(tab_button) = tab.tab_button() {
                        tab_button.draw_widget(ui, bindings);
                    }
                }
            }
        });
    }
}
