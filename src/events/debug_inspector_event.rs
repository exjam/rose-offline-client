use bevy::prelude::Entity;

pub enum DebugInspectorEvent {
    Show,
    Hide,
    InspectEntity(Entity),
}
