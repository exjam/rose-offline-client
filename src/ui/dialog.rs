use bevy::{
    asset::{AssetLoader, BoxedFuture, LoadContext, LoadState, LoadedAsset},
    prelude::{AssetEvent, AssetServer, Assets, EventReader, Handle, Image, Local, Res, ResMut},
    reflect::TypeUuid,
};

use bevy_egui::egui;
use quick_xml::de::from_slice;
use serde::Deserialize;

use crate::resources::{UiResources, UiSpriteSheetType};

#[derive(Default)]
pub struct DialogLoader;

impl AssetLoader for DialogLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            let dialog: Dialog = from_slice(bytes)?;
            load_context.set_default_asset(LoadedAsset::new(dialog));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["xml"]
    }
}

macro_rules! widget_to_rect {
    ( $x:ident ) => {
        impl $x {
            pub fn widget_rect(&self, min: egui::Pos2) -> egui::Rect {
                egui::Rect::from_min_size(
                    min + egui::vec2(self.x, self.y) + egui::vec2(self.offset_x, self.offset_y),
                    egui::vec2(self.width, self.height),
                )
            }
        }
    };
}

#[derive(Clone, Default, Deserialize, TypeUuid)]
#[uuid = "95ddb227-6e9f-43ee-8026-28ddb6fc9634"]
#[serde(rename = "Root_Element")]
#[serde(default)]
pub struct Dialog {
    #[serde(rename = "X")]
    pub x: f32,
    #[serde(rename = "Y")]
    pub y: f32,
    #[serde(rename = "WIDTH")]
    pub width: f32,
    #[serde(rename = "HEIGHT")]
    pub height: f32,
    #[serde(rename = "MODAL")]
    pub modal: i32,
    #[serde(rename = "SHOWSID")]
    pub show_sound_id: i32,
    #[serde(rename = "HIDESID")]
    pub hide_sound_id: i32,
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

pub trait GetWidget {
    fn get_widget_mut(&mut self, id: i32) -> Option<&mut Widget>;
}

impl GetWidget for Vec<Widget> {
    fn get_widget_mut(&mut self, id: i32) -> Option<&mut Widget> {
        for widget in self.iter_mut() {
            if widget.id() == id {
                return Some(widget);
            }

            match widget {
                Widget::Pane(pane) => {
                    if let Some(widget) = pane.widgets.get_widget_mut(id) {
                        return Some(widget);
                    }
                }
                Widget::TabbedPane(tabbed_pane) => {
                    for tab in tabbed_pane.tabs.iter_mut() {
                        if let Some(widget) = tab.widgets.get_widget_mut(id) {
                            return Some(widget);
                        }
                    }
                }
                Widget::Button(_)
                | Widget::Caption(_)
                | Widget::Checkbox(_)
                | Widget::Gauge(_)
                | Widget::Listbox(_)
                | Widget::Textbox(_)
                | Widget::Sprite(_)
                | Widget::TabButton(_) => {
                    continue;
                }
            }
        }

        None
    }
}

impl GetWidget for Dialog {
    fn get_widget_mut(&mut self, id: i32) -> Option<&mut Widget> {
        self.widgets.get_widget_mut(id)
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Deserialize)]
pub enum Widget {
    #[serde(rename = "BUTTON")]
    Button(Button),
    #[serde(rename = "CAPTION")]
    Caption(Caption),
    #[serde(rename = "CHECKBOX")]
    Checkbox(Checkbox),
    #[serde(rename = "GUAGE")]
    Gauge(Gauge),
    #[serde(rename = "LISTBOX")]
    Listbox(Listbox),
    #[serde(rename = "EDITBOX")]
    Textbox(Textbox),
    #[serde(rename = "PANE")]
    Pane(Pane),
    #[serde(rename = "TABBUTTON")]
    TabButton(TabButton),
    #[serde(rename = "TABBEDPANE")]
    TabbedPane(TabbedPane),
    #[serde(rename = "IMAGE")]
    Sprite(Sprite),
}

impl Widget {
    pub fn id(&self) -> i32 {
        match self {
            Widget::Button(x) => x.id,
            Widget::Caption(x) => x.id,
            Widget::Checkbox(x) => x.id,
            Widget::Gauge(x) => x.id,
            Widget::Listbox(x) => x.id,
            Widget::Textbox(x) => x.id,
            Widget::Pane(x) => x.id,
            Widget::TabButton(x) => x.id,
            Widget::TabbedPane(x) => x.id,
            Widget::Sprite(x) => x.id,
        }
    }
}

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
    pub normal_sprite: Option<LoadedSprite>,
    #[serde(skip)]
    pub over_sprite: Option<LoadedSprite>,
    #[serde(skip)]
    pub down_sprite: Option<LoadedSprite>,
    #[serde(skip)]
    pub blink_sprite: Option<LoadedSprite>,
    #[serde(skip)]
    pub disable_sprite: Option<LoadedSprite>,
}

