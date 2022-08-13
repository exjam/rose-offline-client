use bevy::prelude::Entity;

use rose_data::Item;
use rose_game_common::messages::server::PersonalStoreItemList;

pub enum PersonalStoreEvent {
    OpenEntityStore(Entity),
    SetItemList(PersonalStoreItemList),
    BuyItem {
        slot_index: usize,
        item: Item,
    },
    UpdateBuyList {
        entity: Entity,
        item_list: Vec<(usize, Option<Item>)>,
    },
    UpdateSellList {
        entity: Entity,
        item_list: Vec<(usize, Option<Item>)>,
    },
}
