use rose_data::QuestTriggerHash;

pub enum QuestTriggerEvent {
    ApplyRewards(QuestTriggerHash),
    DoTrigger(QuestTriggerHash),
}
