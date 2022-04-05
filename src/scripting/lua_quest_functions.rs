use std::collections::HashMap;

use crate::scripting::{lua4::Lua4Value, LuaUserValueEntity, ScriptFunctionContext};

pub struct LuaQuestFunctions {
    pub closures: HashMap<String, fn(&mut ScriptFunctionContext, Vec<Lua4Value>) -> Vec<Lua4Value>>,
}

impl Default for LuaQuestFunctions {
    fn default() -> Self {
        let mut closures: HashMap<
            String,
            fn(&mut ScriptFunctionContext, Vec<Lua4Value>) -> Vec<Lua4Value>,
        > = HashMap::new();

        closures.insert("QF_checkQuestCondition".into(), QF_checkQuestCondition);
        closures.insert("QF_doQuestTrigger".into(), QF_doQuestTrigger);
        closures.insert("QF_getEventOwner".into(), QF_getEventOwner);
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
        QF_findQuest
        QF_getEpisodeVAR
        QF_getJobVAR
        QF_getNpcQuestZeroVal
        QF_getPlanetVAR
        QF_getQuestCount
        QF_getQuestID
        QF_getQuestItemQuantity
        QF_getQuestSwitch
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
    _context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    if let Ok(quest_trigger_name) = parameters[0].to_string() {
        log::warn!(
            "TODO: Implement QF_checkQuestCondition({})",
            quest_trigger_name
        );
        // TODO: Client-side check of quest condition
    }

    vec![0.into()]
}

#[allow(non_snake_case)]
fn QF_doQuestTrigger(
    _context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    if let Ok(quest_trigger_name) = parameters[0].to_string() {
        log::warn!("TODO: Implement QF_doQuestTrigger({})", quest_trigger_name);
        // TODO: Client-side check of quest condition, then send quest packet to server
    }

    vec![0.into()]
}

#[allow(non_snake_case)]
fn QF_getEventOwner(
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
fn QF_getUserSwitch(
    context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    let switch_id = parameters[0].to_i32().unwrap() as usize;
    let quest_state = context.query_quest.single();

    vec![quest_state.quest_switches[switch_id].into()]
}
