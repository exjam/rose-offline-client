use bevy::{
    ecs::query::WorldQuery,
    prelude::{Entity, EventReader, EventWriter, Query, Res},
};

use rose_data::{
    AmmoIndex, AnimationEventFlags, EffectBulletMoveType, EquipmentIndex, ItemClass, SkillData,
    SkillType, VehiclePartIndex,
};
use rose_game_common::components::{Equipment, MoveMode, MoveSpeed, Npc};

use crate::{
    animation::AnimationFrameEvent,
    components::{Command, PlayerCharacter},
    events::{
        HitEvent, SpawnEffectData, SpawnEffectEvent, SpawnProjectileEvent, SpawnProjectileTarget,
    },
    resources::GameData,
};

#[derive(WorldQuery)]
pub struct EventEntity<'w> {
    entity: Entity,
    command: &'w Command,
    move_mode: Option<&'w MoveMode>,
    equipment: Option<&'w Equipment>,
    npc: Option<&'w Npc>,
    player: Option<&'w PlayerCharacter>,
}

pub fn animation_effect_system(
    mut animation_frame_events: EventReader<AnimationFrameEvent>,
    mut spawn_effect_events: EventWriter<SpawnEffectEvent>,
    mut spawn_projectile_events: EventWriter<SpawnProjectileEvent>,
    mut hit_events: EventWriter<HitEvent>,
    query_event_entity: Query<EventEntity>,
    game_data: Res<GameData>,
) {
    for event in animation_frame_events.iter() {
        let event_entity = if let Ok(event_entity) = query_event_entity.get(event.entity) {
            event_entity
        } else {
            continue;
        };
        let target_entity = event_entity.command.get_target();

        if event_entity.player.is_some() {
            log::debug!(target: "animation", "Player animation event flags: {:?}", event.flags);
        }

        if event
            .flags
            .contains(AnimationEventFlags::EFFECT_WEAPON_ATTACK_HIT)
        {
            if let Some(target_entity) = target_entity {
                let effect_id = if event_entity
                    .move_mode
                    .map_or(false, |move_mode| matches!(move_mode, MoveMode::Drive))
                {
                    event_entity
                        .equipment
                        .and_then(|equipment| equipment.get_vehicle_item(VehiclePartIndex::Arms))
                        .and_then(|legs| game_data.items.get_vehicle_item(legs.item.item_number))
                        .and_then(|vehicle_item_data| vehicle_item_data.hit_effect_id)
                } else {
                    event_entity
                        .equipment
                        .and_then(|equipment| {
                            game_data.items.get_weapon_item(
                                equipment
                                    .get_equipment_item(EquipmentIndex::Weapon)
                                    .map(|weapon| weapon.item.item_number)
                                    .unwrap_or(0),
                            )
                        })
                        .and_then(|weapon_item_data| weapon_item_data.effect_id)
                        .or_else(|| {
                            event_entity
                                .npc
                                .and_then(|npc| game_data.npcs.get_npc(npc.id))
                                .and_then(|npc_data| npc_data.hand_hit_effect_id)
                        })
                };

                hit_events.send(HitEvent::with_weapon(
                    event.entity,
                    target_entity,
                    effect_id,
                ));
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::EFFECT_WEAPON_FIRE_BULLET)
        {
            if let Some(target_entity) = target_entity {
                let (source_dummy_bone_id, projectile_effect_data) = if event_entity
                    .move_mode
                    .map_or(false, |move_mode| matches!(move_mode, MoveMode::Drive))
                {
                    (
                        Some(8),
                        event_entity
                            .equipment
                            .and_then(|equipment| {
                                equipment.get_vehicle_item(VehiclePartIndex::Arms)
                            })
                            .and_then(|legs| {
                                game_data.items.get_vehicle_item(legs.item.item_number)
                            })
                            .and_then(|vehicle_item_data| vehicle_item_data.bullet_effect_id)
                            .and_then(|id| game_data.effect_database.get_effect(id)),
                    )
                } else {
                    (
                        None,
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
                            .and_then(|id| game_data.effect_database.get_effect(id)),
                    )
                };

                if let Some(projectile_effect_data) = projectile_effect_data {
                    if projectile_effect_data.bullet_effect.is_some() {
                        spawn_projectile_events.send(SpawnProjectileEvent {
                            effect_id: projectile_effect_data.id,
                            source: event.entity,
                            source_dummy_bone_id,
                            source_skill_id: None,
                            target: SpawnProjectileTarget::Entity(target_entity),
                            move_type: projectile_effect_data
                                .bullet_move_type
                                .as_ref()
                                .cloned()
                                .unwrap_or(EffectBulletMoveType::Linear),
                            move_speed: MoveSpeed::new(projectile_effect_data.bullet_speed / 100.0),
                            apply_damage: true,
                        });
                    }
                }
            }
        }

        if event.flags.intersects(
            AnimationEventFlags::EFFECT_SKILL_FIRE_BULLET
                | AnimationEventFlags::EFFECT_SKILL_FIRE_DUMMY_BULLET,
        ) {
            if let Some(target_entity) = target_entity {
                if let Some(skill_data) = event_entity
                    .command
                    .get_skill_id()
                    .and_then(|skill_id| game_data.skills.get_skill(skill_id))
                {
                    if let Some(effect_data) = skill_data
                        .bullet_effect_id
                        .and_then(|id| game_data.effect_database.get_effect(id))
                    {
                        if effect_data.bullet_effect.is_some() {
                            spawn_projectile_events.send(SpawnProjectileEvent {
                                effect_id: effect_data.id,
                                source: event.entity,
                                source_dummy_bone_id: Some(
                                    skill_data.bullet_link_dummy_bone_id as usize,
                                ),
                                source_skill_id: Some(skill_data.id),
                                target: SpawnProjectileTarget::Entity(target_entity),
                                move_type: effect_data
                                    .bullet_move_type
                                    .as_ref()
                                    .cloned()
                                    .unwrap_or(EffectBulletMoveType::Linear),
                                move_speed: MoveSpeed::new(effect_data.bullet_speed / 100.0),
                                apply_damage: !event
                                    .flags
                                    .contains(AnimationEventFlags::EFFECT_SKILL_FIRE_DUMMY_BULLET),
                            });
                        }
                    }
                }
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::EFFECT_SKILL_ACTION)
        {
            if let Some(skill_data) = event_entity
                .command
                .get_skill_id()
                .and_then(|skill_id| game_data.skills.get_skill(skill_id))
            {
                match skill_data.skill_type {
                    SkillType::BasicAction => {}
                    SkillType::CreateWindow => {}
                    SkillType::Immediate => {}
                    SkillType::SelfBound
                    | SkillType::SelfBoundDuration
                    | SkillType::SelfStateDuration
                    | SkillType::SelfDamage => {
                        if let Some(effect_data) = skill_data
                            .bullet_effect_id
                            .and_then(|id| game_data.effect_database.get_effect(id))
                        {
                            if let Some(effect_file_id) = effect_data.bullet_effect {
                                spawn_effect_events.send(SpawnEffectEvent::OnEntity(
                                    event.entity,
                                    Some(skill_data.bullet_link_dummy_bone_id as usize),
                                    SpawnEffectData::with_file_id(effect_file_id),
                                ));
                            }
                        }

                        if let Some(hit_effect_file_id) = skill_data.hit_effect_file_id {
                            spawn_effect_events.send(SpawnEffectEvent::OnEntity(
                                event.entity,
                                skill_data.hit_link_dummy_bone_id,
                                SpawnEffectData::with_file_id(hit_effect_file_id),
                            ));
                        }
                    }
                    SkillType::FireBullet => {
                        if let Some(target_entity) = target_entity {
                            if let Some(effect_data) = skill_data
                                .bullet_effect_id
                                .and_then(|id| game_data.effect_database.get_effect(id))
                            {
                                if effect_data.bullet_effect.is_some() {
                                    spawn_projectile_events.send(SpawnProjectileEvent {
                                        effect_id: effect_data.id,
                                        source: event.entity,
                                        source_dummy_bone_id: Some(
                                            skill_data.bullet_link_dummy_bone_id as usize,
                                        ),
                                        source_skill_id: Some(skill_data.id),
                                        target: SpawnProjectileTarget::Entity(target_entity),
                                        move_type: effect_data
                                            .bullet_move_type
                                            .as_ref()
                                            .cloned()
                                            .unwrap_or(EffectBulletMoveType::Linear),
                                        move_speed: MoveSpeed::new(
                                            effect_data.bullet_speed / 100.0,
                                        ),
                                        apply_damage: true,
                                    });
                                }
                            }
                        }
                    }
                    SkillType::TargetBound
                    | SkillType::TargetBoundDuration
                    | SkillType::TargetStateDuration
                    | SkillType::Resurrection => {
                        if let Some(target_entity) = target_entity {
                            if let Some(effect_data) = skill_data
                                .bullet_effect_id
                                .and_then(|id| game_data.effect_database.get_effect(id))
                            {
                                if effect_data.bullet_effect.is_some() {
                                    spawn_projectile_events.send(SpawnProjectileEvent {
                                        effect_id: effect_data.id,
                                        source: event.entity,
                                        source_dummy_bone_id: Some(
                                            skill_data.bullet_link_dummy_bone_id as usize,
                                        ),
                                        source_skill_id: Some(skill_data.id),
                                        target: SpawnProjectileTarget::Entity(target_entity),
                                        move_type: effect_data
                                            .bullet_move_type
                                            .as_ref()
                                            .cloned()
                                            .unwrap_or(EffectBulletMoveType::Linear),
                                        move_speed: MoveSpeed::new(
                                            effect_data.bullet_speed / 100.0,
                                        ),
                                        apply_damage: false,
                                    });
                                }
                            }
                        }
                    }
                    _ => log::warn!(
                        "Unimplemented EFFECT_SKILL_ACTION for skill type {:?}",
                        skill_data.skill_type
                    ),
                }
            }
        }

        if event.flags.contains(AnimationEventFlags::EFFECT_SKILL_HIT) {
            if let Some(skill_data) = event_entity
                .command
                .get_skill_id()
                .and_then(|skill_id| game_data.skills.get_skill(skill_id))
            {
                let weapon_effect_id = event_entity
                    .equipment
                    .and_then(|equipment| {
                        game_data.items.get_weapon_item(
                            equipment
                                .get_equipment_item(EquipmentIndex::Weapon)
                                .map(|weapon| weapon.item.item_number)
                                .unwrap_or(0),
                        )
                    })
                    .and_then(|weapon_item_data| weapon_item_data.effect_id)
                    .or_else(|| {
                        event_entity
                            .npc
                            .and_then(|npc| game_data.npcs.get_npc(npc.id))
                            .and_then(|npc_data| npc_data.hand_hit_effect_id)
                    });

                if skill_data.hit_effect_file_id.is_some() {
                    hit_events.send(HitEvent::with_skill_damage(
                        event.entity,
                        target_entity.unwrap_or(event.entity),
                        skill_data.id,
                    ));
                } else {
                    hit_events.send(HitEvent::with_weapon(
                        event.entity,
                        target_entity.unwrap_or(event.entity),
                        weapon_effect_id,
                    ));
                }
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::EFFECT_SKILL_DUMMY_HIT_0)
        {
            if let Some(effect_file_id) = event_entity
                .command
                .get_skill_id()
                .and_then(|skill_id| game_data.skills.get_skill(skill_id))
                .and_then(|skill_data| skill_data.hit_dummy_effect_file_id[0])
            {
                spawn_effect_events.send(SpawnEffectEvent::OnEntity(
                    target_entity.unwrap_or(event.entity),
                    None,
                    SpawnEffectData::with_file_id(effect_file_id),
                ));
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::EFFECT_SKILL_DUMMY_HIT_1)
        {
            if let Some(effect_file_id) = event_entity
                .command
                .get_skill_id()
                .and_then(|skill_id| game_data.skills.get_skill(skill_id))
                .and_then(|skill_data| skill_data.hit_dummy_effect_file_id[1])
            {
                spawn_effect_events.send(SpawnEffectEvent::OnEntity(
                    target_entity.unwrap_or(event.entity),
                    None,
                    SpawnEffectData::with_file_id(effect_file_id),
                ));
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::EFFECT_SKILL_CASTING_0)
        {
            if let Some(skill_data) = event_entity
                .command
                .get_skill_id()
                .and_then(|skill_id| game_data.skills.get_skill(skill_id))
            {
                show_casting_effect(event.entity, skill_data, 0, &mut spawn_effect_events);
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::EFFECT_SKILL_CASTING_1)
        {
            if let Some(skill_data) = event_entity
                .command
                .get_skill_id()
                .and_then(|skill_id| game_data.skills.get_skill(skill_id))
            {
                show_casting_effect(event.entity, skill_data, 1, &mut spawn_effect_events);
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::EFFECT_SKILL_CASTING_2)
        {
            if let Some(skill_data) = event_entity
                .command
                .get_skill_id()
                .and_then(|skill_id| game_data.skills.get_skill(skill_id))
            {
                show_casting_effect(event.entity, skill_data, 2, &mut spawn_effect_events);
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::EFFECT_SKILL_CASTING_3)
        {
            if let Some(skill_data) = event_entity
                .command
                .get_skill_id()
                .and_then(|skill_id| game_data.skills.get_skill(skill_id))
            {
                show_casting_effect(event.entity, skill_data, 3, &mut spawn_effect_events);
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::EFFECT_MOVE_VEHCILE_DUMMY1)
        {
            if let Some(effect_file_id) = event_entity
                .equipment
                .and_then(|equipment| equipment.get_vehicle_item(VehiclePartIndex::Leg))
                .and_then(|legs| game_data.items.get_vehicle_item(legs.item.item_number))
                .and_then(|vehicle_item_data| vehicle_item_data.move_effect_file_id)
            {
                spawn_effect_events.send(SpawnEffectEvent::OnEntity(
                    event.entity,
                    Some(1),
                    SpawnEffectData::with_file_id(effect_file_id),
                ));
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::EFFECT_MOVE_VEHCILE_DUMMY2)
        {
            if let Some(effect_file_id) = event_entity
                .equipment
                .and_then(|equipment| equipment.get_vehicle_item(VehiclePartIndex::Leg))
                .and_then(|legs| game_data.items.get_vehicle_item(legs.item.item_number))
                .and_then(|vehicle_item_data| vehicle_item_data.move_effect_file_id)
            {
                spawn_effect_events.send(SpawnEffectEvent::OnEntity(
                    event.entity,
                    Some(2),
                    SpawnEffectData::with_file_id(effect_file_id),
                ));
            }
        }
    }
}

fn show_casting_effect(
    entity: Entity,
    skill_data: &SkillData,
    casting_effect_index: usize,
    spawn_effect_events: &mut EventWriter<SpawnEffectEvent>,
) {
    if let Some(casting_effect) = skill_data
        .casting_effects
        .get(casting_effect_index)
        .and_then(|x| x.as_ref())
    {
        spawn_effect_events.send(SpawnEffectEvent::OnEntity(
            entity,
            casting_effect.effect_dummy_bone_id,
            SpawnEffectData::with_file_id(casting_effect.effect_file_id),
        ));
    }
}
