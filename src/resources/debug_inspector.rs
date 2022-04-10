use bevy::prelude::Entity;

#[derive(Default)]
pub struct DebugInspector {
    pub enable_picking: bool,
    pub entity: Option<Entity>,
}
