use std::ops::Range;

use bevy::prelude::EventWriter;
use bevy_egui::egui;

use rose_data::SoundId;

use crate::ui::UiSoundEvent;

#[derive(Default)]
pub struct DataBindings<'a, 'w> {
    pub visible: &'a mut [(i32, bool)],
    pub checked: &'a mut [(i32, &'a mut bool)],
    pub enabled: &'a mut [(i32, bool)],
    pub text: &'a mut [(i32, &'a mut String)],
    pub gauge: &'a mut [(i32, &'a f32, &'a str)],
    pub label: &'a mut [(i32, &'a str)],
    pub listbox: &'a mut [(i32, (&'a mut i32, &'a dyn Fn(i32) -> Option<String>))],
    pub tabs: &'a mut [(i32, &'a mut i32)],
    pub radio: &'a mut [(i32, &'a mut i32)],
    pub scroll: &'a mut [(i32, (&'a mut i32, Range<i32>, i32))], // (current_scroll, scroll_range, num_visible)
    pub zlist: &'a mut [(
        i32,
        (
            &'a mut i32,
            &'a dyn Fn(&mut egui::Ui, i32, bool) -> egui::Response,
        ),
    )],
    pub table: &'a mut [(
        i32,
        (
            &'a mut i32,
            &'a dyn Fn(
                &mut egui::Ui,
                /* index */ i32,
                /* colum */ i32,
                /* row */ i32,
            ) -> egui::Response,
        ),
    )],
    pub response: &'a mut [(i32, &'a mut Option<egui::Response>)],
    pub sound_events: Option<&'a mut EventWriter<'w, UiSoundEvent>>,
}

impl<'a, 'w> DataBindings<'a, 'w> {
    pub fn emit_sound(&mut self, sound_id: SoundId) {
        if let Some(sound_events) = self.sound_events.as_mut() {
            sound_events.send(UiSoundEvent::new(sound_id));
        }
    }

    pub fn set_response(&mut self, id: i32, response: egui::Response) {
        if let Some((_, out)) = self.response.iter_mut().find(|(x, _)| *x == id) {
            **out = Some(response);
        }
    }

    pub fn get_label(&mut self, id: i32) -> Option<&str> {
        self.label
            .iter()
            .find(|(x, _)| *x == id)
            .map(|(_, text)| *text)
    }

    pub fn get_radio(&mut self, id: i32) -> Option<&mut i32> {
        self.radio
            .iter_mut()
            .find(|(x, _)| *x == id)
            .map(|(_, buffer)| &mut **buffer)
    }

    pub fn get_scroll(&mut self, id: i32) -> Option<(&mut i32, Range<i32>, i32)> {
        self.scroll
            .iter_mut()
            .find(|(x, _)| *x == id)
            .map(|(_, (current, range, visible))| (&mut **current, range.clone(), *visible))
    }

    pub fn get_tab(&mut self, pane_id: i32) -> Option<&mut i32> {
        self.tabs
            .iter_mut()
            .find(|(x, _)| *x == pane_id)
            .map(|(_, buffer)| &mut **buffer)
    }

    pub fn get_text(&mut self, id: i32) -> Option<&mut String> {
        self.text
            .iter_mut()
            .find(|(x, _)| *x == id)
            .map(|(_, buffer)| &mut **buffer)
    }

    pub fn get_enabled(&self, id: i32) -> bool {
        self.enabled
            .iter()
            .find(|(x, _)| *x == id)
            .map_or(true, |(_, visible)| *visible)
    }

    pub fn get_visible(&self, id: i32) -> bool {
        self.visible
            .iter()
            .find(|(x, _)| *x == id)
            .map_or(true, |(_, visible)| *visible)
    }

    pub fn get_table(
        &mut self,
        id: i32,
    ) -> Option<(
        &mut i32,
        &dyn Fn(&mut egui::Ui, i32, i32, i32) -> egui::Response,
    )> {
        self.table
            .iter_mut()
            .find(|(x, _)| *x == id)
            .map(|(_, (a, b))| (&mut **a, &**b))
    }

    pub fn get_list(&mut self, id: i32) -> Option<(&mut i32, &dyn Fn(i32) -> Option<String>)> {
        self.listbox
            .iter_mut()
            .find(|(x, _)| *x == id)
            .map(|(_, (a, b))| (&mut **a, &**b))
    }

    pub fn get_zlist(
        &mut self,
        id: i32,
    ) -> Option<(
        &mut i32,
        &dyn Fn(&mut egui::Ui, i32, bool) -> egui::Response,
    )> {
        self.zlist
            .iter_mut()
            .find(|(x, _)| *x == id)
            .map(|(_, (a, b))| (&mut **a, &**b))
    }

    pub fn get_zlist_selected_index(&mut self, id: i32) -> Option<i32> {
        self.zlist
            .iter_mut()
            .find(|(x, _)| *x == id)
            .map(|(_, (a, _))| **a)
    }
}
