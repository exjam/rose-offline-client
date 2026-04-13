use crate::{
    audio::{AudioSource, GlobalSound},
    components::SoundCategory,
    resources::{CurrentZone, GameData, ZoneTime, ZoneTimeState},
    Config,
};
use bevy::prelude::{AssetServer, Commands, Entity, Handle, Local, Res};
use rose_data::ZoneId;

#[derive(Default)]
pub enum BackgroundMusicState {
    #[default]
    None,
    PlayingDay,
    PlayingNight,
}

#[derive(Default)]
pub struct BackgroundMusic {
    pub zone: Option<ZoneId>,
    pub entity: Option<Entity>,
    pub day_audio_source: Option<Handle<AudioSource>>,
    pub night_audio_source: Option<Handle<AudioSource>>,
    pub state: BackgroundMusicState,
}

pub fn background_music_system(
    mut commands: Commands,
    mut background_music: Local<BackgroundMusic>,
    asset_server: Res<AssetServer>,
    current_zone: Option<Res<CurrentZone>>,
    game_data: Res<GameData>,
    zone_time: Res<ZoneTime>,
    config: Res<Config>,
) {
    let sound_settings = &config.sound;

    let current_zone = match current_zone {
        None => {
            if let Some(entity) = background_music.entity.take() {
                commands.entity(entity).despawn();
            }

            return;
        }
        Some(it) => it,
    };

    if background_music.zone != Some(current_zone.id) {
        if let Some(entity) = background_music.entity.take() {
            commands.entity(entity).despawn();
        }
        background_music.state = BackgroundMusicState::None;

        if let Some(zone_data) = game_data.zone_list.get_zone(current_zone.id) {
            background_music.day_audio_source = zone_data
                .background_music_day
                .as_ref()
                .map(|path| asset_server.load(path.path()));
            background_music.night_audio_source = zone_data
                .background_music_night
                .as_ref()
                .map(|path| asset_server.load(path.path()));
        } else {
            background_music.day_audio_source = None;
            background_music.night_audio_source = None;
        }

        background_music.zone = Some(current_zone.id);
    }

    if background_music.day_audio_source == background_music.night_audio_source
        && background_music.entity.is_some()
    {
        return;
    }

    match zone_time.state {
        ZoneTimeState::Morning | ZoneTimeState::Day => {
            match background_music.state {
                BackgroundMusicState::None | BackgroundMusicState::PlayingNight => {
                    // TODO: Should probably cross fade between the tracks
                    if let Some(entity) = background_music.entity.take() {
                        commands.entity(entity).despawn();
                    }

                    if let Some(audio_source) = background_music.day_audio_source.as_ref() {
                        background_music.entity = Some(
                            commands
                                .spawn((
                                    SoundCategory::BackgroundMusic,
                                    GlobalSound::new_repeating(audio_source.clone()),
                                    sound_settings.gain(SoundCategory::BackgroundMusic),
                                ))
                                .id(),
                        );
                    }

                    background_music.state = BackgroundMusicState::PlayingDay;
                }
                BackgroundMusicState::PlayingDay => {}
            }
        }
        ZoneTimeState::Evening | ZoneTimeState::Night => {
            match background_music.state {
                BackgroundMusicState::None | BackgroundMusicState::PlayingDay => {
                    // TODO: Should probably cross fade between the tracks
                    if let Some(entity) = background_music.entity.take() {
                        commands.entity(entity).despawn();
                    }

                    if let Some(audio_source) = background_music.night_audio_source.as_ref() {
                        background_music.entity = Some(
                            commands
                                .spawn((
                                    SoundCategory::BackgroundMusic,
                                    GlobalSound::new_repeating(audio_source.clone()),
                                    sound_settings.gain(SoundCategory::BackgroundMusic),
                                ))
                                .id(),
                        );
                    }

                    background_music.state = BackgroundMusicState::PlayingNight;
                }
                BackgroundMusicState::PlayingNight => {}
            }
        }
    }
}
