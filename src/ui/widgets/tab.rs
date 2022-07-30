use serde::Deserialize;

use crate::resources::UiResources;

use super::{LoadWidget, TabButton, Widget};

#[derive(Clone, Default, Deserialize)]
#[serde(rename = "TAB")]
#[serde(default)]
pub struct Tab {
    #[serde(rename = "ID")]
    pub id: i32,

    #[serde(rename = "$value")]
    pub widgets: Vec<Widget>,

    #[serde(skip)]
    pub tab_button_widget_index: Option<usize>, // Index into self.widgets
}

impl Tab {
    pub fn tab_button(&self) -> Option<&TabButton> {
        if let Some(Widget::TabButton(tab_button)) = self
            .tab_button_widget_index
            .and_then(|i| self.widgets.get(i))
        {
            Some(tab_button)
        } else {
            None
        }
    }
}

impl LoadWidget for Tab {
    fn load_widget(&mut self, ui_resources: &UiResources) {
        self.widgets.load_widget(ui_resources);
    }
}
