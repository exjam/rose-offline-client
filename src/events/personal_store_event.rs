use bevy::prelude::Entity;
use rose_game_common::messages::server::PersonalStoreItemList;

pub enum PersonalStoreEvent {
    OpenEntityStore(Entity),
    SetItemList(PersonalStoreItemList),
}
