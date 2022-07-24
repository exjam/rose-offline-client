use std::sync::Arc;

use bevy::reflect::TypeUuid;

pub trait StreamingAudioSource {
    fn channel_count(&self) -> u32;
    fn sample_rate(&self) -> u32;
    fn rewind(&mut self);
    fn read_packet(&mut self) -> Vec<f32>;
}

#[derive(Clone, TypeUuid)]
#[uuid = "f40c2d6a-d2ad-42cc-8f86-0147d3ddd68c"]
pub struct AudioSource {
    pub bytes: Arc<[u8]>,
    pub create_streaming_source_fn:
        fn(&Self) -> Result<Box<dyn StreamingAudioSource + Send + Sync>, anyhow::Error>,
}

impl AudioSource {
    pub fn create_streaming_source(
        &self,
    ) -> Result<Box<dyn StreamingAudioSource + Send + Sync>, anyhow::Error> {
        (self.create_streaming_source_fn)(self)
    }
}

impl AsRef<[u8]> for AudioSource {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}
