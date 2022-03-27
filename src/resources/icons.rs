use bevy::prelude::{Handle, Image};
use bevy_egui::egui;

const ICONS_PER_PAGE: usize = 13 * 13;
const ICONS_PER_ROW: usize = 13;
const ICON_SIZE: usize = 40;
const ICON_PAGE_SIZE: usize = 512;
const ICON_UV_SIZE: f32 = ICON_SIZE as f32 / ICON_PAGE_SIZE as f32;

pub struct Icons {
    pub item_pages: Vec<(Handle<Image>, egui::TextureId)>,
    pub skill_pages: Vec<(Handle<Image>, egui::TextureId)>,
}

impl Icons {
    pub fn get_item_icon(&self, index: usize) -> Option<(egui::TextureId, egui::Rect)> {
        let page_index = index / ICONS_PER_PAGE;
        let (_, item_texture_id) = self.item_pages.get(page_index)?;

        let page_offset = index % ICONS_PER_PAGE;
        let row = page_offset / ICONS_PER_ROW;
        let column = page_offset % ICONS_PER_ROW;
        Some((
            *item_texture_id,
            egui::Rect::from_min_size(
                egui::pos2(column as f32 * ICON_UV_SIZE, row as f32 * ICON_UV_SIZE),
                egui::vec2(ICON_UV_SIZE, ICON_UV_SIZE),
            ),
        ))
    }

    #[allow(dead_code)]
    pub fn get_skill_icon(&self, index: usize) -> Option<(egui::TextureId, egui::Rect)> {
        let page_index = index / ICONS_PER_PAGE;
        let (_, item_texture_id) = self.skill_pages.get(page_index)?;

        let page_offset = index % ICONS_PER_PAGE;
        let row = page_offset / ICONS_PER_ROW;
        let column = page_offset % ICONS_PER_ROW;
        Some((
            *item_texture_id,
            egui::Rect::from_min_size(
                egui::pos2(column as f32 * ICON_UV_SIZE, row as f32 * ICON_UV_SIZE),
                egui::vec2(ICON_UV_SIZE, ICON_UV_SIZE),
            ),
        ))
    }
}
