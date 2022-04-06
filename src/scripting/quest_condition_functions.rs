use rose_data::QuestTrigger;
use rose_file_readers::QsdCondition;

use crate::scripting::{QuestFunctionContext, ScriptFunctionContext, ScriptFunctionResources};

fn quest_condition_check_switch(
    _script_resources: &ScriptFunctionResources,
    script_context: &mut ScriptFunctionContext,
    _quest_context: &mut QuestFunctionContext,
    switch_id: usize,
    value: bool,
) -> bool {
    let quest_state = script_context.query_quest.single();

    if let Some(switch_value) = quest_state.quest_switches.get(switch_id) {
        return *switch_value == value;
    }

    false
}

fn quest_condition_select_quest(
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

pub fn quest_trigger_check_conditions(
    script_resources: &ScriptFunctionResources,
    script_context: &mut ScriptFunctionContext,
    quest_context: &mut QuestFunctionContext,
    quest_trigger: &QuestTrigger,
) -> bool {
    for condition in quest_trigger.conditions.iter() {
        let result = match *condition {
            QsdCondition::QuestSwitch(switch_id, value) => quest_condition_check_switch(
                script_resources,
                script_context,
                quest_context,
                switch_id,
                value,
            ),
            QsdCondition::SelectQuest(quest_id) => quest_condition_select_quest(
                script_resources,
                script_context,
                quest_context,
                quest_id,
            ),
            _ => {
                log::warn!("Unimplemented quest condition: {:?}", condition);
                false
            }
        };

        if !result {
            return false;
        }
    }

    true
}