widget_to_rect! { Button }

#[derive(Clone, Default, Deserialize)]
#[serde(rename = "CAPTION")]
#[serde(default)]
pub struct Caption {
    #[serde(rename = "ID")]
    pub id: i32,
    #[serde(rename = "NAME")]
    pub name: String,
    #[serde(rename = "X")]
    pub x: f32,
    #[serde(rename = "Y")]
    pub y: f32,
    #[serde(rename = "WIDTH")]
    pub width: f32,
    #[serde(rename = "HEIGHT")]
    pub height: f32,
}

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
    pub checked_sprite: Option<LoadedSprite>,
    #[serde(skip)]
    pub unchecked_sprite: Option<LoadedSprite>,
}

widget_to_rect! { Checkbox }

#[derive(Clone, Default, Deserialize)]
#[serde(rename = "GUAGE")] // Intentionally incorrect spelling
#[serde(default)]
pub struct Gauge {
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
    #[serde(rename = "GID")]
    pub foreground_sprite_name: String,
    #[serde(rename = "BGID")]
    pub background_sprite_name: String,

    #[serde(skip)]
    pub foreground_sprite: Option<LoadedSprite>,
    #[serde(skip)]
    pub background_sprite: Option<LoadedSprite>,
}

widget_to_rect! { Gauge }

#[derive(Clone, Default, Deserialize)]
#[serde(rename = "LISTBOX")]
#[serde(default)]
pub struct Listbox {
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
    #[serde(rename = "EXTENT")]
    pub extent: i32,
    #[serde(rename = "LINESPACE")]
    pub line_space: i32,
    #[serde(rename = "SELECTABLE")]
    pub selectable: i32,
    #[serde(rename = "CHARWIDTH")]
    pub char_width: i32,
    #[serde(rename = "CHARHEIGHT")]
    pub char_height: i32,
    #[serde(rename = "MAXSIZE")]
    pub max_size: i32,
    #[serde(rename = "OWNERDRAW")]
    pub owner_draw: i32,
    #[serde(rename = "FONT")]
    pub font: i32,
}

widget_to_rect! { Listbox }

#[derive(Clone, Default, Deserialize)]
#[serde(rename = "EDITBOX")]
#[serde(default)]
pub struct Textbox {
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
    #[serde(rename = "CHARWIDTH")]
    pub char_width: i32,
    #[serde(rename = "CHARHEIGHT")]
    pub char_height: i32,
    #[serde(rename = "NUMBER")]
    pub number: i32,
    #[serde(rename = "LIMITTEXT")]
    pub limit_text: i32,
    #[serde(rename = "PASSWORD")]
    pub password: i32,
    #[serde(rename = "HIDECURSOR")]
    pub hide_cursor: i32,
    #[serde(rename = "TEXTALIGN")]
    pub text_align: i32,
    #[serde(rename = "MULTILINE")]
    pub multiline: i32,
    #[serde(rename = "TEXTCOLOR")]
    pub textcolor: i32,
}

widget_to_rect! { Textbox }

#[derive(Clone, Default, Deserialize)]
#[serde(rename = "PANE")]
#[serde(default)]
pub struct Pane {
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

    #[serde(rename = "$value")]
    pub widgets: Vec<Widget>,
}

widget_to_rect! { Pane }

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
    #[serde(rename = "DISABLESID")]
    pub disable_sound_id: i32,

    #[serde(skip)]
    pub normal_sprite: Option<LoadedSprite>,
    #[serde(skip)]
    pub over_sprite: Option<LoadedSprite>,
    #[serde(skip)]
    pub down_sprite: Option<LoadedSprite>,
}

widget_to_rect! { TabButton }

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

