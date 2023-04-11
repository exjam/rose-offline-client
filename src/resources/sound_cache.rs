use std::sync::RwLock;

use bevy::prelude::{AssetServer, Handle, Resource};
use rose_data::{SoundData, SoundId};

use crate::audio::AudioSource;

#[derive(Resource)]
pub struct SoundCache {
    pub cached_sounds: RwLock<Vec<Option<Handle<AudioSource>>>>,
}

impl SoundCache {
    pub fn new(size: usize) -> Self {
        Self {
            cached_sounds: RwLock::new(vec![None; size]),
        }
    }

    pub fn get(&self, id: SoundId) -> Option<Handle<AudioSource>> {
        self.cached_sounds
            .read()
            .unwrap()
            .get(id.get() as usize)
            .and_then(|x| x.clone())
    }

    pub fn load(&self, sound_data: &SoundData, asset_server: &AssetServer) -> Handle<AudioSource> {
        if let Some(cached) = self.get(sound_data.id) {
            return cached;
        }

        let handle = asset_server.load(sound_data.path.path());
        self.set(sound_data.id, handle.clone());
        handle
    }

    pub fn set(&self, id: SoundId, handle: Handle<AudioSource>) {
        if let Some(cache) = self
            .cached_sounds
            .write()
            .unwrap()
            .get_mut(id.get() as usize)
        {
            *cache = Some(handle);
        }
    }

    pub fn clear(&self) {
        self.cached_sounds.write().unwrap().fill(None);
    }
}
