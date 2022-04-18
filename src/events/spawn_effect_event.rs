use bevy::prelude::{Entity, Transform};
use rose_data::EffectFileId;
use rose_file_readers::VfsPathBuf;

pub enum SpawnEffect {
    FileId(EffectFileId),
    Path(VfsPathBuf),
}

pub struct SpawnEffectData {
    pub effect: SpawnEffect,
    pub manual_despawn: bool,
}

impl SpawnEffectData {
    pub fn with_path(path: VfsPathBuf) -> Self {
        Self {
            effect: SpawnEffect::Path(path),
            manual_despawn: false,
        }
    }

    pub fn with_file_id(effect_file_id: EffectFileId) -> Self {
        Self {
            effect: SpawnEffect::FileId(effect_file_id),
            manual_despawn: false,
        }
    }

    pub fn manual_despawn(mut self, manual_despawn: bool) -> Self {
        self.manual_despawn = manual_despawn;
        self
    }
}

#[allow(dead_code)]
pub enum SpawnEffectEvent {
    // Spawns an effect at the same location than Entity, but does not attach to entity
    AtEntity(Entity, SpawnEffectData),

    // Adds the components for effect to the given entity
    InEntity(Entity, SpawnEffectData),

    // Spawns an effect attached to Entity, optionally attached to dummy bone
    OnEntity(Entity, Option<usize>, SpawnEffectData),

    // Spawns an effect with the given transform
    WithTransform(Transform, SpawnEffectData),
}