#[derive(Clone, Default, Deserialize)]
#[serde(rename = "IMAGE")]
#[serde(default)]
pub struct Sprite {
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
    #[serde(rename = "GID")]
    pub sprite_name: String,
    #[serde(rename = "ALPHAVALUE")]
    pub alpha_value: i32,
    #[serde(rename = "SCALEWIDTH")]
    pub scale_width: f32,
    #[serde(rename = "SCALEHEIGHT")]
    pub scale_height: f32,

    #[serde(skip)]
    pub sprite: Option<LoadedSprite>,
}

widget_to_rect! { Sprite }

#[derive(Clone)]
pub struct LoadedSprite {
    texture_id: egui::TextureId,
    uv: egui::Rect,
    width: f32,
    height: f32,
}

impl LoadedSprite {
    pub fn try_load(
        resources: &UiResources,
        images: &Assets<Image>,
        module_id: i32,
        sprite_name: &str,
    ) -> Option<LoadedSprite> {
        let sprite_sheet_type = match module_id {
            0 => UiSpriteSheetType::Ui,
            3 => UiSpriteSheetType::ExUi,
            _ => return None,
        };
        let sprite_sheet = &resources.sprite_sheets[sprite_sheet_type];
        let sprite_index = sprite_sheet.sprites_by_name.get(sprite_name)?;

        let sprite = sprite_sheet.sprites.get(*sprite_index as usize)?;
        let (image_handle, texture_id) = sprite_sheet
            .loaded_textures
            .get(sprite.texture_id as usize)?;

        let image_size = images.get(image_handle)?.size();

        Some(LoadedSprite {
            texture_id: *texture_id,
            uv: egui::Rect::from_min_max(
                egui::pos2(
                    (sprite.left as f32 + 0.5) / image_size.x,
                    (sprite.top as f32 + 0.5) / image_size.y,
                ),
                egui::pos2(
                    (sprite.right as f32 - 0.5) / image_size.x,
                    (sprite.bottom as f32 - 0.5) / image_size.y,
                ),
            ),
            width: (sprite.right - sprite.left) as f32,
            height: (sprite.bottom - sprite.top) as f32,
        })
    }
}

#[derive(Default)]
pub struct DialogsLoadState {
    sprite_sheets_loaded: bool,
    pending_dialogs: Vec<Handle<Dialog>>,
}

fn load_widgets_sprites(
    widgets: &mut [Widget],
    ui_resources: &UiResources,
    images: &Assets<Image>,
) {
    for widget in widgets.iter_mut() {
        match widget {
            Widget::Button(button) => {
                button.normal_sprite = LoadedSprite::try_load(
                    ui_resources,
                    images,
                    button.module_id,
                    &button.normal_sprite_name,
                );
                button.over_sprite = LoadedSprite::try_load(
                    ui_resources,
                    images,
                    button.module_id,
                    &button.over_sprite_name,
                );
                button.blink_sprite = LoadedSprite::try_load(
                    ui_resources,
                    images,
                    button.module_id,
                    &button.blink_sprite_name,
                );
                button.down_sprite = LoadedSprite::try_load(
                    ui_resources,
                    images,
                    button.module_id,
                    &button.down_sprite_name,
                );
                button.disable_sprite = LoadedSprite::try_load(
                    ui_resources,
                    images,
                    button.module_id,
                    &button.disable_sprite_name,
                );
            }
            Widget::Checkbox(checkbox) => {
                checkbox.checked_sprite = LoadedSprite::try_load(
                    ui_resources,
                    images,
                    checkbox.module_id,
                    &checkbox.checked_sprite_name,
                );
                checkbox.unchecked_sprite = LoadedSprite::try_load(
                    ui_resources,
                    images,
                    checkbox.module_id,
                    &checkbox.unchecked_sprite_name,
                );
            }
            Widget::Gauge(gauge) => {
                gauge.foreground_sprite = LoadedSprite::try_load(
                    ui_resources,
                    images,
                    gauge.module_id,
                    &gauge.foreground_sprite_name,
                );
                gauge.background_sprite = LoadedSprite::try_load(
                    ui_resources,
                    images,
                    gauge.module_id,
                    &gauge.background_sprite_name,
                );
            }
            Widget::Sprite(sprite) => {
                sprite.sprite = LoadedSprite::try_load(
                    ui_resources,
                    images,
                    sprite.module_id,
                    &sprite.sprite_name,
                );
            }
            Widget::Pane(pane) => {
                load_widgets_sprites(&mut pane.widgets, ui_resources, images);
            }
            Widget::TabButton(tab_button) => {
                tab_button.normal_sprite = LoadedSprite::try_load(
                    ui_resources,
                    images,
                    tab_button.module_id,
                    &tab_button.normal_sprite_name,
                );
                tab_button.over_sprite = LoadedSprite::try_load(
                    ui_resources,
                    images,
                    tab_button.module_id,
                    &tab_button.over_sprite_name,
                );
                tab_button.down_sprite = LoadedSprite::try_load(
                    ui_resources,
                    images,
                    tab_button.module_id,
                    &tab_button.down_sprite_name,
                );
            }
            Widget::TabbedPane(pane) => {
                for tab in pane.tabs.iter_mut() {
                    load_widgets_sprites(&mut tab.widgets, ui_resources, images);

                    for (index, widget) in tab.widgets.iter().enumerate() {
                        if matches!(widget, Widget::TabButton(_)) {
                            tab.tab_button_widget_index = Some(index);
                        }
                    }
                }
            }
            Widget::Textbox(_) | Widget::Listbox(_) | Widget::Caption(_) => {}
        }
    }
}

