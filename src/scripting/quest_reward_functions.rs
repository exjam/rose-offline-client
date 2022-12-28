use rose_data::{QuestTrigger, StackableItem, WorldTicks};
use rose_file_readers::{
    QsdItemBase1000, QsdReward, QsdRewardOperator, QsdRewardQuestAction, QsdVariableType,
};
use rose_game_common::components::ActiveQuest;

use crate::{
    events::{ChatboxEvent, SystemFuncEvent},
    scripting::{
        quest::{get_quest_variable, set_quest_variable},
        QuestFunctionContext, ScriptFunctionContext, ScriptFunctionResources,
    },
};

fn quest_reward_operator(operator: QsdRewardOperator, variable_value: i32, value: i32) -> i32 {
    match operator {
        QsdRewardOperator::Set => value,
        QsdRewardOperator::Add => variable_value + value,
        QsdRewardOperator::Subtract => variable_value - value,
        QsdRewardOperator::Zero => 0,
        QsdRewardOperator::One => 1,
    }
}

fn quest_get_expire_time(
    script_resources: &ScriptFunctionResources,
    quest_id: usize,
) -> Option<WorldTicks> {
    script_resources
        .game_data
        .quests
        .get_quest_data(quest_id)
        .and_then(|quest_data| quest_data.time_limit)
        .map(|time_limit| script_resources.world_time.ticks + time_limit)
}

fn quest_reward_add_item(
    script_resources: &ScriptFunctionResources,
    script_context: &mut ScriptFunctionContext,
    quest_context: &mut QuestFunctionContext,
    item_base1000: QsdItemBase1000,
    quantity: usize,
) -> bool {
    let item_reference = script_resources
        .game_data
        .data_decoder
        .decode_item_base1000(item_base1000.get());
    if item_reference.is_none() {
        return false;
    }
    let item_reference = item_reference.unwrap();

    let mut quest_state = script_context.query_quest.single_mut();

    if item_reference.item_type.is_quest_item() {
        // Add to quest items
        if let Some(selected_quest_index) = quest_context.selected_quest_index {
            if quest_state
                .get_quest_mut(selected_quest_index)
                .and_then(|active_quest| {
                    StackableItem::new(item_reference, quantity as u32)
                        .and_then(|item| active_quest.try_add_item(item.into()).ok())
                })
                .is_some()
            {
                if let Some(item_data) = script_resources
                    .game_data
                    .items
                    .get_base_item(item_reference)
                {
                    script_context
                        .chatbox_events
                        .send(ChatboxEvent::Quest(format!(
                            "You have earned {}.",
                            item_data.name
                        )));
                }

                return true;
            }
        }
    } else {
        // Server is responsible for updating inventory, ignore on client
        return true;
    }

    false
}

fn quest_reward_remove_item(
    script_resources: &ScriptFunctionResources,
    script_context: &mut ScriptFunctionContext,
    quest_context: &mut QuestFunctionContext,
    item_base1000: QsdItemBase1000,
    quantity: usize,
) -> bool {
    let item_reference = script_resources
        .game_data
        .data_decoder
        .decode_item_base1000(item_base1000.get());
    if item_reference.is_none() {
        return false;
    }
    let item_reference = item_reference.unwrap();

    let mut quest_state = script_context.query_quest.single_mut();

    if item_reference.item_type.is_quest_item() {
        // Remove from quest items
        if let Some(selected_quest_index) = quest_context.selected_quest_index {
            return quest_state
                .get_quest_mut(selected_quest_index)
                .and_then(|active_quest| {
                    active_quest.try_take_item(item_reference, quantity as u32)
                })
                .is_some();
        }
    } else {
        // Server is responsible for updating inventory, ignore on client
        return true;
    }

    false
}

fn quest_reward_select_quest(
    _script_resources: &ScriptFunctionResources,
    script_context: &mut ScriptFunctionContext,
    quest_context: &mut QuestFunctionContext,
    quest_id: usize,
) -> bool {
    let quest_state = script_context.query_quest.single();

    if let Some(quest_index) = quest_state.find_active_quest_index(quest_id) {
        quest_context.selected_quest_index = Some(quest_index);
        return true;
    }

    false
}

