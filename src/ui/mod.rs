mod dialog_loader;
mod drag_and_drop_slot;
mod tooltips;
mod ui_character_create_system;
mod ui_character_info_system;
mod ui_character_select_name_tag_system;
mod ui_character_select_system;
mod ui_chatbox_system;
mod ui_debug_camera_info_system;
mod ui_debug_client_entity_list_system;
mod ui_debug_command_viewer_system;
mod ui_debug_diagnostics_system;
mod ui_debug_dialog_list;
mod ui_debug_entity_inspector_system;
mod ui_debug_item_list_system;
mod ui_debug_npc_list_system;
mod ui_debug_physics;
mod ui_debug_render_system;
mod ui_debug_skill_list_system;
mod ui_debug_window_system;
mod ui_debug_zone_lighting_system;
mod ui_debug_zone_list_system;
mod ui_debug_zone_time_system;
mod ui_drag_and_drop_system;
mod ui_game_menu_system;
mod ui_hotbar_system;
mod ui_inventory_system;
mod ui_login_system;
mod ui_message_box_system;
mod ui_minimap_system;
mod ui_npc_store_system;
mod ui_number_input_dialog_system;
mod ui_party_option_system;
mod ui_party_system;
mod ui_personal_store_system;
mod ui_player_info_system;
mod ui_quest_list_system;
mod ui_selected_target_system;
mod ui_server_select_system;
mod ui_settings_system;
mod ui_skill_list_system;
pub mod widgets;

#[derive(Default)]
pub struct UiStateWindows {
    pub character_info_open: bool,
    pub inventory_open: bool,
    pub skill_list_open: bool,
    pub quest_list_open: bool,
    pub settings_open: bool,
    pub menu_open: bool,
    pub party_options_open: bool,
}

pub use dialog_loader::{load_dialog_sprites_system, DialogInstance, DialogLoader};
pub use drag_and_drop_slot::{DragAndDropId, DragAndDropSlot};
pub use tooltips::{ui_add_item_tooltip, ui_add_skill_tooltip};
pub use ui_character_create_system::ui_character_create_system;
pub use ui_character_info_system::ui_character_info_system;
pub use ui_character_select_name_tag_system::ui_character_select_name_tag_system;
pub use ui_character_select_system::ui_character_select_system;
pub use ui_chatbox_system::ui_chatbox_system;
pub use ui_debug_camera_info_system::ui_debug_camera_info_system;
pub use ui_debug_client_entity_list_system::ui_debug_client_entity_list_system;
pub use ui_debug_command_viewer_system::ui_debug_command_viewer_system;
pub use ui_debug_diagnostics_system::ui_debug_diagnostics_system;
pub use ui_debug_dialog_list::ui_debug_dialog_list_system;
pub use ui_debug_entity_inspector_system::ui_debug_entity_inspector_system;
pub use ui_debug_item_list_system::ui_debug_item_list_system;
pub use ui_debug_npc_list_system::ui_debug_npc_list_system;
pub use ui_debug_physics::ui_debug_physics_system;
pub use ui_debug_render_system::ui_debug_render_system;
pub use ui_debug_skill_list_system::ui_debug_skill_list_system;
pub use ui_debug_window_system::{ui_debug_menu_system, UiStateDebugWindows};
pub use ui_debug_zone_lighting_system::ui_debug_zone_lighting_system;
pub use ui_debug_zone_list_system::ui_debug_zone_list_system;
pub use ui_debug_zone_time_system::ui_debug_zone_time_system;
pub use ui_drag_and_drop_system::{ui_drag_and_drop_system, UiStateDragAndDrop};
pub use ui_game_menu_system::ui_game_menu_system;
pub use ui_hotbar_system::ui_hotbar_system;
pub use ui_inventory_system::ui_inventory_system;
pub use ui_login_system::ui_login_system;
pub use ui_message_box_system::ui_message_box_system;
pub use ui_minimap_system::ui_minimap_system;
pub use ui_npc_store_system::ui_npc_store_system;
pub use ui_number_input_dialog_system::ui_number_input_dialog_system;
pub use ui_party_option_system::ui_party_option_system;
pub use ui_party_system::ui_party_system;
pub use ui_personal_store_system::ui_personal_store_system;
pub use ui_player_info_system::ui_player_info_system;
pub use ui_quest_list_system::ui_quest_list_system;
pub use ui_selected_target_system::ui_selected_target_system;
pub use ui_server_select_system::ui_server_select_system;
pub use ui_settings_system::ui_settings_system;
pub use ui_skill_list_system::ui_skill_list_system;
pub use widgets::DataBindings;
