use rose_data::QuestTriggerHash;
use rose_file_readers::QsdVariableType;

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
        ) && quest_triggers_skip_rewards(
            script_resources,
            script_context,
            &mut quest_context,
            quest_trigger,
        ) {
            success = true;

            if quest_context.next_quest_trigger.is_some() {
                trigger = quest_context
                    .next_quest_trigger
                    .take()
                    .and_then(|name| script_resources.game_data.quests.get_trigger_by_name(&name));
            } else {
                trigger = None;
            }
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

            if quest_context.next_quest_trigger.is_some() {
                trigger = quest_context
                    .next_quest_trigger
                    .take()
                    .and_then(|name| script_resources.game_data.quests.get_trigger_by_name(&name));
            } else {
                trigger = None;
            }
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

pub fn get_quest_variable(
    script_resources: &ScriptFunctionResources,
    script_context: &mut ScriptFunctionContext,
    quest_context: &mut QuestFunctionContext,
    variable_type: QsdVariableType,
    variable_id: usize,
) -> Option<i32> {
    let quest_state = script_context.query_quest.single();
    let active_quest = quest_context
        .selected_quest_index
        .and_then(|quest_index| quest_state.get_quest(quest_index));

    match variable_type {
        QsdVariableType::Variable => active_quest
            .and_then(|active_quest| active_quest.variables.get(variable_id))
            .map(|x| *x as i32),
        QsdVariableType::Switch => active_quest
            .and_then(|active_quest| active_quest.switches.get(variable_id))
            .map(|x| *x as i32),
        QsdVariableType::Timer => active_quest
            .and_then(|active_quest| active_quest.expire_time)
            .map(|expire_time| {
                expire_time
                    .0
                    .saturating_sub(script_resources.world_time.ticks.0) as i32
            }),
        QsdVariableType::Episode => quest_state
            .episode_variables
            .get(variable_id)
            .map(|x| *x as i32),
        QsdVariableType::Job => quest_state
            .job_variables
            .get(variable_id)
            .map(|x| *x as i32),
        QsdVariableType::Planet => quest_state
            .planet_variables
            .get(variable_id)
            .map(|x| *x as i32),
        QsdVariableType::Union => quest_state
            .union_variables
            .get(variable_id)
            .map(|x| *x as i32),
    }
}

pub fn set_quest_variable(
    _script_resources: &ScriptFunctionResources,
    script_context: &mut ScriptFunctionContext,
    quest_context: &mut QuestFunctionContext,
    variable_type: QsdVariableType,
    variable_id: usize,
    value: i32,
) {
    let mut quest_state = script_context.query_quest.single_mut();
    let active_quest = quest_context
        .selected_quest_index
        .and_then(|quest_index| quest_state.get_quest_mut(quest_index));

    match variable_type {
        QsdVariableType::Variable => active_quest
            .and_then(|active_quest| active_quest.variables.get_mut(variable_id))
            .map(|x| *x = value as u16),
        QsdVariableType::Switch => active_quest
            .and_then(|active_quest| active_quest.switches.get_mut(variable_id))
            .map(|mut x| *x = value != 0),
        QsdVariableType::Episode => quest_state
            .episode_variables
            .get_mut(variable_id)
            .map(|x| *x = value as u16),
        QsdVariableType::Job => quest_state
            .job_variables
            .get_mut(variable_id)
            .map(|x| *x = value as u16),
        QsdVariableType::Planet => quest_state
            .planet_variables
            .get_mut(variable_id)
            .map(|x| *x = value as u16),
        QsdVariableType::Union => quest_state
            .union_variables
            .get_mut(variable_id)
            .map(|x| *x = value as u16),
        QsdVariableType::Timer => None, // Does nothing
    };
}
