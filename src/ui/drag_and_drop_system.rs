use bevy::prelude::ResMut;
use bevy_egui::{egui, EguiContext};

use crate::ui::DragAndDropId;

#[derive(Default)]
pub struct UiStateDragAndDrop {
    pub source: Option<DragAndDropId>,
    pub destination: Option<DragAndDropId>,
}

pub fn ui_drag_and_drop_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_dnd: ResMut<UiStateDragAndDrop>,
) {
    let input = egui_context.ctx_mut().input();

    // When mouse is released, clear drag and drop state
    if input.pointer.any_released() && !input.pointer.button_down(egui::PointerButton::Primary) {
        ui_state_dnd.source = None;
        ui_state_dnd.destination = None;
    }
}
