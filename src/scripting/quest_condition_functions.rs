use rose_data::QuestTrigger;
use rose_file_readers::{
    QsdCondition, QsdConditionOperator, QsdConditionQuestItem, QsdEquipmentIndex, QsdItemBase1000,
    QsdVariableType,
};

use crate::scripting::{
    quest::get_quest_variable, QuestFunctionContext, ScriptFunctionContext, ScriptFunctionResources,
};

fn quest_condition_operator<T: PartialEq + PartialOrd>(
    operator: QsdConditionOperator,
    value_lhs: T,
    value_rhs: T,
) -> bool {
    match operator {
        QsdConditionOperator::Equals => value_lhs == value_rhs,
        QsdConditionOperator::GreaterThan => value_lhs > value_rhs,
        QsdConditionOperator::GreaterThanEqual => value_lhs >= value_rhs,
        QsdConditionOperator::LessThan => value_lhs < value_rhs,
        QsdConditionOperator::LessThanEqual => value_lhs <= value_rhs,
        QsdConditionOperator::NotEqual => value_lhs != value_rhs,
    }
}

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

fn quest_condition_quest_item(
    script_resources: &ScriptFunctionResources,
    script_context: &mut ScriptFunctionContext,
    quest_context: &mut QuestFunctionContext,
    item_base1000: Option<QsdItemBase1000>,
    equipment_index: Option<QsdEquipmentIndex>,
    required_count: u32,
    operator: QsdConditionOperator,
) -> bool {
    let item_reference = item_base1000.and_then(|item_base1000| {
        script_resources
            .game_data
            .data_decoder
            .decode_item_base1000(item_base1000.get() as usize)
    });

    let equipment_index = equipment_index.and_then(|equipment_index| {
        script_resources
            .game_data
            .data_decoder
            .decode_equipment_index(equipment_index.get())
    });

    let quest_state = script_context.query_quest.single();
    let (equipment, inventory) = script_context.query_player_items.single();

    if let Some(equipment_index) = equipment_index {
        item_reference
            == equipment
                .get_equipment_item(equipment_index)
                .map(|item| item.item)
    } else {
        let quantity = if let Some(item_reference) = item_reference {
            if item_reference.item_type.is_quest_item() {
                // Check selected quest item
                if let Some(selected_quest_index) = quest_context.selected_quest_index {
                    quest_state
                        .get_quest(selected_quest_index)
                        .and_then(|active_quest| active_quest.find_item(item_reference))
                        .map(|quest_item| quest_item.get_quantity())
                        .unwrap_or(0)
                } else {
                    0
                }
            } else {
                // Check inventory
                inventory
                    .find_item(item_reference)
                    .and_then(|slot| inventory.get_item(slot))
                    .map(|inventory_item| inventory_item.get_quantity())
                    .unwrap_or(0)
            }
        } else {
            0
        };

        quest_condition_operator(operator, quantity, required_count)
    }
}

fn quest_condition_quest_items(
    script_resources: &ScriptFunctionResources,
    script_context: &mut ScriptFunctionContext,
    quest_context: &mut QuestFunctionContext,
    items: &[QsdConditionQuestItem],
) -> bool {
    for &QsdConditionQuestItem {
        item,
        equipment_index,
        required_count,
        operator,
    } in items
    {
        if !quest_condition_quest_item(
            script_resources,
            script_context,
            quest_context,
            item,
            equipment_index,
            required_count,
            operator,
        ) {
            return false;
        }
    }

    true
}

fn quest_condition_quest_variable(
    script_resources: &ScriptFunctionResources,
    script_context: &mut ScriptFunctionContext,
    quest_context: &mut QuestFunctionContext,
    variable_type: QsdVariableType,
    variable_id: usize,
    operator: QsdConditionOperator,
    value: i32,
) -> bool {
    if let Some(variable_value) = get_quest_variable(
        script_resources,
        script_context,
        quest_context,
        variable_type,
        variable_id,
    ) {
        quest_condition_operator(operator, variable_value, value)
    } else {
        false
    }
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
            QsdCondition::QuestItems(ref items) => {
                quest_condition_quest_items(script_resources, script_context, quest_context, items)
            }
            QsdCondition::QuestVariable(ref quest_variables) => {
                quest_variables.iter().all(|quest_variable| {
                    quest_condition_quest_variable(
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
            log::debug!(target: "quest", "Condition Failed: {:?}", condition);
            return false;
        } else {
            log::debug!(target: "quest", "Condition Success: {:?}", condition);
        }
    }

    true
}
