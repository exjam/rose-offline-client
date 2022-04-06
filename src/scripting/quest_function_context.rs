#[derive(Default)]
pub struct QuestFunctionContext {
    pub selected_quest_index: Option<usize>,
    pub next_quest_trigger: Option<String>,
}
