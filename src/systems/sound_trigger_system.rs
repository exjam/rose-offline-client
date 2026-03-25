use crate::{
    audio::SpatialSound,
    components::SoundCategory,
    events::{ClientEntityEvent, PlayerCommandEvent, UseItemEvent},
    resources::{GameData, SoundCache},
    ui::UiSoundEvent,
    Config,
};
use bevy::{
    asset::AssetServer,
    ecs::query::WorldQuery,
    prelude::{Commands, Entity, EventReader, EventWriter, GlobalTransform, Query, Res, Transform},
};
use rose_data::SoundId;
use rose_game_common::components::{Inventory, ItemSlot, Npc};

const LEVEL_UP: u16 = 16;
const GET_ITEM: u16 = 531;

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct PlayerQuery<'w> {
    entity: Entity,
    inventory: &'w Inventory,
}

pub fn sound_trigger_system(
    mut commands: Commands,
    mut ui_sound_events: EventWriter<UiSoundEvent>,
    mut player_command_events: EventReader<PlayerCommandEvent>,
    mut client_entity_events: EventReader<ClientEntityEvent>,
    mut use_item_events: EventReader<UseItemEvent>,
    mut query_player: Query<PlayerQuery>,
    query_global_transform: Query<&GlobalTransform>,
    query_npc: Query<(&Npc, &GlobalTransform)>,
    game_data: Res<GameData>,
    sound_cache: Res<SoundCache>,
    asset_server: Res<AssetServer>,
    config: Res<Config>,
) {
    let player = match query_player.get_single_mut() {
        Ok(player) => player,
        Err(_) => return,
    };

    let mut play_sound =
        |sound_id: SoundId, sound_category: SoundCategory, global_transform: &GlobalTransform| {
            let sound_data = match game_data.sounds.get_sound(sound_id) {
                Some(sound_data) => sound_data,
                None => return,
            };

            commands.spawn((
                sound_category,
                config.sound.gain(sound_category),
                SpatialSound::new(sound_cache.load(sound_data, &asset_server)),
                Transform::from_translation(global_transform.translation()),
                GlobalTransform::from_translation(global_transform.translation()),
            ));
        };

    let get_entity_sound = |entity: Entity| -> Option<(SoundCategory, &GlobalTransform)> {
        let sound_category = if player.entity == entity {
            SoundCategory::PlayerCombat
        } else {
            SoundCategory::OtherCombat
        };

        let global_transform = query_global_transform.get(entity).ok()?;
        Some((sound_category, global_transform))
    };

    let get_equip_sound = |item_slot: ItemSlot| -> Option<SoundId> {
        let item = player.inventory.get_item(item_slot)?;
        let item_data = game_data.items.get_base_item(item.get_item_reference())?;

        item_data.equip_sound_id
    };

    for event in player_command_events.iter() {
        let event = event.clone();

        match event {
            PlayerCommandEvent::EquipAmmo(item_slot)
            | PlayerCommandEvent::EquipEquipment(item_slot)
            | PlayerCommandEvent::EquipVehicle(item_slot) => {
                if let Some(sound_id) = get_equip_sound(item_slot) {
                    ui_sound_events.send(UiSoundEvent::new(sound_id));
                }
            }
            PlayerCommandEvent::PickupDropItem(_, _, _)
            | PlayerCommandEvent::PickupDropMoney(_, _) => {
                ui_sound_events.send(UiSoundEvent::new(SoundId::new(GET_ITEM).unwrap()));
            }
            _ => {}
        };
    }

    for event in client_entity_events.iter() {
        match *event {
            ClientEntityEvent::Die(entity) => {
                let (npc, global_transform) = match query_npc.get(entity) {
                    Ok((npc, global_transform)) => (npc, global_transform),
                    Err(_) => continue,
                };

                let npc_data = match game_data.npcs.get_npc(npc.id) {
                    Some(npc_data) => npc_data,
                    None => continue,
                };

                let sound_id = match npc_data.die_sound_id {
                    Some(sound_id) => sound_id,
                    None => continue,
                };

                play_sound(sound_id, SoundCategory::NpcSounds, global_transform);
            }
            ClientEntityEvent::LevelUp(entity, _) => {
                if let Some((sound_category, global_transform)) = get_entity_sound(entity) {
                    play_sound(
                        SoundId::new(LEVEL_UP).unwrap(),
                        sound_category,
                        global_transform,
                    );
                }
            }
        }
    }

    for event in use_item_events.iter() {
        let UseItemEvent { entity, item } = *event;

        let item_data = match game_data.items.get_consumable_item(item.item_number) {
            Some(item_data) => item_data,
            None => continue,
        };

        let sound_id = match item_data.effect_sound_id {
            Some(sound_id) => sound_id,
            None => continue,
        };

        if let Some((sound_category, global_transform)) = get_entity_sound(entity) {
            play_sound(sound_id, sound_category, global_transform);
        }
    }
}
