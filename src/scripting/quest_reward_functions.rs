use rose_data::QuestTrigger;
use rose_file_readers::QsdReward;

use crate::scripting::{QuestFunctionContext, ScriptFunctionContext, ScriptFunctionResources};

pub fn quest_trigger_do_rewards(
    _script_resources: &ScriptFunctionResources,
    _script_context: &mut ScriptFunctionContext,
    quest_context: &mut QuestFunctionContext,
    quest_trigger: &QuestTrigger,
) -> bool {
    // QsdReward::Trigger is the only reward which runs on client
    for reward in quest_trigger.rewards.iter() {
        if let QsdReward::Trigger(name) = reward {
            quest_context.next_quest_trigger = Some(name.clone());
        }
    }

    true
}
