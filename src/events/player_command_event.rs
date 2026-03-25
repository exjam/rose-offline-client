use bevy::prelude::{Entity, Event};

use rose_data::{AmmoIndex, EquipmentIndex, Item, VehiclePartIndex};
use rose_game_common::components::{HotbarSlot, ItemSlot, Money, SkillSlot};

use crate::components::Position;

#[derive(Event, Clone)]
pub enum PlayerCommandEvent {
    UseSkill(SkillSlot),
    DropItem(ItemSlot),
    UseItem(ItemSlot),
    UseHotbar(usize, usize),
    SetHotbar(usize, usize, Option<HotbarSlot>),
    Attack(Entity),
    Move(Position, Option<Entity>),
    UnequipAmmo(AmmoIndex),
    UnequipEquipment(EquipmentIndex),
    UnequipVehicle(VehiclePartIndex),
    EquipAmmo(ItemSlot),
    EquipEquipment(ItemSlot),
    EquipVehicle(ItemSlot),
    DropMoney(usize),
    BankDepositItem(ItemSlot),
    BankWithdrawItem(usize),
    PickupDropItem(Item, Entity, ItemSlot),
    PickupDropMoney(Money, Entity),
}
