use bevy::prelude::Event;

use rose_data::QuestTriggerHash;

#[derive(Event)]
pub enum QuestTriggerEvent {
    ApplyRewards(QuestTriggerHash),
    DoTrigger(QuestTriggerHash),
}
