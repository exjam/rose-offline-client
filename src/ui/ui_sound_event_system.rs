use bevy::prelude::{AssetServer, Commands, Event, EventReader, Res, ResMut};

use rose_data::SoundId;

use crate::{
    audio::GlobalSound,
    components::SoundCategory,
    resources::{GameData, SoundCache, SoundSettings},
};

#[derive(Event)]
pub struct UiSoundEvent {
    sound_id: SoundId,
}

impl UiSoundEvent {
    pub fn new(sound_id: SoundId) -> Self {
        Self { sound_id }
    }
}

pub fn ui_sound_event_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut ui_sound_events: EventReader<UiSoundEvent>,
    sound_settings: Res<SoundSettings>,
    game_data: Res<GameData>,
    sound_cache: ResMut<SoundCache>,
) {
    for event in ui_sound_events.iter() {
        if let Some(sound_data) = game_data.sounds.get_sound(event.sound_id) {
            commands.spawn((
                SoundCategory::Ui,
                sound_settings.gain(SoundCategory::Ui),
                GlobalSound::new(sound_cache.load(sound_data, &asset_server)),
            ));
        }
    }
}
