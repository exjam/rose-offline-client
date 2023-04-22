use bevy::{
    ecs::query::WorldQuery,
    math::Vec3,
    prelude::{AssetServer, Assets, Commands, EventReader, GlobalTransform, Query, Res, Transform},
    render::mesh::skinning::SkinnedMesh,
};

use rose_data::{
    AmmoIndex, AnimationEventFlags, EquipmentIndex, ItemClass, SoundId, VehiclePartIndex,
};
use rose_game_common::components::{Equipment, MoveMode, Npc};

use crate::{
    animation::AnimationFrameEvent,
    audio::SpatialSound,
    components::{Command, DummyBoneOffset, PlayerCharacter, SoundCategory},
    resources::{CurrentZone, GameData, SoundCache, SoundSettings},
    zone_loader::ZoneLoaderAsset,
};

#[derive(WorldQuery)]
pub struct EventEntity<'w> {
    command: &'w Command,
    global_transform: &'w GlobalTransform,
    move_mode: Option<&'w MoveMode>,
    skinned_mesh: &'w SkinnedMesh,
    dummy_bone_offset: &'w DummyBoneOffset,
    equipment: Option<&'w Equipment>,
    npc: Option<&'w Npc>,
    player: Option<&'w PlayerCharacter>,
}

#[derive(WorldQuery)]
pub struct TargetEntity<'w> {
    global_transform: &'w GlobalTransform,
    npc: Option<&'w Npc>,
    player: Option<&'w PlayerCharacter>,
}

