use crate::resources::NameTagSettings;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct InterfaceConfig {
    pub targeting: TargetingType,
    pub name_tag_settings: NameTagSettings,
}

impl Default for InterfaceConfig {
    fn default() -> Self {
        Self {
            targeting: TargetingType::DoubleClick,
            name_tag_settings: NameTagSettings::default(),
        }
    }
}

#[derive(Deserialize, Serialize, Copy, Clone, PartialEq)]
pub enum TargetingType {
    DoubleClick,
    SingleClick,
}
