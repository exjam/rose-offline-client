use bevy::{
    math::Vec3,
    prelude::{AssetServer, Assets, Commands, EventReader, GlobalTransform, Query, Res, Transform},
};

use rose_data::{
    AmmoIndex, AnimationEventFlags, EquipmentIndex, ItemClass, SoundId, VehiclePartIndex,
};
use rose_game_common::components::{Equipment, MoveMode, Npc, Target};

use crate::{
    audio::SpatialSound,
    components::{PlayerCharacter, SoundCategory},
    events::AnimationFrameEvent,
    resources::{CurrentZone, GameData, SoundSettings},
    zone_loader::ZoneLoaderAsset,
};

pub fn animation_sound_system(
    mut commands: Commands,
    mut animation_frame_events: EventReader<AnimationFrameEvent>,
    game_data: Res<GameData>,
    asset_server: Res<AssetServer>,
    current_zone: Option<Res<CurrentZone>>,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
    sound_settings: Res<SoundSettings>,
    query_global_transform: Query<&GlobalTransform>,
    query_move_mode: Query<&MoveMode>,
    query_player: Query<&PlayerCharacter>,
    query_npc: Query<&Npc>,
    query_equipment: Query<&Equipment>,
    query_target: Query<&Target>,
) {
    let is_player = |entity| -> bool { query_player.contains(entity) };
    let target_is_player = |entity| -> bool {
        query_target
            .get(entity)
            .map(|target| query_player.contains(target.entity))
            .unwrap_or(false)
    };
    let is_player_or_target_is_player =
        |entity| -> bool { is_player(entity) || target_is_player(entity) };

    for event in animation_frame_events.iter() {
        if event.flags.contains(AnimationEventFlags::SOUND_FOOTSTEP) {
            if let Ok(global_transform) = query_global_transform.get(event.entity) {
                let default_step_sound_data =
                    game_data.sounds.get_sound(SoundId::new(653).unwrap());

                let step_sound_data = if let Some(current_zone) = current_zone.as_ref() {
                    if let Some(current_zone_data) = zone_loader_assets.get(&current_zone.handle) {
                        let translation = global_transform.translation();
                        let position =
                            Vec3::new(translation.x * 100.0, -translation.z * 100.0, translation.y);

                        // TODO: Collision system should set a component indicating whether we are standing on object or terrain
                        if current_zone_data.get_terrain_height(position.x, position.y) / 100.0
                            < (translation.y - 0.05)
                        {
                            // Standing on an object, use default sound
                            default_step_sound_data
                        } else {
                            let tile_number =
                                current_zone_data.get_tile_index(position.x, position.y);
                            let zone_type = game_data
                                .zone_list
                                .get_zone(current_zone.id)
                                .and_then(|zone_data| zone_data.footstep_type)
                                .unwrap_or(0) as usize;
                            game_data.sounds.get_step_sound(tile_number, zone_type)
                        }
                    } else {
                        default_step_sound_data
                    }
                } else {
                    default_step_sound_data
                };

                if let Some(sound_data) = step_sound_data {
                    let sound_category = if is_player(event.entity) {
                        SoundCategory::PlayerFootstep
                    } else {
                        SoundCategory::OtherFootstep
                    };

                    commands.spawn_bundle((
                        sound_category,
                        sound_settings.gain(sound_category),
                        SpatialSound::new(asset_server.load(sound_data.path.path())),
                        Transform::from_translation(global_transform.translation()),
                        GlobalTransform::from_translation(global_transform.translation()),
                    ));
                }
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::SOUND_MOVE_VEHICLE_DUMMY1)
            || event
                .flags
                .contains(AnimationEventFlags::SOUND_MOVE_VEHICLE_DUMMY2)
        {
            if let Ok(global_transform) = query_global_transform.get(event.entity) {
                if let Some(sound_data) = query_equipment
                    .get(event.entity)
                    .ok()
                    .and_then(|equipment| equipment.get_vehicle_item(VehiclePartIndex::Leg))
                    .and_then(|legs| game_data.items.get_vehicle_item(legs.item.item_number))
                    .and_then(|vehicle_item_data| vehicle_item_data.move_sound_id)
                    .and_then(|sound_id| game_data.sounds.get_sound(sound_id))
                {
                    let sound_category = if is_player(event.entity) {
                        SoundCategory::PlayerFootstep
                    } else {
                        SoundCategory::OtherFootstep
                    };

                    commands.spawn_bundle((
                        sound_category,
                        sound_settings.gain(sound_category),
                        SpatialSound::new(asset_server.load(sound_data.path.path())),
                        Transform::from_translation(global_transform.translation()),
                        GlobalTransform::from_translation(global_transform.translation()),
                    ));
                }
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::SOUND_WEAPON_ATTACK_START)
        {
            if let Ok(global_transform) = query_global_transform.get(event.entity) {
                let sound_id = if let Some(weapon_item_data) = query_equipment
                    .get(event.entity)
                    .ok()
                    .and_then(|equipment| equipment.get_equipment_item(EquipmentIndex::Weapon))
                    .and_then(|weapon| game_data.items.get_weapon_item(weapon.item.item_number))
                {
                    weapon_item_data.attack_start_sound_id
                } else if let Some(npc_data) = query_npc
                    .get(event.entity)
                    .ok()
                    .and_then(|npc| game_data.npcs.get_npc(npc.id))
                {
                    npc_data.attack_sound_id
                } else {
                    game_data
                        .items
                        .get_weapon_item(0)
                        .and_then(|weapon_item_data| weapon_item_data.attack_start_sound_id)
                };

                if let Some(sound_data) = sound_id.and_then(|id| game_data.sounds.get_sound(id)) {
                    let sound_category = if is_player_or_target_is_player(event.entity) {
                        SoundCategory::PlayerCombat
                    } else {
                        SoundCategory::OtherCombat
                    };

                    commands.spawn_bundle((
                        sound_category,
                        sound_settings.gain(sound_category),
                        SpatialSound::new(asset_server.load(sound_data.path.path())),
                        Transform::from_translation(global_transform.translation()),
                        GlobalTransform::from_translation(global_transform.translation()),
                    ));
                }
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::SOUND_WEAPON_ATTACK_HIT)
        {
            if let Ok(target_entity) = query_target.get(event.entity).map(|target| target.entity) {
                if let Ok(target_global_transform) = query_global_transform.get(target_entity) {
                    let sound_data = if query_move_mode
                        .get(event.entity)
                        .map_or(false, |move_mode| matches!(move_mode, MoveMode::Drive))
                    {
                        query_equipment
                            .get(event.entity)
                            .ok()
                            .and_then(|equipment| {
                                equipment.get_vehicle_item(VehiclePartIndex::Arms)
                            })
                            .and_then(|legs| {
                                game_data.items.get_vehicle_item(legs.item.item_number)
                            })
                            .and_then(|vehicle_item_data| vehicle_item_data.hit_sound_id)
                            .and_then(|sound_id| game_data.sounds.get_sound(sound_id))
                    } else {
                        let hit_sound_material_type =
                            if let Ok(target_npc) = query_npc.get(target_entity) {
                                game_data
                                    .npcs
                                    .get_npc(target_npc.id)
                                    .map_or(0, |npc_data| npc_data.hit_sound_material_type as usize)
                            } else {
                                1
                            };

                        let weapon_item_number = query_equipment
                            .get(event.entity)
                            .ok()
                            .and_then(|equipment| {
                                equipment.get_equipment_item(EquipmentIndex::Weapon)
                            })
                            .map_or(0, |weapon| weapon.item.item_number);

                        let weapon_hit_sound_type = game_data
                            .items
                            .get_weapon_item(weapon_item_number)
                            .map_or(0, |weapon_item_data| {
                                weapon_item_data.attack_hit_sound_index as usize
                            });

                        game_data
                            .sounds
                            .get_hit_sound(weapon_hit_sound_type, hit_sound_material_type)
                    };

                    if let Some(sound_data) = sound_data {
                        let sound_category = if is_player(event.entity) || is_player(target_entity)
                        {
                            SoundCategory::PlayerCombat
                        } else {
                            SoundCategory::OtherCombat
                        };

                        commands.spawn_bundle((
                            sound_category,
                            sound_settings.gain(sound_category),
                            SpatialSound::new(asset_server.load(sound_data.path.path())),
                            Transform::from_translation(target_global_transform.translation()),
                            GlobalTransform::from_translation(
                                target_global_transform.translation(),
                            ),
                        ));
                    }
                }
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::SOUND_WEAPON_FIRE_BULLET)
        {
            if let Ok(global_transform) = query_global_transform.get(event.entity) {
                let fire_sound_id = if query_move_mode
                    .get(event.entity)
                    .map_or(false, |move_mode| matches!(move_mode, MoveMode::Drive))
                {
                    query_equipment
                        .get(event.entity)
                        .ok()
                        .and_then(|equipment| equipment.get_vehicle_item(VehiclePartIndex::Arms))
                        .and_then(|legs| game_data.items.get_vehicle_item(legs.item.item_number))
                        .and_then(|vehicle_item_data| vehicle_item_data.bullet_effect_id)
                        .and_then(|id| game_data.effect_database.get_effect(id))
                        .and_then(|projectile_effect_data| projectile_effect_data.fire_sound_id)
                } else {
                    query_equipment
                        .get(event.entity)
                        .ok()
                        .and_then(|equipment| {
                            game_data
                                .items
                                .get_weapon_item(
                                    equipment
                                        .get_equipment_item(EquipmentIndex::Weapon)
                                        .map(|weapon| weapon.item.item_number)
                                        .unwrap_or(0),
                                )
                                .and_then(|weapon_item_data| {
                                    match weapon_item_data.item_data.class {
                                        ItemClass::Bow | ItemClass::Crossbow => {
                                            Some(AmmoIndex::Arrow)
                                        }
                                        ItemClass::Gun | ItemClass::DualGuns => {
                                            Some(AmmoIndex::Bullet)
                                        }
                                        ItemClass::Launcher => Some(AmmoIndex::Throw),
                                        _ => None,
                                    }
                                    .and_then(|ammo_index| equipment.get_ammo_item(ammo_index))
                                    .and_then(|ammo_item| {
                                        game_data
                                            .items
                                            .get_material_item(ammo_item.item.item_number)
                                    })
                                    .and_then(|ammo_item_data| ammo_item_data.bullet_effect_id)
                                    .or(weapon_item_data.bullet_effect_id)
                                })
                        })
                        .and_then(|id| game_data.effect_database.get_effect(id))
                        .and_then(|projectile_effect_data| projectile_effect_data.fire_sound_id)
                };

                if let Some(sound_data) =
                    fire_sound_id.and_then(|id| game_data.sounds.get_sound(id))
                {
                    let sound_category = if is_player_or_target_is_player(event.entity) {
                        SoundCategory::PlayerCombat
                    } else {
                        SoundCategory::OtherCombat
                    };

                    commands.spawn_bundle((
                        sound_category,
                        sound_settings.gain(sound_category),
                        SpatialSound::new(asset_server.load(sound_data.path.path())),
                        Transform::from_translation(global_transform.translation()),
                        GlobalTransform::from_translation(global_transform.translation()),
                    ));
                }
            }
        }
    }
}
