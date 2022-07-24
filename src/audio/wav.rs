use bevy::asset::{AssetLoader, BoxedFuture, LoadContext, LoadedAsset};
use hound::WavReader;

use crate::audio::audio_source::AudioSource;

use super::audio_source::StreamingAudioSource;

#[derive(Default)]
pub struct WavLoader;

impl AssetLoader for WavLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, anyhow::Result<()>> {
        load_context.set_default_asset(LoadedAsset::new(AudioSource {
            bytes: bytes.into(),
            create_streaming_source_fn: |audio_source| {
                WavAudioSource::new(audio_source)
                    .map(|source| Box::new(source) as Box<dyn StreamingAudioSource + Send + Sync>)
            },
        }));
        Box::pin(async move { Ok(()) })
    }

    fn extensions(&self) -> &[&str] {
        &["wav"]
    }
}

struct WavAudioSource {
    reader: WavReader<std::io::Cursor<AudioSource>>,
}

impl WavAudioSource {
    pub fn new(audio_source: &AudioSource) -> Result<Self, anyhow::Error> {
        Ok(Self {
            reader: WavReader::new(std::io::Cursor::new(audio_source.clone()))?,
        })
    }
}

impl StreamingAudioSource for WavAudioSource {
    fn channel_count(&self) -> u32 {
        self.reader.spec().channels as u32
    }

    fn sample_rate(&self) -> u32 {
        self.reader.spec().sample_rate as u32
    }

    fn rewind(&mut self) {
        self.reader.seek(0).ok();
    }

    fn read_packet(&mut self) -> Vec<f32> {
        let hound::WavSpec {
            bits_per_sample,
            sample_format,
            ..
        } = self.reader.spec();

        let samples: Result<Vec<f32>, _> = match sample_format {
            hound::SampleFormat::Int => {
                let max_value = 2_u32.pow(bits_per_sample as u32 - 1) - 1;
                self.reader
                    .samples::<i32>()
                    .map(|sample| sample.map(|sample| sample as f32 / max_value as f32))
                    .collect()
            }
            hound::SampleFormat::Float => self.reader.samples::<f32>().collect(),
        };

        samples.unwrap_or_else(|_| Vec::default())
    }
}
