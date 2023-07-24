use bevy::reflect::{TypePath, TypeUuid};
use bevy_egui::egui;
use serde::{Deserialize, Deserializer};

use rose_data::SoundId;

use super::{DataBindings, DrawWidget, GetWidget, Widget};

pub fn default_on_error<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    #[derive(Debug, serde::Deserialize)]
    #[serde(untagged)]
    enum GoodOrError<T> {
        Good(T),
        // This consumes one "item" when `T` errors while deserializing.
        // This is necessary to make this work, when instead of having a direct value
        // like integer or string, the deserializer sees a list or map.
        Error(serde::de::IgnoredAny),
    }

    Ok(match Deserialize::deserialize(deserializer) {
        Ok(GoodOrError::Good(res)) => res,
        _ => Default::default(),
    })
}

pub fn deserialize_sound_id<'de, D>(deserializer: D) -> Result<Option<SoundId>, D::Error>
where
    D: Deserializer<'de>,
{
    let sound_id = u16::deserialize(deserializer).unwrap_or(0);
    Ok(SoundId::new(sound_id))
}

#[derive(Clone, Default, Deserialize, TypeUuid, TypePath)]
#[uuid = "95ddb227-6e9f-43ee-8026-28ddb6fc9634"]
#[serde(rename = "Root_Element")]
#[serde(default)]
pub struct Dialog {
    #[serde(deserialize_with = "default_on_error")]
    #[serde(rename = "X")]
    pub x: f32,
    #[serde(deserialize_with = "default_on_error")]
    #[serde(rename = "Y")]
    pub y: f32,
    #[serde(rename = "WIDTH")]
    pub width: f32,
    #[serde(rename = "HEIGHT")]
    pub height: f32,
    #[serde(rename = "MODAL")]
    pub modal: i32,
    #[serde(deserialize_with = "deserialize_sound_id")]
    #[serde(rename = "SHOWSID")]
    pub show_sound_id: Option<SoundId>,
    #[serde(deserialize_with = "deserialize_sound_id")]
    #[serde(rename = "HIDESID")]
    pub hide_sound_id: Option<SoundId>,
    #[serde(rename = "EXTENT")]
    pub extent: i32,
    #[serde(rename = "DEFAULT_X")]
    pub default_x: f32,
    #[serde(rename = "DEFAULT_Y")]
    pub default_y: f32,
    #[serde(rename = "DEFAULT_VISIBLE")]
    pub default_visible: i32,
    #[serde(rename = "ADJUST_X")]
    pub adjust_x: f32,
    #[serde(rename = "ADJUST_Y")]
    pub adjust_y: f32,

    #[serde(rename = "$value")]
    pub widgets: Vec<Widget>,

    #[serde(skip)]
    pub loaded: bool,
}

impl Dialog {
    pub fn draw<R>(
        &self,
        ui: &mut egui::Ui,
        mut bindings: DataBindings,
        add_contents: impl FnOnce(&mut egui::Ui, &mut DataBindings) -> R,
    ) {
        let style = ui.style_mut();
        style.visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::WHITE;
        style.spacing.item_spacing = egui::Vec2::ZERO;
        style.spacing.window_margin = egui::style::Margin::same(0.0);

        self.widgets.draw_widget(ui, &mut bindings);

        add_contents(ui, &mut bindings);
    }

    pub fn get_widget(&self, id: i32) -> Option<&Widget> {
        self.widgets.get_widget(id)
    }

    pub fn get_widget_mut(&mut self, id: i32) -> Option<&mut Widget> {
        self.widgets.get_widget_mut(id)
    }
}
