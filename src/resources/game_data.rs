use std::sync::Arc;

use rose_data::{
    CharacterMotionDatabase, DataDecoder, ItemDatabase, NpcDatabase, QuestDatabase, SkillDatabase,
    StatusEffectDatabase, ZoneList,
};
use rose_game_common::data::AbilityValueCalculator;

pub struct GameData {
    pub ability_value_calculator: Box<dyn AbilityValueCalculator + Send + Sync>,
    pub character_motion_database: Arc<CharacterMotionDatabase>,
    pub data_decoder: Box<dyn DataDecoder + Send + Sync>,
    pub items: Arc<ItemDatabase>,
    pub npcs: Arc<NpcDatabase>,
    pub quests: Arc<QuestDatabase>,
    pub skills: Arc<SkillDatabase>,
    pub status_effects: Arc<StatusEffectDatabase>,
    pub zone_list: Arc<ZoneList>,
}
