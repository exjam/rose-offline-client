use enum_map::EnumMap;

use crate::{audio::SoundGain, components::SoundCategory};

pub struct SoundSettings {
    pub enabled: bool,
    pub global_gain: f32,
    pub gains: EnumMap<SoundCategory, f32>,
}

impl SoundSettings {
    pub fn gain(&self, category: SoundCategory) -> SoundGain {
        if self.enabled {
            SoundGain::Ratio(self.global_gain * self.gains[category])
        } else {
            SoundGain::Ratio(0.0)
        }
    }
}