fn quest_reward_remove_selected_quest(
    script_resources: &ScriptFunctionResources,
    script_context: &mut ScriptFunctionContext,
    quest_context: &mut QuestFunctionContext,
) -> bool {
    let mut quest_state = script_context.query_quest.single_mut();

    if let Some(quest_index) = quest_context.selected_quest_index {
        if let Some(quest_slot) = quest_state.get_quest_slot_mut(quest_index) {
            if let Some(quest_data) = quest_slot.as_ref().and_then(|active_quest| {
                script_resources
                    .game_data
                    .quests
                    .get_quest_data(active_quest.quest_id)
            }) {
                script_context
                    .chatbox_events
                    .send(ChatboxEvent::Quest(format!(
                        "Completed quest \"{}\".",
                        quest_data.name
                    )));
            }

            *quest_slot = None;
            return true;
        }
    }

    false
}

fn quest_reward_add_quest(
    script_resources: &ScriptFunctionResources,
    script_context: &mut ScriptFunctionContext,
    quest_context: &mut QuestFunctionContext,
    quest_id: usize,
) -> bool {
    let mut quest_state = script_context.query_quest.single_mut();

    if let Some(quest_index) = quest_state.try_add_quest(ActiveQuest::new(
        quest_id,
        quest_get_expire_time(script_resources, quest_id),
    )) {
        if quest_context.selected_quest_index.is_none() {
            quest_context.selected_quest_index = Some(quest_index);
        }

        if let Some(quest_data) = script_resources.game_data.quests.get_quest_data(quest_id) {
            script_context
                .chatbox_events
                .send(ChatboxEvent::Quest(format!(
                    "Started quest \"{}\".",
                    quest_data.name
                )));
            script_context
                .chatbox_events
                .send(ChatboxEvent::Quest(quest_data.description.to_string()));
        }

        return true;
    }

    false
}

fn quest_reward_change_selected_quest_id(
    script_resources: &ScriptFunctionResources,
    script_context: &mut ScriptFunctionContext,
    quest_context: &mut QuestFunctionContext,
    quest_id: usize,
    keep_data: bool,
) -> bool {
    let mut quest_state = script_context.query_quest.single_mut();

    if let Some(quest_index) = quest_context.selected_quest_index {
        if let Some(Some(active_quest)) = quest_state.get_quest_slot_mut(quest_index) {
            if keep_data {
                active_quest.quest_id = quest_id;
            } else {
                *active_quest =
                    ActiveQuest::new(quest_id, quest_get_expire_time(script_resources, quest_id));
            }

            if let Some(quest_data) = script_resources.game_data.quests.get_quest_data(quest_id) {
                script_context
                    .chatbox_events
                    .send(ChatboxEvent::Quest(format!(
                        "Started quest \"{}\"",
                        quest_data.name
                    )));

                script_context
                    .chatbox_events
                    .send(ChatboxEvent::Quest(quest_data.description.to_string()));
            }

            return true;
        }
    }

    false
}

fn quest_reward_set_health_mana_percent(
    _script_resources: &ScriptFunctionResources,
    script_context: &mut ScriptFunctionContext,
    _quest_context: &mut QuestFunctionContext,
    health_percent: i32,
    mana_percent: i32,
) -> bool {
    let mut character = script_context.query_player.single_mut();
    character.health_points.hp = (character.ability_values.get_max_health() * health_percent) / 100;
    character.mana_points.mp = (character.ability_values.get_max_mana() * mana_percent) / 100;
    true
}

fn quest_reward_set_quest_switch(
    _script_resources: &ScriptFunctionResources,
    script_context: &mut ScriptFunctionContext,
    _quest_context: &mut QuestFunctionContext,
    switch_id: usize,
    value: bool,
) -> bool {
    let mut quest_state = script_context.query_quest.single_mut();

    if let Some(mut switch) = quest_state.quest_switches.get_mut(switch_id) {
        *switch = value;
        return true;
    }

    false
}

fn quest_reward_quest_variable(
    script_resources: &ScriptFunctionResources,
    script_context: &mut ScriptFunctionContext,
    quest_context: &mut QuestFunctionContext,
    variable_type: QsdVariableType,
    variable_id: usize,
    operator: QsdRewardOperator,
    value: i32,
) -> bool {
    if let Some(variable_value) = get_quest_variable(
        script_resources,
        script_context,
        quest_context,
        variable_type,
        variable_id,
    ) {
        let value = quest_reward_operator(operator, variable_value, value);
        set_quest_variable(
            script_resources,
            script_context,
            quest_context,
            variable_type,
            variable_id,
            value,
        );
        true
    } else {
        false
    }
}

fn quest_reward_set_next_trigger(
    _script_resources: &ScriptFunctionResources,
    _script_context: &mut ScriptFunctionContext,
    quest_context: &mut QuestFunctionContext,
    name: String,
) -> bool {
    quest_context.next_quest_trigger = Some(name);
    true
}

