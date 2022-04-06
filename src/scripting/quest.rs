use rose_data::QuestTriggerHash;

use crate::scripting::{
    quest_trigger_check_conditions, quest_triggers_apply_rewards, quest_triggers_skip_rewards,
    QuestFunctionContext, ScriptFunctionContext, ScriptFunctionResources,
};

pub enum QuestError {
    TriggerNotFound,
}

pub fn quest_check_conditions(
    script_resources: &ScriptFunctionResources,
    script_context: &mut ScriptFunctionContext,
    name: &str,
) -> Result<bool, QuestError> {
    let mut trigger = script_resources.game_data.quests.get_trigger_by_name(name);
    if trigger.is_none() {
        return Err(QuestError::TriggerNotFound);
    }

    let mut quest_context = QuestFunctionContext::default();
    let mut success = false;

    while trigger.is_some() {
        let quest_trigger = trigger.unwrap();

        if quest_trigger_check_conditions(
            script_resources,
            script_context,
            &mut quest_context,
            quest_trigger,
        ) && quest_triggers_skip_rewards(
            script_resources,
            script_context,
            &mut quest_context,
            quest_trigger,
        ) {
            success = true;
            break;
        }

        if quest_context.next_quest_trigger.is_some() {
            trigger = quest_context
                .next_quest_trigger
                .take()
                .and_then(|name| script_resources.game_data.quests.get_trigger_by_name(&name));
        } else {
            trigger = trigger
                .unwrap()
                .next_trigger_name
                .as_ref()
                .and_then(|name| script_resources.game_data.quests.get_trigger_by_name(name));
        }
    }

    Ok(success)
}

pub fn quest_apply_rewards(
    script_resources: &ScriptFunctionResources,
    script_context: &mut ScriptFunctionContext,
    trigger_hash: QuestTriggerHash,
) -> Result<bool, QuestError> {
    let mut trigger = script_resources
        .game_data
        .quests
        .get_trigger_by_hash(trigger_hash);
    if trigger.is_none() {
        return Err(QuestError::TriggerNotFound);
    }

    let mut quest_context = QuestFunctionContext::default();
    let mut success = false;

    while trigger.is_some() {
        let quest_trigger = trigger.unwrap();

        if quest_trigger_check_conditions(
            script_resources,
            script_context,
            &mut quest_context,
            quest_trigger,
        ) && quest_triggers_apply_rewards(
            script_resources,
            script_context,
            &mut quest_context,
            quest_trigger,
        ) {
            success = true;
            break;
        }

        if quest_context.next_quest_trigger.is_some() {
            trigger = quest_context
                .next_quest_trigger
                .take()
                .and_then(|name| script_resources.game_data.quests.get_trigger_by_name(&name));
        } else {
            trigger = trigger
                .unwrap()
                .next_trigger_name
                .as_ref()
                .and_then(|name| script_resources.game_data.quests.get_trigger_by_name(name));
        }
    }

    Ok(success)
}