pub fn load_dialog_sprites_system(
    mut ev_asset: EventReader<AssetEvent<Dialog>>,
    mut assets: ResMut<Assets<Dialog>>,
    mut load_state: Local<DialogsLoadState>,
    images: Res<Assets<Image>>,
    asset_server: Res<AssetServer>,
    ui_resources: Res<UiResources>,
) {
    if !load_state.sprite_sheets_loaded
        && !ui_resources
            .sprite_sheets_load_group
            .iter()
            .any(|id| matches!(asset_server.get_load_state(*id), LoadState::Loading))
    {
        load_state.sprite_sheets_loaded = true;
    }

    for ev in ev_asset.iter() {
        match ev {
            AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                load_state.pending_dialogs.push(handle.clone_weak());
            }
            _ => {}
        }
    }

    if load_state.sprite_sheets_loaded {
        for handle in load_state.pending_dialogs.drain(..) {
            let dialog = assets.get_mut(&handle).unwrap();

            load_widgets_sprites(&mut dialog.widgets, &ui_resources, &images);
            dialog.loaded = true;
        }
    }
}

fn draw_loaded_sprite(ui: &mut egui::Ui, pos: egui::Pos2, sprite: &LoadedSprite) {
    use egui::epaint::*;
    let rect = egui::Rect::from_min_size(pos, egui::vec2(sprite.width, sprite.height));
    let mut mesh = Mesh::with_texture(sprite.texture_id);
    mesh.add_rect_with_uv(rect, sprite.uv, Color32::WHITE);
    ui.painter().add(Shape::mesh(mesh));
}

fn draw_loaded_sprite_stretched(ui: &mut egui::Ui, rect: egui::Rect, sprite: &LoadedSprite) {
    use egui::epaint::*;
    let mut mesh = Mesh::with_texture(sprite.texture_id);
    mesh.add_rect_with_uv(rect, sprite.uv, Color32::WHITE);
    ui.painter().add(Shape::mesh(mesh));
}

struct DrawButton<'a> {
    button: &'a Button,
}

impl<'a> egui::Widget for DrawButton<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let rect = self.button.widget_rect(ui.min_rect().min);
        let response = ui.allocate_rect(rect, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let sprite = if !response.sense.interactive() {
                self.button.disable_sprite.as_ref()
            } else if response.is_pointer_button_down_on() {
                self.button.down_sprite.as_ref()
            } else if response.hovered() || response.has_focus() {
                self.button.over_sprite.as_ref()
            } else {
                None
            }
            .or(self.button.normal_sprite.as_ref());

            if let Some(sprite) = sprite {
                draw_loaded_sprite(ui, rect.min, sprite);
            }
        }

        response
    }
}

struct DrawCheckbox<'a, 'b> {
    checkbox: &'a Checkbox,
    checked: &'b mut bool,
}

impl<'a, 'b> egui::Widget for DrawCheckbox<'a, 'b> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let rect = self.checkbox.widget_rect(ui.min_rect().min);
        let mut response = ui.allocate_rect(rect, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            if response.clicked() {
                *self.checked = !*self.checked;
                response.mark_changed();
            }

            let sprite = if *self.checked {
                self.checkbox.checked_sprite.as_ref()
            } else {
                self.checkbox.unchecked_sprite.as_ref()
            };

            if let Some(sprite) = sprite {
                draw_loaded_sprite(ui, rect.min, sprite);
            }
        }

        response
    }
}