fn quest_reward_call_lua_function(
    _script_resources: &ScriptFunctionResources,
    script_context: &mut ScriptFunctionContext,
    _quest_context: &mut QuestFunctionContext,
    name: String,
) -> bool {
    script_context
        .script_system_events
        .send(SystemFuncEvent::CallFunction(
            name,
            vec![script_context
                .query_player
                .get_single()
                .ok()
                .and_then(|player| player.client_entity.cloned())
                .map_or(0, |client_entity| client_entity.id.0)
                .into()],
        ));
    true
}

pub fn quest_triggers_skip_rewards(
    script_resources: &ScriptFunctionResources,
    script_context: &mut ScriptFunctionContext,
    quest_context: &mut QuestFunctionContext,
    quest_trigger: &QuestTrigger,
) -> bool {
    // When we are skipping rewards, we only need to process QsdReward::Trigger
    for reward in quest_trigger.rewards.iter() {
        if let QsdReward::Trigger(name) = reward {
            quest_reward_set_next_trigger(
                script_resources,
                script_context,
                quest_context,
                name.clone(),
            );
        }
    }

    true
}

pub fn quest_triggers_apply_rewards(
    script_resources: &ScriptFunctionResources,
    script_context: &mut ScriptFunctionContext,
    quest_context: &mut QuestFunctionContext,
    quest_trigger: &QuestTrigger,
) -> bool {
    for reward in quest_trigger.rewards.iter() {
        let result = match *reward {
            QsdReward::AddItem(_, item_base1000, quantity) => quest_reward_add_item(
                script_resources,
                script_context,
                quest_context,
                item_base1000,
                quantity,
            ),
            QsdReward::RemoveItem(_, item_base1000, quantity) => quest_reward_remove_item(
                script_resources,
                script_context,
                quest_context,
                item_base1000,
                quantity,
            ),
            QsdReward::Quest(QsdRewardQuestAction::Select(quest_id)) => {
                quest_reward_select_quest(script_resources, script_context, quest_context, quest_id)
            }
            QsdReward::Quest(QsdRewardQuestAction::RemoveSelected) => {
                quest_reward_remove_selected_quest(script_resources, script_context, quest_context)
            }
            QsdReward::Quest(QsdRewardQuestAction::Add(quest_id)) => {
                quest_reward_add_quest(script_resources, script_context, quest_context, quest_id)
            }
            QsdReward::Quest(QsdRewardQuestAction::ChangeSelectedIdKeepData(quest_id)) => {
                quest_reward_change_selected_quest_id(
                    script_resources,
                    script_context,
                    quest_context,
                    quest_id,
                    true,
                )
            }
            QsdReward::Quest(QsdRewardQuestAction::ChangeSelectedIdResetData(quest_id)) => {
                quest_reward_change_selected_quest_id(
                    script_resources,
                    script_context,
                    quest_context,
                    quest_id,
                    false,
                )
            }
            QsdReward::QuestVariable(ref quest_variables) => {
                quest_variables.iter().all(|quest_variable| {
                    quest_reward_quest_variable(
                        script_resources,
                        script_context,
                        quest_context,
                        quest_variable.variable_type,
                        quest_variable.variable_id,
                        quest_variable.operator,
                        quest_variable.value,
                    )
                })
            }
            QsdReward::SetHealthManaPercent(_target, health_percent, mana_percent) => {
                quest_reward_set_health_mana_percent(
                    script_resources,
                    script_context,
                    quest_context,
                    health_percent as i32,
                    mana_percent as i32,
                )
            }
            QsdReward::SetQuestSwitch(switch_id, value) => quest_reward_set_quest_switch(
                script_resources,
                script_context,
                quest_context,
                switch_id,
                value,
            ),
            QsdReward::Trigger(ref name) => quest_reward_set_next_trigger(
                script_resources,
                script_context,
                quest_context,
                name.clone(),
            ),
            QsdReward::CallLuaFunction(ref name) => quest_reward_call_lua_function(
                script_resources,
                script_context,
                quest_context,
                name.clone(),
            ),
            // Server side only rewards:
            QsdReward::AbilityValue(_)
            | QsdReward::CalculatedExperiencePoints(_, _, _)
            | QsdReward::CalculatedItem(_, _)
            | QsdReward::CalculatedMoney(_, _, _)
            | QsdReward::Teleport(_, _, _) => true,
            _ => {
                log::warn!("Unimplemented quest reward: {:?}", reward);
                true
            }
        };

        if !result {
            log::debug!(target: "quest", "Reward Failed: {:?}", reward);
            return false;
        } else {
            log::debug!(target: "quest", "Reward Success: {:?}", reward);
        }
    }

    true
}
