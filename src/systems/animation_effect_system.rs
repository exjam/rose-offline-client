use bevy::prelude::{Entity, EventReader, EventWriter, Query, Res};

use rose_data::{
    AmmoIndex, AnimationEventFlags, EffectBulletMoveType, EquipmentIndex, ItemClass, SkillData,
    SkillType,
};
use rose_game_common::components::{Equipment, MoveSpeed, Npc};

use crate::{
    components::{Command, CommandCastSkillTarget},
    events::{
        AnimationFrameEvent, HitEvent, SpawnEffectData, SpawnEffectEvent, SpawnProjectileEvent,
        SpawnProjectileTarget,
    },
    resources::{ClientEntityList, GameData},
};

pub fn animation_effect_system(
    mut animation_frame_events: EventReader<AnimationFrameEvent>,
    mut spawn_effect_events: EventWriter<SpawnEffectEvent>,
    mut spawn_projectile_events: EventWriter<SpawnProjectileEvent>,
    mut hit_events: EventWriter<HitEvent>,
    query_command: Query<&Command>,
    query_equipment: Query<&Equipment>,
    query_npc: Query<&Npc>,
    game_data: Res<GameData>,
    client_entity_list: Res<ClientEntityList>,
) {
    for event in animation_frame_events.iter() {
        if client_entity_list.player_entity == Some(event.entity) {
            log::info!(target: "animation", "Player animation event flags: {:?}", event.flags);
        }

        if event
            .flags
            .contains(AnimationEventFlags::EFFECT_WEAPON_ATTACK_HIT)
        {
            if let Ok(Command::Attack(command_attack)) = query_command.get(event.entity) {
                let effect_id = query_equipment
                    .get(event.entity)
                    .ok()
                    .and_then(|equipment| {
                        game_data.items.get_weapon_item(
                            equipment
                                .get_equipment_item(EquipmentIndex::WeaponRight)
                                .map(|weapon| weapon.item.item_number)
                                .unwrap_or(0),
                        )
                    })
                    .and_then(|weapon_item_data| weapon_item_data.effect_id)
                    .or_else(|| {
                        query_npc
                            .get(event.entity)
                            .ok()
                            .and_then(|npc| game_data.npcs.get_npc(npc.id))
                            .and_then(|npc_data| npc_data.hand_hit_effect_id)
                    });

                hit_events.send(HitEvent::with_weapon(
                    event.entity,
                    command_attack.target,
                    effect_id,
                ));
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::EFFECT_WEAPON_FIRE_BULLET)
        {
            if let Ok(Command::Attack(command_attack)) = query_command.get(event.entity) {
                let projectile_effect_data = query_equipment
                    .get(event.entity)
                    .ok()
                    .and_then(|equipment| {
                        game_data
                            .items
                            .get_weapon_item(
                                equipment
                                    .get_equipment_item(EquipmentIndex::WeaponRight)
                                    .map(|weapon| weapon.item.item_number)
                                    .unwrap_or(0),
                            )
                            .and_then(|weapon_item_data| {
                                match weapon_item_data.item_data.class {
                                    ItemClass::Bow | ItemClass::Crossbow => Some(AmmoIndex::Arrow),
                                    ItemClass::Gun | ItemClass::DualGuns => Some(AmmoIndex::Bullet),
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
                    .and_then(|id| game_data.effect_database.get_effect(id));

                if let Some(projectile_effect_data) = projectile_effect_data {
                    if projectile_effect_data.bullet_effect.is_some() {
                        spawn_projectile_events.send(SpawnProjectileEvent {
                            effect_id: projectile_effect_data.id,
                            source: event.entity,
                            source_dummy_bone_id: Some(0),
                            source_skill_id: None,
                            target: SpawnProjectileTarget::Entity(command_attack.target),
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
            if let Ok(Command::CastSkill(command_cast_skill)) = query_command.get(event.entity) {
                if let Some(skill_data) = game_data.skills.get_skill(command_cast_skill.skill_id) {
                    if let Some(CommandCastSkillTarget::Entity(target_entity)) =
                        command_cast_skill.skill_target
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
                                    apply_damage: !event.flags.contains(
                                        AnimationEventFlags::EFFECT_SKILL_FIRE_DUMMY_BULLET,
                                    ),
                                });
                            }
                        }
                    }
                }
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::EFFECT_SKILL_ACTION)
        {
            if let Ok(Command::CastSkill(command_cast_skill)) = query_command.get(event.entity) {
                if let Some(skill_data) = game_data.skills.get_skill(command_cast_skill.skill_id) {
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
                            if let Some(CommandCastSkillTarget::Entity(target_entity)) =
                                command_cast_skill.skill_target
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
                            if let Some(CommandCastSkillTarget::Entity(target_entity)) =
                                command_cast_skill.skill_target
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
        }

        if event.flags.contains(AnimationEventFlags::EFFECT_SKILL_HIT) {
            if let Ok(Command::CastSkill(command_cast_skill)) = query_command.get(event.entity) {
                if let Some(CommandCastSkillTarget::Entity(target_entity)) =
                    command_cast_skill.skill_target
                {
                    if let Some(skill_data) =
                        game_data.skills.get_skill(command_cast_skill.skill_id)
                    {
                        let weapon_effect_id = query_equipment
                            .get(event.entity)
                            .ok()
                            .and_then(|equipment| {
                                game_data.items.get_weapon_item(
                                    equipment
                                        .get_equipment_item(EquipmentIndex::WeaponRight)
                                        .map(|weapon| weapon.item.item_number)
                                        .unwrap_or(0),
                                )
                            })
                            .and_then(|weapon_item_data| weapon_item_data.effect_id)
                            .or_else(|| {
                                query_npc
                                    .get(event.entity)
                                    .ok()
                                    .and_then(|npc| game_data.npcs.get_npc(npc.id))
                                    .and_then(|npc_data| npc_data.hand_hit_effect_id)
                            });

                        if skill_data.hit_effect_file_id.is_some() {
                            hit_events.send(HitEvent::with_skill(
                                event.entity,
                                target_entity,
                                command_cast_skill.skill_id,
                            ));
                        } else {
                            hit_events.send(HitEvent::with_weapon(
                                event.entity,
                                target_entity,
                                weapon_effect_id,
                            ));
                        }
                    }
                }
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::EFFECT_SKILL_CASTING_0)
        {
            if let Ok(Command::CastSkill(command_cast_skill)) = query_command.get(event.entity) {
                if let Some(skill_data) = game_data.skills.get_skill(command_cast_skill.skill_id) {
                    show_casting_effect(event.entity, skill_data, 0, &mut spawn_effect_events);
                }
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::EFFECT_SKILL_CASTING_1)
        {
            if let Ok(Command::CastSkill(command_cast_skill)) = query_command.get(event.entity) {
                if let Some(skill_data) = game_data.skills.get_skill(command_cast_skill.skill_id) {
                    show_casting_effect(event.entity, skill_data, 1, &mut spawn_effect_events);
                }
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::EFFECT_SKILL_CASTING_2)
        {
            if let Ok(Command::CastSkill(command_cast_skill)) = query_command.get(event.entity) {
                if let Some(skill_data) = game_data.skills.get_skill(command_cast_skill.skill_id) {
                    show_casting_effect(event.entity, skill_data, 2, &mut spawn_effect_events);
                }
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::EFFECT_SKILL_CASTING_3)
        {
            if let Ok(Command::CastSkill(command_cast_skill)) = query_command.get(event.entity) {
                if let Some(skill_data) = game_data.skills.get_skill(command_cast_skill.skill_id) {
                    show_casting_effect(event.entity, skill_data, 3, &mut spawn_effect_events);
                }
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
