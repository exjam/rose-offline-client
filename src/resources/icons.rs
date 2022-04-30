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
    pub window_icons_image: (Handle<Image>, egui::TextureId),
    pub minimap_player_icon: (Handle<Image>, egui::TextureId),
}

impl Icons {
    pub fn get_window_icon_character_info(&self) -> (egui::TextureId, egui::Rect) {
        (
            self.window_icons_image.1,
            egui::Rect::from_min_max(
                egui::Pos2::new(68.5 / 512.0, 187.5 / 512.0),
                egui::Pos2::new(107.5 / 512.0, 226.5 / 512.0),
            ),
        )
    }

    pub fn get_window_icon_inventory(&self) -> (egui::TextureId, egui::Rect) {
        (
            self.window_icons_image.1,
            egui::Rect::from_min_max(
                egui::Pos2::new(109.5 / 512.0, 187.5 / 512.0),
                egui::Pos2::new(148.5 / 512.0, 226.5 / 512.0),
            ),
        )
    }

    pub fn get_window_icon_skills(&self) -> (egui::TextureId, egui::Rect) {
        (
            self.window_icons_image.1,
            egui::Rect::from_min_max(
                egui::Pos2::new(150.5 / 512.0, 187.5 / 512.0),
                egui::Pos2::new(189.5 / 512.0, 226.5 / 512.0),
            ),
        )
    }

    pub fn get_window_icon_quests(&self) -> (egui::TextureId, egui::Rect) {
        (
            self.window_icons_image.1,
            egui::Rect::from_min_max(
                egui::Pos2::new(191.5 / 512.0, 187.5 / 512.0),
                egui::Pos2::new(230.5 / 512.0, 226.5 / 512.0),
            ),
        )
    }

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
