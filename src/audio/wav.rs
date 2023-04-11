use std::sync::Arc;

use bevy::asset::{AssetLoader, BoxedFuture, LoadContext, LoadedAsset};
use hound::WavReader;

use crate::audio::audio_source::AudioSource;

use super::audio_source::AudioSourceDecoded;

#[derive(Default)]
pub struct WavLoader;

impl AssetLoader for WavLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, anyhow::Result<()>> {
        let mut reader = match WavReader::new(std::io::Cursor::new(bytes)) {
            Ok(reader) => reader,
            Err(error) => {
                return Box::pin(async move { Err(error.into()) });
            }
        };

        let hound::WavSpec {
            bits_per_sample,
            sample_format,
            sample_rate,
            channels,
        } = reader.spec();

        let samples: Result<Vec<f32>, _> = match sample_format {
            hound::SampleFormat::Int => {
                let max_value = 2_u32.pow(bits_per_sample as u32 - 1) - 1;
                reader
                    .samples::<i32>()
                    .map(|sample| sample.map(|sample| sample as f32 / max_value as f32))
                    .collect()
            }
            hound::SampleFormat::Float => reader.samples::<f32>().collect(),
        };

        let samples = match samples {
            Ok(samples) => samples,
            Err(error) => {
                return Box::pin(async move { Err(error.into()) });
            }
        };

        load_context.set_default_asset(LoadedAsset::new(AudioSource {
            bytes: Arc::new([]),
            decoded: Some(Arc::new(AudioSourceDecoded {
                samples,
                channel_count: channels as u32,
                sample_rate,
            })),
            create_streaming_source_fn: |_| Err(anyhow::anyhow!("Unsupported")),
        }));
        Box::pin(async move { Ok(()) })
    }

    fn extensions(&self) -> &[&str] {
        &["wav"]
    }
}
