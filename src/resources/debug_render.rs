use bevy::prelude::{Color, Entity, Resource};

const DEBUG_RENDER_COLOR_LIST: [Color; 8] = [
    Color::RED,
    Color::GREEN,
    Color::BLUE,
    Color::YELLOW,
    Color::CYAN,
    Color::FUCHSIA,
    Color::WHITE,
    Color::BLACK,
];

#[derive(Default, Resource)]
pub struct DebugRenderConfig {
    pub colliders: bool,
    pub skeleton: bool,
    pub bone_up: bool,
    pub directional_light_frustum: bool,
    pub directional_light_frustum_freeze: bool,
}

impl DebugRenderConfig {
    pub fn color_for_entity(&self, entity: Entity) -> Color {
        DEBUG_RENDER_COLOR_LIST[entity.index() as usize % DEBUG_RENDER_COLOR_LIST.len()]
    }
}