struct DrawGauge<'a> {
    gauge: &'a Gauge,
    value: f32,
    text: &'a str,
}

impl<'a> egui::Widget for DrawGauge<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let rect = self.gauge.widget_rect(ui.min_rect().min);
        let response = ui.allocate_rect(rect, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            if let Some(sprite) = self.gauge.background_sprite.as_ref() {
                draw_loaded_sprite_stretched(ui, rect, sprite);
            }

            if self.value * self.gauge.width > 0.5 {
                if let Some(sprite) = self.gauge.foreground_sprite.as_ref() {
                    let mut stretched_rect = rect;
                    stretched_rect.set_width(self.value * self.gauge.width);
                    draw_loaded_sprite_stretched(ui, stretched_rect, sprite);
                }
            }

            if !self.text.is_empty() {
                ui.put(
                    rect.translate(egui::vec2(1.0, 1.0)),
                    egui::Label::new(egui::RichText::new(self.text).color(egui::Color32::BLACK)),
                );

                ui.put(rect, egui::Label::new(self.text));
            }
        }

        response
    }
}

struct DrawListbox<'a> {
    listbox: &'a Listbox,
}

impl<'a> egui::Widget for DrawListbox<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let rect = self.listbox.widget_rect(ui.min_rect().min);
        let response = ui.allocate_rect(rect, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            // TODO: Implement list box... somehow...
        }

        response
    }
}

struct DrawSprite<'a> {
    sprite: &'a Sprite,
}

impl<'a> egui::Widget for DrawSprite<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let rect = self.sprite.widget_rect(ui.min_rect().min);
        let response = ui.allocate_rect(rect, egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            if let Some(sprite) = self.sprite.sprite.as_ref() {
                draw_loaded_sprite(ui, rect.min, sprite);
            }
        }

        response
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
                draw_loaded_sprite(ui, rect.min, sprite);
            }
        }

        response
    }
}

struct DrawTextbox<'a, 'b> {
    textbox: &'a Textbox,
    buffer: &'b mut String,
}

impl<'a, 'b> egui::Widget for DrawTextbox<'a, 'b> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let rect = self.textbox.widget_rect(ui.min_rect().min);
        ui.put(
            rect,
            egui::TextEdit::singleline(self.buffer)
                .frame(false)
                .margin(egui::vec2(0.0, 0.0))
                .password(self.textbox.password != 0),
        )
    }
}

#[derive(Default)]
pub struct DialogDataBindings<'a> {
    pub checked: &'a mut [(i32, &'a mut bool)],
    pub text: &'a mut [(i32, &'a mut String)],
    pub gauge: &'a mut [(i32, &'a f32, &'a str)],
    pub tabs: &'a mut [(i32, &'a mut i32)],
    pub response: &'a mut [(i32, &'a mut Option<egui::Response>)],
    pub visible: &'a mut [(i32, bool)],
}

impl<'a> DialogDataBindings<'a> {
    pub fn set_response(&mut self, id: i32, response: egui::Response) {
        if let Some((_, out)) = self.response.iter_mut().find(|(x, _)| *x == id) {
            **out = Some(response);
        }
    }

    pub fn tab(&mut self, id: i32) -> Option<&mut i32> {
        self.tabs
            .iter_mut()
            .find(|(x, _)| *x == id)
            .map(|(_, buffer)| &mut **buffer)
    }

    pub fn text(&mut self, id: i32) -> Option<&mut String> {
        self.text
            .iter_mut()
            .find(|(x, _)| *x == id)
            .map(|(_, buffer)| &mut **buffer)
    }

    pub fn visible(&self, id: i32) -> bool {
        self.visible
            .iter()
            .find(|(x, _)| *x == id)
            .map_or(true, |(_, visible)| *visible)
    }
}

