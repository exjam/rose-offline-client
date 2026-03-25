use crate::{audio::SoundGain, components::SoundCategory};
use enum_map::enum_map;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct SoundVolumeConfig {
    pub global: f32,
    pub background_music: f32,
    pub player_footstep: f32,
    pub player_combat: f32,
    pub other_footstep: f32,
    pub other_combat: f32,
    pub npc_sounds: f32,
    pub ui_sounds: f32,
}

impl Default for SoundVolumeConfig {
    fn default() -> Self {
        Self {
            global: 0.6,
            background_music: 0.15,
            player_footstep: 0.9,
            player_combat: 1.0,
            other_footstep: 0.5,
            other_combat: 0.5,
            npc_sounds: 0.6,
            ui_sounds: 0.5,
        }
    }
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct SoundConfig {
    pub enabled: bool,
    pub volume: SoundVolumeConfig,
}

impl SoundConfig {
    pub fn gain(&self, category: SoundCategory) -> SoundGain {
        let gains = enum_map! {
            SoundCategory::BackgroundMusic => self.volume.background_music,
            SoundCategory::PlayerFootstep => self.volume.player_footstep,
            SoundCategory::PlayerCombat => self.volume.player_combat,
            SoundCategory::OtherFootstep => self.volume.other_footstep,
            SoundCategory::OtherCombat => self.volume.other_combat,
            SoundCategory::NpcSounds => self.volume.npc_sounds,
            SoundCategory::Ui => self.volume.ui_sounds,
        };

        if self.enabled {
            SoundGain::Ratio(self.volume.global * gains[category])
        } else {
            SoundGain::Ratio(0.0)
        }
    }
}

impl Default for SoundConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            volume: SoundVolumeConfig::default(),
        }
    }
}
