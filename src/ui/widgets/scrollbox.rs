use serde::Deserialize;

use crate::resources::{UiResources, UiSprite};

use super::LoadWidget;

#[derive(Clone, Default, Deserialize)]
#[serde(rename = "SCROLLBOX")]
#[serde(default)]
pub struct Scrollbox {
    #[serde(rename = "ID")]
    pub id: i32,
    #[serde(rename = "NAME")]
    pub name: String,
    #[serde(rename = "WIDTH")]
    pub width: f32,
    #[serde(rename = "HEIGHT")]
    pub height: f32,
    #[serde(rename = "MODULEID")]
    pub module_id: i32,
    #[serde(rename = "TICKMOVE")]
    pub tick_move: i32,
    #[serde(rename = "GID")]
    pub sprite_name: String,
    #[serde(rename = "BLINKGID")]
    pub blink_sprite_name: String,
    #[serde(rename = "BLINK")]
    pub is_blink: i32,
    #[serde(rename = "BLINKMID")]
    pub blink_mid: i32,
    #[serde(rename = "BLINKSWAPTIME")]
    pub blink_swap_time: i32,

    #[serde(skip)]
    pub sprite: Option<UiSprite>,
    #[serde(skip)]
    pub blink_sprite: Option<UiSprite>,
}

impl LoadWidget for Scrollbox {
    fn load_widget(&mut self, ui_resources: &UiResources) {
        self.sprite = ui_resources.get_sprite(self.module_id, &self.sprite_name);
        self.blink_sprite = ui_resources.get_sprite(self.module_id, &self.blink_sprite_name);
    }
}
