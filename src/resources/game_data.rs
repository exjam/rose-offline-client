use bevy::prelude::Transform;
use std::sync::Arc;

use rose_data::{
    AnimationEventFlags, CharacterMotionDatabase, DataDecoder, EffectDatabase, ItemDatabase,
    NpcDatabase, QuestDatabase, SkillDatabase, SkyboxDatabase, SoundDatabase, StatusEffectDatabase,
    ZoneList,
};
use rose_file_readers::{LtbFile, StbFile, StlFile, ZscFile};
use rose_game_common::data::AbilityValueCalculator;

pub struct GameData {
    pub ability_value_calculator: Box<dyn AbilityValueCalculator + Send + Sync>,
    pub animation_event_flags: Vec<AnimationEventFlags>,
    pub character_motion_database: Arc<CharacterMotionDatabase>,
    pub data_decoder: Box<dyn DataDecoder + Send + Sync>,
    pub effect_database: Arc<EffectDatabase>,
    pub items: Arc<ItemDatabase>,
    pub npcs: Arc<NpcDatabase>,
    pub quests: Arc<QuestDatabase>,
    pub skills: Arc<SkillDatabase>,
    pub skybox: Arc<SkyboxDatabase>,
    pub sounds: Arc<SoundDatabase>,
    pub status_effects: Arc<StatusEffectDatabase>,
    pub zone_list: Arc<ZoneList>,
    pub ltb_event: LtbFile,
    pub stl_quest: StlFile,
    pub zsc_event_object: ZscFile,
    pub zsc_special_object: ZscFile,
    pub stb_morph_object: StbFile,
    pub character_select_positions: Vec<Transform>,
}
