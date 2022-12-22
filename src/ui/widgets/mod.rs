use bevy_egui::egui;
use serde::Deserialize;

use crate::resources::UiResources;

macro_rules! widget_to_rect {
    ( $x:ident ) => {
        impl $x {
            #[allow(dead_code)]
            pub fn widget_rect(&self, min: egui::Pos2) -> egui::Rect {
                egui::Rect::from_min_size(
                    min + egui::vec2(self.x, self.y) + egui::vec2(self.offset_x, self.offset_y),
                    egui::vec2(self.width, self.height),
                )
            }
        }
    };
}

mod button;
mod caption;
mod checkbox;
mod data_bindings;
mod dialog;
mod draw;
mod editbox;
mod gauge;
mod image;
mod listbox;
mod pane;
mod radio_box;
mod radio_button;
mod scrollbar;
mod scrollbox;
mod skill;
mod tab;
mod tab_button;
mod tabbed_pane;
mod table;
mod zlistbox;

pub use self::image::Image;
pub use button::Button;
pub use caption::Caption;
pub use checkbox::Checkbox;
pub use data_bindings::DataBindings;
pub use dialog::Dialog;
pub use draw::DrawText;
pub use editbox::Editbox;
pub use gauge::Gauge;
pub use listbox::Listbox;
pub use pane::Pane;
pub use radio_box::RadioBox;
pub use radio_button::RadioButton;
pub use scrollbar::Scrollbar;
pub use scrollbox::Scrollbox;
pub use skill::Skill;
pub use tab::Tab;
pub use tab_button::TabButton;
pub use tabbed_pane::TabbedPane;
pub use table::Table;
pub use zlistbox::ZListbox;

pub trait DrawWidget {
    fn draw_widget(&self, ui: &mut egui::Ui, bindings: &mut DataBindings);
}

pub trait GetWidget {
    fn get_widget(&self, id: i32) -> Option<&Widget>;
    fn get_widget_mut(&mut self, id: i32) -> Option<&mut Widget>;
}

pub trait LoadWidget {
    fn load_widget(&mut self, ui_resources: &UiResources);
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Deserialize)]
pub enum Widget {
    #[serde(rename = "BUTTON")]
    #[serde(alias = "BUTTONOK")]
    #[serde(alias = "BUTTONCANCEL")]
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
    Editbox(Editbox),
    #[serde(rename = "PANE")]
    Pane(Pane),
    #[serde(rename = "RADIOBOX")]
    RadioBox(RadioBox),
    #[serde(rename = "RADIOBUTTON")]
    RadioButton(RadioButton),
    #[serde(rename = "SCROLLBAR")]
    Scrollbar(Scrollbar),
    #[serde(rename = "SKILL")]
    Skill(Skill),
    #[serde(rename = "IMAGE")]
    #[serde(alias = "IMAGETOP")]
    #[serde(alias = "IMAGEMIDDLE")]
    #[serde(alias = "IMAGEBOTTOM")]
    Image(Image),
    #[serde(rename = "TABLE")]
    Table(Table),
    #[serde(rename = "TABBUTTON")]
    TabButton(TabButton),
    #[serde(rename = "TABBEDPANE")]
    TabbedPane(TabbedPane),
    #[serde(rename = "ZLISTBOX")]
    ZListbox(ZListbox),
    #[serde(other)]
    Unknown,
}

impl Widget {
    pub fn id(&self) -> i32 {
        match self {
            Widget::Button(x) => x.id,
            Widget::Caption(x) => x.id,
            Widget::Checkbox(x) => x.id,
            Widget::Gauge(x) => x.id,
            Widget::Listbox(x) => x.id,
            Widget::Editbox(x) => x.id,
            Widget::Pane(x) => x.id,
            Widget::RadioBox(x) => x.id,
            Widget::RadioButton(x) => x.id,
            Widget::Scrollbar(x) => x.id,
            Widget::Skill(x) => (x.id + x.level) as i32,
            Widget::Image(x) => x.id,
            Widget::Table(x) => x.id,
            Widget::TabButton(x) => x.id,
            Widget::TabbedPane(x) => x.id,
            Widget::ZListbox(x) => x.id,
            Widget::Unknown => panic!("Use of unknown widget"),
        }
    }
}

