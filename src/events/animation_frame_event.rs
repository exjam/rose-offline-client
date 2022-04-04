use bevy::prelude::Entity;
use std::num::NonZeroU16;

pub struct AnimationFrameEvent {
    pub entity: Entity,
    pub event_id: NonZeroU16,
}

impl AnimationFrameEvent {
    pub fn new(entity: Entity, event_id: NonZeroU16) -> Self {
        Self { entity, event_id }
    }
}
