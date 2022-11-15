use bevy::prelude::{Entity, Resource};

#[derive(Resource, Default)]
pub struct DebugInspector {
    pub enable_picking: bool,
    pub entity: Option<Entity>,
}
