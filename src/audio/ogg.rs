use bevy::asset::{AssetLoader, BoxedFuture, LoadContext, LoadedAsset};
use lewton::{
    audio::AudioReadError, inside_ogg::OggStreamReader, samples::InterleavedSamples, VorbisError,
};

use crate::audio::audio_source::AudioSource;

use super::audio_source::StreamingAudioSource;

#[derive(Default)]
pub struct OggLoader;

impl AssetLoader for OggLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, anyhow::Result<()>> {
        load_context.set_default_asset(LoadedAsset::new(AudioSource {
            bytes: bytes.into(),
            decoded: None,
            create_streaming_source_fn: |audio_source| {
                OggAudioSource::new(audio_source)
                    .map(|source| Box::new(source) as Box<dyn StreamingAudioSource + Send + Sync>)
            },
        }));
        Box::pin(async move { Ok(()) })
    }

    fn extensions(&self) -> &[&str] {
        &["ogg"]
    }
}

struct OggAudioSource {
    reader: OggStreamReader<std::io::Cursor<AudioSource>>,
}

impl OggAudioSource {
    pub fn new(audio_source: &AudioSource) -> Result<Self, anyhow::Error> {
        Ok(Self {
            reader: OggStreamReader::new(std::io::Cursor::new(audio_source.clone()))?,
        })
    }
}

impl StreamingAudioSource for OggAudioSource {
    fn channel_count(&self) -> u32 {
        self.reader.ident_hdr.audio_channels as u32
    }

    fn sample_rate(&self) -> u32 {
        self.reader.ident_hdr.audio_sample_rate
    }

    fn rewind(&mut self) {
        // Seek back to start
        self.reader.seek_absgp_pg(0).ok();
    }

    fn read_packet(&mut self) -> Vec<f32> {
        loop {
            match self
                .reader
                .read_dec_packet_generic::<InterleavedSamples<f32>>()
            {
                Ok(Some(packet)) => {
                    if !packet.samples.is_empty() {
                        return packet.samples;
                    }
                }
                Err(VorbisError::BadAudio(AudioReadError::AudioIsHeader)) => {
                    continue;
                }
                Ok(_) | Err(_) => break,
            }
        }

        Vec::default()
    }
}
