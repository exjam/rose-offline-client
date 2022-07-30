use std::ops::Range;

use bevy_egui::egui;

#[derive(Default)]
pub struct DataBindings<'a> {
    pub visible: &'a mut [(i32, bool)],
    pub checked: &'a mut [(i32, &'a mut bool)],
    pub text: &'a mut [(i32, &'a mut String)],
    pub gauge: &'a mut [(i32, &'a f32, &'a str)],
    pub tabs: &'a mut [(i32, &'a mut i32)],
    pub radio: &'a mut [(i32, &'a mut i32)],
    pub scroll: &'a mut [(i32, (&'a mut i32, Range<i32>))],
    pub response: &'a mut [(i32, &'a mut Option<egui::Response>)],
}

impl<'a> DataBindings<'a> {
    pub fn set_response(&mut self, id: i32, response: egui::Response) {
        if let Some((_, out)) = self.response.iter_mut().find(|(x, _)| *x == id) {
            **out = Some(response);
        }
    }

    pub fn get_radio(&mut self, id: i32) -> Option<&mut i32> {
        self.radio
            .iter_mut()
            .find(|(x, _)| *x == id)
            .map(|(_, buffer)| &mut **buffer)
    }

    pub fn get_scroll(&mut self, id: i32) -> Option<(&mut i32, Range<i32>)> {
        self.scroll
            .iter_mut()
            .find(|(x, _)| *x == id)
            .map(|(_, (current, range))| (&mut **current, range.clone()))
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

    pub fn get_visible(&self, id: i32) -> bool {
        self.visible
            .iter()
            .find(|(x, _)| *x == id)
            .map_or(true, |(_, visible)| *visible)
    }
}
