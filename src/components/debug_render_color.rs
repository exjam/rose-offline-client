use bevy::prelude::{Color, Component};
use rand::prelude::SliceRandom;

#[derive(Copy, Clone, Debug, Component)]
pub struct DebugRenderColor {
    pub color: Color,
}

const COLOR_LIST: [Color; 38] = [
    Color::ALICE_BLUE,
    Color::ANTIQUE_WHITE,
    Color::AQUAMARINE,
    Color::AZURE,
    Color::BEIGE,
    Color::BISQUE,
    Color::BLACK,
    Color::BLUE,
    Color::CRIMSON,
    Color::CYAN,
    Color::DARK_GRAY,
    Color::DARK_GREEN,
    Color::FUCHSIA,
    Color::GOLD,
    Color::GRAY,
    Color::GREEN,
    Color::INDIGO,
    Color::LIME_GREEN,
    Color::MAROON,
    Color::MIDNIGHT_BLUE,
    Color::NAVY,
    Color::NONE,
    Color::OLIVE,
    Color::ORANGE,
    Color::ORANGE_RED,
    Color::PINK,
    Color::PURPLE,
    Color::RED,
    Color::SALMON,
    Color::SEA_GREEN,
    Color::SILVER,
    Color::TEAL,
    Color::TOMATO,
    Color::TURQUOISE,
    Color::VIOLET,
    Color::WHITE,
    Color::YELLOW,
    Color::YELLOW_GREEN,
];

impl DebugRenderColor {
    pub fn random() -> Self {
        Self {
            color: *COLOR_LIST.choose(&mut rand::thread_rng()).unwrap(),
        }
    }
}
