use bevy::prelude::{EventReader, EventWriter};
use rose_file_readers::VfsPathBuf;

use crate::events::{ConversationDialogEvent, SystemFuncEvent};

pub fn system_func_event_system(
    mut events: EventReader<SystemFuncEvent>,
    mut conversation_dialog_events: EventWriter<ConversationDialogEvent>,
) {
    for event in events.iter() {
        let SystemFuncEvent::CallFunction(function_name, _parameters) = event;

        match function_name.as_str() {
            "Lunar_Warp_Gate01" => {
                conversation_dialog_events.send(ConversationDialogEvent::OpenEventDialog(
                    VfsPathBuf::new("3DDATA/EVENT/OBJECT001.CON"),
                ));
            }
            "mushroom" => {
                conversation_dialog_events.send(ConversationDialogEvent::OpenEventDialog(
                    VfsPathBuf::new("3DDATA/EVENT/OBJECT002.CON"),
                ));
            }
            "sandglass" => {
                conversation_dialog_events.send(ConversationDialogEvent::OpenEventDialog(
                    VfsPathBuf::new("3DDATA/EVENT/OBJECT003.CON"),
                ));
            }
            "horriblebook" => {
                conversation_dialog_events.send(ConversationDialogEvent::OpenEventDialog(
                    VfsPathBuf::new("3DDATA/EVENT/OBJECT004.CON"),
                ));
            }
            "piramid01" | "piramid03" => {
                conversation_dialog_events.send(ConversationDialogEvent::OpenEventDialog(
                    VfsPathBuf::new("3DDATA/EVENT/OBJECT005.CON"),
                ));
            }
            "piramid02" => {
                conversation_dialog_events.send(ConversationDialogEvent::OpenEventDialog(
                    VfsPathBuf::new("3DDATA/EVENT/OBJECT006.CON"),
                ));
            }
            "owl" => {
                conversation_dialog_events.send(ConversationDialogEvent::OpenEventDialog(
                    VfsPathBuf::new("3DDATA/EVENT/OBJECT007.CON"),
                ));
            }
            "mana" => {
                conversation_dialog_events.send(ConversationDialogEvent::OpenEventDialog(
                    VfsPathBuf::new("3DDATA/EVENT/OBJECT008.CON"),
                ));
            }
            "genzistone" => {
                conversation_dialog_events.send(ConversationDialogEvent::OpenEventDialog(
                    VfsPathBuf::new("3DDATA/EVENT/OBJECT009.CON"),
                ));
            }
            unimplemented => log::warn!("Unimplemented system func function {}", unimplemented),
        }
    }
}
