use bevy::prelude::EventReader;

use crate::{
    events::QuestTriggerEvent,
    scripting::{quest_apply_rewards, ScriptFunctionContext, ScriptFunctionResources},
};

pub fn quest_trigger_system(
    mut quest_trigger_events: EventReader<QuestTriggerEvent>,
    mut script_context: ScriptFunctionContext,
    script_resources: ScriptFunctionResources,
) {
    for event in quest_trigger_events.iter() {
        let &QuestTriggerEvent::ApplyRewards(trigger_hash) = event;
        quest_apply_rewards(&script_resources, &mut script_context, trigger_hash).ok();
    }
}
