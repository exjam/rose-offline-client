use super::audio_source::{AudioSource, StreamingAudioSource};

pub struct StreamingSound {
    source: Box<dyn StreamingAudioSource + Send + Sync>,
    buffer: Vec<f32>,
}

impl StreamingSound {
    pub fn new(audio_source: &AudioSource) -> Self {
        Self {
            source: audio_source.create_streaming_source().unwrap(),
            buffer: Vec::with_capacity(1024),
        }
    }

    pub fn channel_count(&self) -> u32 {
        self.source.channel_count()
    }

    pub fn sample_rate(&self) -> u32 {
        self.source.sample_rate()
    }

    pub fn fill_mono(&mut self, stream: &mut oddio::StreamControl<f32>, repeating: bool) -> bool {
        if !self.buffer.is_empty() {
            let samples_read = stream.write(&self.buffer);
            self.buffer.drain(0..samples_read);
        }

        if self.buffer.is_empty() {
            let mut did_repeat = false;

            loop {
                let packet = self.source.read_packet();
                if packet.is_empty() {
                    if repeating {
                        if !did_repeat {
                            self.source.rewind();
                            did_repeat = true;
                            continue;
                        } else {
                            return false; // Encountered an error
                        }
                    } else {
                        return false; // Reached end of stream
                    }
                }

                let samples_read = stream.write(&packet);
                if samples_read == packet.len() {
                    continue;
                } else {
                    self.buffer.extend_from_slice(&packet[samples_read..]);
                    break;
                }
            }
        }

        true
    }

    pub fn fill_stereo(
        &mut self,
        stream: &mut oddio::StreamControl<[f32; 2]>,
        repeating: bool,
    ) -> bool {
        if !self.buffer.is_empty() {
            let samples_read = stream.write(oddio::frame_stereo(&mut self.buffer)) * 2;
            self.buffer.drain(0..samples_read);
        }

        if self.buffer.is_empty() {
            let mut did_repeat = false;

            loop {
                let mut packet = self.source.as_mut().read_packet();
                if packet.is_empty() {
                    if repeating {
                        if !did_repeat {
                            self.source.rewind();
                            did_repeat = true;
                            continue;
                        } else {
                            return false; // Encountered an error
                        }
                    } else {
                        return false; // Reached end of stream
                    }
                }

                let samples_read = stream.write(oddio::frame_stereo(&mut packet)) * 2;
                if samples_read == packet.len() {
                    continue;
                } else {
                    self.buffer.extend_from_slice(&packet[samples_read..]);
                    break;
                }
            }
        }

        true
    }
}
