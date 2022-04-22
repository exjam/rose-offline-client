use std::collections::HashMap;

use rose_game_common::messages::client::ClientMessage;

use crate::scripting::{
    lua4::Lua4Value, quest::quest_check_conditions, LuaUserValueEntity, ScriptFunctionContext,
    ScriptFunctionResources,
};

pub struct LuaQuestFunctions {
    pub closures: HashMap<
        String,
        fn(&ScriptFunctionResources, &mut ScriptFunctionContext, Vec<Lua4Value>) -> Vec<Lua4Value>,
    >,
}

impl Default for LuaQuestFunctions {
    fn default() -> Self {
        let mut closures: HashMap<
            String,
            fn(
                &ScriptFunctionResources,
                &mut ScriptFunctionContext,
                Vec<Lua4Value>,
            ) -> Vec<Lua4Value>,
        > = HashMap::new();

        closures.insert("QF_checkQuestCondition".into(), QF_checkQuestCondition);
        closures.insert("QF_doQuestTrigger".into(), QF_doQuestTrigger);
        closures.insert("QF_findQuest".into(), QF_findQuest);
        closures.insert("QF_getEventOwner".into(), QF_getEventOwner);
        closures.insert("QF_getQuestSwitch".into(), QF_getQuestSwitch);
        closures.insert("QF_getUserSwitch".into(), QF_getUserSwitch);

        /*
        QF_appendQuest
        QF_beginCon
        QF_CameraworkingNpc
        QF_CameraworkingPoint
        QF_CameraworkingSelf
        QF_ChangetalkImage
        QF_ChangetalkName
        QF_closeCon
        QF_deleteQuest
        QF_EffectCallNpc
        QF_EffectCallSelf
        QF_getEpisodeVAR
        QF_getJobVAR
        QF_getNpcQuestZeroVal
        QF_getPlanetVAR
        QF_getQuestCount
        QF_getQuestID
        QF_getQuestItemQuantity
        QF_getQuestVar
        QF_getSkillLevel
        QF_getUnionVAR
        QF_givePoint
        QF_gotoCon
        QF_MotionCallNpc
        QF_MotionCallSelf
        QF_NpcHide
        QF_NpcTalkinterfaceHide
        QF_NpcTalkinterfaceView
        QF_NpcView
        */

        Self { closures }
    }
}

#[allow(non_snake_case)]
fn QF_checkQuestCondition(
    resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    let result = if let Ok(quest_trigger_name) = parameters[0].to_string() {
        match quest_check_conditions(resources, context, quest_trigger_name.as_str().into()) {
            Ok(result) => {
                if result {
                    1 // Success
                } else {
                    2 // Failed
                }
            }
            Err(_) => {
                0 // Error
            }
        }
    } else {
        0 // Error
    };

    vec![result.into()]
}

#[allow(non_snake_case)]
fn QF_doQuestTrigger(
    resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    let result = if let Ok(quest_trigger_name) = parameters[0].to_string() {
        if let Ok(true) =
            quest_check_conditions(resources, context, quest_trigger_name.as_str().into())
        {
            if let Some(game_connection) = resources.game_connection.as_ref() {
                game_connection
                    .client_message_tx
                    .send(ClientMessage::QuestTrigger(
                        quest_trigger_name.as_str().into(),
                    ))
                    .ok();
            }

            1
        } else {
            0
        }
    } else {
        0
    };

    vec![result.into()]
}

#[allow(non_snake_case)]
fn QF_findQuest(
    _resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    let quest_id = parameters[0].to_i32().unwrap() as usize;
    let quest_state = context.query_quest.single();

    vec![quest_state
        .find_active_quest_index(quest_id)
        .map(|x| x as i32)
        .unwrap_or(-1)
        .into()]
}

#[allow(non_snake_case)]
fn QF_getEventOwner(
    _resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    if let Ok(lua_value_entity) = parameters[0].to_user_type::<LuaUserValueEntity>() {
        if let Ok(client_entity) = context.query_client_entity.get(lua_value_entity.entity) {
            return vec![client_entity.id.0.into()];
        }
    }

    vec![0.into()]
}

#[allow(non_snake_case)]
fn QF_getQuestSwitch(
    _resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    let quest_index = parameters[0].to_i32().unwrap();
    let quest_switch_id = parameters[1].to_i32().unwrap() as usize;
    let quest_state = context.query_quest.single();

    let result = if quest_index >= 0 {
        if let Some(quest) = quest_state.get_quest(quest_index as usize) {
            if quest.switches[quest_switch_id] {
                1
            } else {
                0
            }
        } else {
            -1
        }
    } else {
        -1
    };

    vec![result.into()]
}

#[allow(non_snake_case)]
fn QF_getUserSwitch(
    _resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    let switch_id = parameters[0].to_i32().unwrap() as usize;
    let quest_state = context.query_quest.single();

    vec![quest_state.quest_switches[switch_id].into()]
}