pub fn animation_sound_system(
    mut commands: Commands,
    mut animation_frame_events: EventReader<AnimationFrameEvent>,
    game_data: Res<GameData>,
    asset_server: Res<AssetServer>,
    current_zone: Option<Res<CurrentZone>>,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
    sound_settings: Res<SoundSettings>,
    query_event_entity: Query<EventEntity>,
    query_target_entity: Query<TargetEntity>,
    query_global_transform: Query<&GlobalTransform>,
    sound_cache: Res<SoundCache>,
) {
    for event in animation_frame_events.iter() {
        let event_entity = if let Ok(event_entity) = query_event_entity.get(event.entity) {
            event_entity
        } else {
            continue;
        };
        let target_entity = event_entity
            .command
            .get_target()
            .and_then(|target_entity| query_target_entity.get(target_entity).ok());
        let event_entity_is_player = event_entity.player.is_some();
        let target_entity_is_player = target_entity
            .as_ref()
            .map_or(false, |target_entity| target_entity.player.is_some());

        if event.flags.contains(AnimationEventFlags::SOUND_FOOTSTEP) {
            let default_step_sound_data = game_data.sounds.get_sound(SoundId::new(653).unwrap());

            let step_sound_data = if let Some(current_zone) = current_zone.as_ref() {
                if let Some(current_zone_data) = zone_loader_assets.get(&current_zone.handle) {
                    let translation = event_entity.global_transform.translation();
                    let position =
                        Vec3::new(translation.x * 100.0, -translation.z * 100.0, translation.y);

                    // TODO: Collision system should set a component indicating whether we are standing on object or terrain
                    if current_zone_data.get_terrain_height(position.x, position.y) / 100.0
                        < (translation.y - 0.05)
                    {
                        // Standing on an object, use default sound
                        default_step_sound_data
                    } else {
                        let tile_number = current_zone_data.get_tile_index(position.x, position.y);
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
                let sound_category = if event_entity.player.is_some() {
                    SoundCategory::PlayerFootstep
                } else {
                    SoundCategory::OtherFootstep
                };

                commands.spawn((
                    sound_category,
                    sound_settings.gain(sound_category),
                    SpatialSound::new(sound_cache.load(sound_data, &asset_server)),
                    Transform::from_translation(event_entity.global_transform.translation()),
                    GlobalTransform::from_translation(event_entity.global_transform.translation()),
                ));
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::SOUND_MOVE_VEHICLE_DUMMY1)
            || event
                .flags
                .contains(AnimationEventFlags::SOUND_MOVE_VEHICLE_DUMMY2)
        {
            if let Some(sound_data) = event_entity
                .equipment
                .and_then(|equipment| equipment.get_vehicle_item(VehiclePartIndex::Leg))
                .and_then(|legs| game_data.items.get_vehicle_item(legs.item.item_number))
                .and_then(|vehicle_item_data| vehicle_item_data.move_sound_id)
                .and_then(|sound_id| game_data.sounds.get_sound(sound_id))
            {
                let sound_category = if event_entity.player.is_some() {
                    SoundCategory::PlayerFootstep
                } else {
                    SoundCategory::OtherFootstep
                };

                if event
                    .flags
                    .contains(AnimationEventFlags::SOUND_MOVE_VEHICLE_DUMMY1)
                {
                    if let Some(dummy_transform) = event_entity
                        .skinned_mesh
                        .joints
                        .get(event_entity.dummy_bone_offset.index + 1)
                        .and_then(|dummy_entity| query_global_transform.get(*dummy_entity).ok())
                    {
                        commands.spawn((
                            sound_category,
                            sound_settings.gain(sound_category),
                            SpatialSound::new(sound_cache.load(sound_data, &asset_server)),
                            Transform::from_translation(dummy_transform.translation()),
                            GlobalTransform::from_translation(dummy_transform.translation()),
                        ));
                    }
                }

                if event
                    .flags
                    .contains(AnimationEventFlags::SOUND_MOVE_VEHICLE_DUMMY2)
                {
                    if let Some(dummy_transform) = event_entity
                        .skinned_mesh
                        .joints
                        .get(event_entity.dummy_bone_offset.index + 2)
                        .and_then(|dummy_entity| query_global_transform.get(*dummy_entity).ok())
                    {
                        commands.spawn((
                            sound_category,
                            sound_settings.gain(sound_category),
                            SpatialSound::new(sound_cache.load(sound_data, &asset_server)),
                            Transform::from_translation(dummy_transform.translation()),
                            GlobalTransform::from_translation(dummy_transform.translation()),
                        ));
                    }
                }
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::SOUND_WEAPON_ATTACK_START)
        {
            let sound_id = if let Some(weapon_item_data) = event_entity
                .equipment
                .and_then(|equipment| equipment.get_equipment_item(EquipmentIndex::Weapon))
                .and_then(|weapon| game_data.items.get_weapon_item(weapon.item.item_number))
            {
                weapon_item_data.attack_start_sound_id
            } else if let Some(npc_data) = event_entity
                .npc
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
                let sound_category = if event_entity_is_player || target_entity_is_player {
                    SoundCategory::PlayerCombat
                } else {
                    SoundCategory::OtherCombat
                };

                commands.spawn((
                    sound_category,
                    sound_settings.gain(sound_category),
                    SpatialSound::new(sound_cache.load(sound_data, &asset_server)),
                    Transform::from_translation(event_entity.global_transform.translation()),
                    GlobalTransform::from_translation(event_entity.global_transform.translation()),
                ));
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::SOUND_WEAPON_ATTACK_HIT)
        {
            if let Some(target_entity) = target_entity.as_ref() {
                let sound_data = if event_entity
                    .move_mode
                    .map_or(false, |move_mode| matches!(move_mode, MoveMode::Drive))
                {
                    event_entity
                        .equipment
                        .and_then(|equipment| equipment.get_vehicle_item(VehiclePartIndex::Arms))
                        .and_then(|legs| game_data.items.get_vehicle_item(legs.item.item_number))
                        .and_then(|vehicle_item_data| vehicle_item_data.hit_sound_id)
                        .and_then(|sound_id| game_data.sounds.get_sound(sound_id))
                } else {
                    let hit_sound_material_type = if let Some(target_npc) = target_entity.npc {
                        game_data
                            .npcs
                            .get_npc(target_npc.id)
                            .map_or(0, |npc_data| npc_data.hit_sound_material_type as usize)
                    } else {
                        1
                    };

                    let weapon_item_number = event_entity
                        .equipment
                        .and_then(|equipment| equipment.get_equipment_item(EquipmentIndex::Weapon))
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
                    let sound_category =
                        if event_entity.player.is_some() || target_entity.player.is_some() {
                            SoundCategory::PlayerCombat
                        } else {
                            SoundCategory::OtherCombat
                        };

                    commands.spawn((
                        sound_category,
                        sound_settings.gain(sound_category),
                        SpatialSound::new(sound_cache.load(sound_data, &asset_server)),
                        Transform::from_translation(target_entity.global_transform.translation()),
                        GlobalTransform::from_translation(
                            target_entity.global_transform.translation(),
                        ),
                    ));
                }
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::SOUND_WEAPON_FIRE_BULLET)
        {
            if let Some(target_entity) = target_entity.as_ref() {
                let fire_sound_id = if event_entity
                    .move_mode
                    .map_or(false, |move_mode| matches!(move_mode, MoveMode::Drive))
                {
                    event_entity
                        .equipment
                        .and_then(|equipment| equipment.get_vehicle_item(VehiclePartIndex::Arms))
                        .and_then(|legs| game_data.items.get_vehicle_item(legs.item.item_number))
                        .and_then(|vehicle_item_data| vehicle_item_data.bullet_effect_id)
                        .and_then(|id| game_data.effect_database.get_effect(id))
                        .and_then(|projectile_effect_data| projectile_effect_data.fire_sound_id)
                } else {
                    event_entity
                        .equipment
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
                    let sound_category =
                        if event_entity.player.is_some() || target_entity.player.is_some() {
                            SoundCategory::PlayerCombat
                        } else {
                            SoundCategory::OtherCombat
                        };

                    commands.spawn((
                        sound_category,
                        sound_settings.gain(sound_category),
                        SpatialSound::new(sound_cache.load(sound_data, &asset_server)),
                        Transform::from_translation(event_entity.global_transform.translation()),
                        GlobalTransform::from_translation(
                            event_entity.global_transform.translation(),
                        ),
                    ));
                }
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::SOUND_SKILL_FIRE_BULLET)
        {
            if let Some(target_entity) = target_entity.as_ref() {
                if let Some(sound_data) = event_entity
                    .command
                    .get_skill_id()
                    .and_then(|skill_id| game_data.skills.get_skill(skill_id))
                    .and_then(|skill_data| skill_data.bullet_fire_sound_id)
                    .and_then(|id| game_data.sounds.get_sound(id))
                {
                    let sound_category =
                        if event_entity.player.is_some() || target_entity.player.is_some() {
                            SoundCategory::PlayerCombat
                        } else {
                            SoundCategory::OtherCombat
                        };

                    commands.spawn((
                        sound_category,
                        sound_settings.gain(sound_category),
                        SpatialSound::new(sound_cache.load(sound_data, &asset_server)),
                        Transform::from_translation(event_entity.global_transform.translation()),
                        GlobalTransform::from_translation(
                            event_entity.global_transform.translation(),
                        ),
                    ));
                }
            }
        }

        if event.flags.contains(AnimationEventFlags::SOUND_SKILL_HIT) {
            if let Some(target_entity) = target_entity.as_ref() {
                if let Some(sound_data) = event_entity
                    .command
                    .get_skill_id()
                    .and_then(|skill_id| game_data.skills.get_skill(skill_id))
                    .and_then(|skill_data| skill_data.hit_sound_id)
                    .and_then(|id| game_data.sounds.get_sound(id))
                {
                    let sound_category =
                        if event_entity.player.is_some() || target_entity.player.is_some() {
                            SoundCategory::PlayerCombat
                        } else {
                            SoundCategory::OtherCombat
                        };

                    commands.spawn((
                        sound_category,
                        sound_settings.gain(sound_category),
                        SpatialSound::new(sound_cache.load(sound_data, &asset_server)),
                        Transform::from_translation(target_entity.global_transform.translation()),
                        GlobalTransform::from_translation(
                            target_entity.global_transform.translation(),
                        ),
                    ));
                }
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::SOUND_SKILL_DUMMY_HIT_0)
        {
            if let Some(target_entity) = target_entity.as_ref() {
                if let Some(sound_data) = event_entity
                    .command
                    .get_skill_id()
                    .and_then(|skill_id| game_data.skills.get_skill(skill_id))
                    .and_then(|skill_data| skill_data.hit_dummy_sound_id[0])
                    .and_then(|id| game_data.sounds.get_sound(id))
                {
                    let sound_category =
                        if event_entity.player.is_some() || target_entity.player.is_some() {
                            SoundCategory::PlayerCombat
                        } else {
                            SoundCategory::OtherCombat
                        };

                    commands.spawn((
                        sound_category,
                        sound_settings.gain(sound_category),
                        SpatialSound::new(sound_cache.load(sound_data, &asset_server)),
                        Transform::from_translation(target_entity.global_transform.translation()),
                        GlobalTransform::from_translation(
                            target_entity.global_transform.translation(),
                        ),
                    ));
                }
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::SOUND_SKILL_DUMMY_HIT_1)
        {
            if let Some(target_entity) = target_entity.as_ref() {
                if let Some(sound_data) = event_entity
                    .command
                    .get_skill_id()
                    .and_then(|skill_id| game_data.skills.get_skill(skill_id))
                    .and_then(|skill_data| skill_data.hit_dummy_sound_id[1])
                    .and_then(|id| game_data.sounds.get_sound(id))
                {
                    let sound_category =
                        if event_entity.player.is_some() || target_entity.player.is_some() {
                            SoundCategory::PlayerCombat
                        } else {
                            SoundCategory::OtherCombat
                        };

                    commands.spawn((
                        sound_category,
                        sound_settings.gain(sound_category),
                        SpatialSound::new(sound_cache.load(sound_data, &asset_server)),
                        Transform::from_translation(target_entity.global_transform.translation()),
                        GlobalTransform::from_translation(
                            target_entity.global_transform.translation(),
                        ),
                    ));
                }
            }
        }
    }
}
