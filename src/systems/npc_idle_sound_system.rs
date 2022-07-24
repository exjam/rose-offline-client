use bevy::{
    hierarchy::BuildChildren,
    prelude::{AssetServer, Commands, Component, Entity, GlobalTransform, Query, Res, Transform},
};
use rand::Rng;

use rose_data::SoundId;
use rose_game_common::components::Npc;

use crate::{
    audio::{SoundRadius, SpatialSound},
    components::{ActiveMotion, Command, SoundCategory},
    resources::{GameData, SoundSettings},
};

#[derive(Component, Default)]
pub struct NpcIdleSoundState {
    pub last_idle_loop_count: Option<usize>,
}

pub fn npc_idle_sound_system(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &Npc,
        &ActiveMotion,
        &Command,
        &GlobalTransform,
        Option<&mut NpcIdleSoundState>,
    )>,
    asset_server: Res<AssetServer>,
    game_data: Res<GameData>,
    sound_settings: Res<SoundSettings>,
) {
    let mut rng = rand::thread_rng();
    let gain = sound_settings.gain(SoundCategory::NpcSounds);

    for (entity, npc, active_motion, command, global_transform, idle_sound_state) in
        query.iter_mut()
    {
        if idle_sound_state.is_none() {
            commands.entity(entity).insert(NpcIdleSoundState::default());
            continue;
        }
        let mut idle_sound_state = idle_sound_state.unwrap();

        if !command.is_stop() {
            idle_sound_state.last_idle_loop_count = None;
            continue;
        }

        // There is a 20% chance to play the idle sound, once per animation loop
        if let Some(last_idle_loop_count) = idle_sound_state.last_idle_loop_count {
            if last_idle_loop_count >= active_motion.loop_count {
                continue;
            }
            idle_sound_state.last_idle_loop_count = Some(active_motion.loop_count);
        } else {
            idle_sound_state.last_idle_loop_count = Some(active_motion.loop_count);
        }

        if rng.gen_range(0..100) < 20 {
            if let Some(npc_data) = game_data.npcs.get_npc(npc.id) {
                if let Some(sound_data) = SoundId::new(npc_data.normal_effect_sound_index as u16)
                    .and_then(|id| game_data.sounds.get_sound(id))
                {
                    commands.entity(entity).with_children(|builder| {
                        builder.spawn_bundle((
                            SpatialSound::new(asset_server.load(sound_data.path.path())),
                            SoundRadius::new(4.0),
                            SoundCategory::NpcSounds,
                            gain,
                            Transform::default(),
                            *global_transform,
                        ));
                    });
                }
            }
        }
    }
}