fn draw_widgets<'a>(ui: &mut egui::Ui, widgets: &[Widget], bindings: &mut DialogDataBindings<'a>) {
    for element in widgets.iter() {
        match element {
            Widget::Textbox(textbox) => {
                if !bindings.visible(textbox.id) {
                    continue;
                }

                let mut unbound_buffer = format!("<{} unbound>", textbox.id);
                let buffer = bindings.text(textbox.id).unwrap_or(&mut unbound_buffer);

                let response = egui::Widget::ui(DrawTextbox { textbox, buffer }, ui);
                bindings.set_response(textbox.id, response);
            }
            Widget::Sprite(sprite) => {
                if !bindings.visible(sprite.id) {
                    continue;
                }

                let response = egui::Widget::ui(DrawSprite { sprite }, ui);
                bindings.set_response(sprite.id, response);
            }
            Widget::Button(button) => {
                if !bindings.visible(button.id) {
                    continue;
                }

                let response = egui::Widget::ui(DrawButton { button }, ui);
                bindings.set_response(button.id, response);
            }
            Widget::Checkbox(checkbox) => {
                if !bindings.visible(checkbox.id) {
                    continue;
                }

                let mut unbound_checked = false;
                let checked = bindings
                    .checked
                    .iter_mut()
                    .find(|(id, _)| *id == checkbox.id)
                    .map(|(_, buffer)| &mut **buffer)
                    .unwrap_or(&mut unbound_checked);

                let response = egui::Widget::ui(DrawCheckbox { checkbox, checked }, ui);

                bindings.set_response(checkbox.id, response);
            }
            Widget::Gauge(gauge) => {
                if !bindings.visible(gauge.id) {
                    continue;
                }

                let (value, text) = bindings
                    .gauge
                    .iter()
                    .find(|(id, _, _)| *id == gauge.id)
                    .map_or((0.5, ""), |(_, value, text)| (**value, &**text));

                let response = egui::Widget::ui(DrawGauge { gauge, value, text }, ui);
                bindings.set_response(gauge.id, response);
            }
            Widget::Listbox(listbox) => {
                if !bindings.visible(listbox.id) {
                    continue;
                }

                let response = egui::Widget::ui(DrawListbox { listbox }, ui);
                bindings.set_response(listbox.id, response);
            }
            Widget::Pane(pane) => {
                if !bindings.visible(pane.id) {
                    continue;
                }

                ui.allocate_ui_at_rect(pane.widget_rect(ui.min_rect().min), |ui| {
                    draw_widgets(ui, &pane.widgets, bindings)
                });
            }
            Widget::TabbedPane(tabbed_pane) => {
                if !bindings.visible(tabbed_pane.id) || tabbed_pane.tabs.is_empty() {
                    continue;
                }

                let mut rect = ui.min_rect();
                rect.min.x += tabbed_pane.x;
                rect.min.y += tabbed_pane.y;

                ui.allocate_ui_at_rect(rect, |ui| {
                    let mut current_tab = bindings
                        .tab(tabbed_pane.id)
                        .map(|x| *x)
                        .unwrap_or(tabbed_pane.tabs[0].id);

                    // First draw the buttons
                    for tab in tabbed_pane.tabs.iter() {
                        if let Some(tab_button) = tab.tab_button() {
                            let response = egui::Widget::ui(
                                DrawTabButton {
                                    tab_button,
                                    selected: current_tab == tab.id,
                                },
                                ui,
                            );
                            if response.clicked() {
                                current_tab = tab.id;
                            }
                            bindings.set_response(tab_button.id, response);
                        }
                    }

                    // Update current tab
                    if let Some(tab_id) = bindings.tab(tabbed_pane.id) {
                        *tab_id = current_tab;
                    }

                    // Next draw active tab
                    for tab in tabbed_pane.tabs.iter() {
                        if tab.id != current_tab {
                            continue;
                        }

                        draw_widgets(ui, &tab.widgets, bindings)
                    }
                });
            }
            Widget::TabButton(_) => {} // These are drawn by Widget::TabbedPane
            Widget::Caption(_) => {}
        }
    }
}

pub fn draw_dialog<'a, R>(
    ui: &mut egui::Ui,
    dialog: &Dialog,
    mut bindings: DialogDataBindings<'a>,
    add_contents: impl FnOnce(&mut egui::Ui, &mut DialogDataBindings<'a>) -> R,
) {
    let style = ui.style_mut();
    style.visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::WHITE;
    style.spacing.item_spacing = egui::Vec2::ZERO;
    style.spacing.window_margin = egui::style::Margin::same(0.0);

    draw_widgets(ui, &dialog.widgets, &mut bindings);

    add_contents(ui, &mut bindings);
}
