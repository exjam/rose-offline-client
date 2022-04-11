use bevy::prelude::Entity;

use rose_data::AnimationEventFlags;

pub struct AnimationFrameEvent {
    pub entity: Entity,
    pub flags: AnimationEventFlags,
}

impl AnimationFrameEvent {
    pub fn new(entity: Entity, flags: AnimationEventFlags) -> Self {
        Self { entity, flags }
    }
}