impl DrawWidget for Widget {
    fn draw_widget(&self, ui: &mut egui::Ui, bindings: &mut DataBindings) {
        match self {
            Widget::Button(this) => this.draw_widget(ui, bindings),
            Widget::Caption(this) => this.draw_widget(ui, bindings),
            Widget::Checkbox(this) => this.draw_widget(ui, bindings),
            Widget::Gauge(this) => this.draw_widget(ui, bindings),
            Widget::Listbox(this) => this.draw_widget(ui, bindings),
            Widget::Editbox(this) => this.draw_widget(ui, bindings),
            Widget::Pane(this) => this.draw_widget(ui, bindings),
            Widget::RadioBox(this) => this.draw_widget(ui, bindings),
            Widget::RadioButton(this) => this.draw_widget(ui, bindings),
            Widget::Scrollbar(this) => this.draw_widget(ui, bindings),
            Widget::Skill(this) => this.draw_widget(ui, bindings),
            Widget::Image(this) => this.draw_widget(ui, bindings),
            Widget::Table(this) => this.draw_widget(ui, bindings),
            Widget::TabButton(this) => this.draw_widget(ui, bindings),
            Widget::TabbedPane(this) => this.draw_widget(ui, bindings),
            Widget::ZListbox(this) => this.draw_widget(ui, bindings),
            Widget::Unknown => {}
        }
    }
}

impl LoadWidget for Widget {
    fn load_widget(&mut self, ui_resources: &UiResources) {
        match self {
            Widget::Button(this) => this.load_widget(ui_resources),
            Widget::Caption(this) => this.load_widget(ui_resources),
            Widget::Checkbox(this) => this.load_widget(ui_resources),
            Widget::Gauge(this) => this.load_widget(ui_resources),
            Widget::Listbox(this) => this.load_widget(ui_resources),
            Widget::Editbox(this) => this.load_widget(ui_resources),
            Widget::Pane(this) => this.load_widget(ui_resources),
            Widget::RadioBox(this) => this.load_widget(ui_resources),
            Widget::RadioButton(this) => this.load_widget(ui_resources),
            Widget::Scrollbar(this) => this.load_widget(ui_resources),
            Widget::Skill(this) => this.load_widget(ui_resources),
            Widget::Image(this) => this.load_widget(ui_resources),
            Widget::Table(this) => this.load_widget(ui_resources),
            Widget::TabButton(this) => this.load_widget(ui_resources),
            Widget::TabbedPane(this) => this.load_widget(ui_resources),
            Widget::ZListbox(this) => this.load_widget(ui_resources),
            Widget::Unknown => {}
        }
    }
}

impl DrawWidget for Vec<Widget> {
    fn draw_widget(&self, ui: &mut egui::Ui, bindings: &mut DataBindings) {
        for widget in self.iter() {
            widget.draw_widget(ui, bindings);
        }
    }
}

impl LoadWidget for Vec<Widget> {
    fn load_widget(&mut self, ui_resources: &UiResources) {
        for widget in self.iter_mut() {
            widget.load_widget(ui_resources);
        }
    }
}

impl GetWidget for Vec<Widget> {
    fn get_widget(&self, id: i32) -> Option<&Widget> {
        for widget in self.iter() {
            if widget.id() == id {
                return Some(widget);
            }

            match widget {
                Widget::Pane(pane) => {
                    if let Some(widget) = pane.widgets.get_widget(id) {
                        return Some(widget);
                    }
                }
                Widget::TabbedPane(tabbed_pane) => {
                    for tab in tabbed_pane.tabs.iter() {
                        if let Some(widget) = tab.widgets.get_widget(id) {
                            return Some(widget);
                        }
                    }
                }
                Widget::Skill(skill) => {
                    if let Some(widget) = skill.widgets.get_widget(id) {
                        return Some(widget);
                    }
                }
                Widget::Button(_)
                | Widget::Caption(_)
                | Widget::Checkbox(_)
                | Widget::Gauge(_)
                | Widget::Listbox(_)
                | Widget::Editbox(_)
                | Widget::RadioBox(_)
                | Widget::RadioButton(_)
                | Widget::Image(_)
                | Widget::Table(_)
                | Widget::TabButton(_)
                | Widget::ZListbox(_)
                | Widget::Scrollbar(_) => {
                    continue;
                }
                Widget::Unknown => panic!("Use of unknown widget"),
            }
        }

        None
    }

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
                Widget::Skill(skill) => {
                    if let Some(widget) = skill.widgets.get_widget_mut(id) {
                        return Some(widget);
                    }
                }
                Widget::Button(_)
                | Widget::Caption(_)
                | Widget::Checkbox(_)
                | Widget::Gauge(_)
                | Widget::Listbox(_)
                | Widget::Editbox(_)
                | Widget::RadioBox(_)
                | Widget::RadioButton(_)
                | Widget::Image(_)
                | Widget::Table(_)
                | Widget::TabButton(_)
                | Widget::ZListbox(_)
                | Widget::Scrollbar(_) => {
                    continue;
                }
                Widget::Unknown => panic!("Use of unknown widget"),
            }
        }

        None
    }
}
