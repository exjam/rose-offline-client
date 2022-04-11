use std::sync::Arc;

use rose_data::{
    AnimationEventFlags, CharacterMotionDatabase, DataDecoder, EffectDatabase, ItemDatabase,
    NpcDatabase, QuestDatabase, SkillDatabase, StatusEffectDatabase, ZoneList,
};
use rose_file_readers::{LtbFile, StlFile};
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
    pub status_effects: Arc<StatusEffectDatabase>,
    pub zone_list: Arc<ZoneList>,
    pub ltb_event: LtbFile,
    pub stl_quest: StlFile,
}
