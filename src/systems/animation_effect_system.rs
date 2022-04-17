use bevy::prelude::{Entity, EventReader, EventWriter, Query, Res};

use rose_data::{AnimationEventFlags, EffectBulletMoveType, EquipmentIndex, SkillData, SkillType};
use rose_game_common::components::{Equipment, MoveSpeed, Npc};

use crate::{
    components::{Command, CommandCastSkillTarget},
    events::{
        AnimationFrameEvent, SpawnEffectData, SpawnEffectEvent, SpawnProjectileEvent,
        SpawnProjectileTarget,
    },
    resources::{ClientEntityList, GameData},
};

pub fn animation_effect_system(
    mut animation_frame_events: EventReader<AnimationFrameEvent>,
    mut spawn_effect_events: EventWriter<SpawnEffectEvent>,
    mut spawn_projectile_events: EventWriter<SpawnProjectileEvent>,
    query_command: Query<&Command>,
    query_equipment: Query<&Equipment>,
    query_npc: Query<&Npc>,
    game_data: Res<GameData>,
    client_entity_list: Res<ClientEntityList>,
) {
    for event in animation_frame_events.iter() {
        if client_entity_list.player_entity == Some(event.entity) {
            log::trace!(target: "animation", "Player animation event flags: {:?}", event.flags);
        }

        if event
            .flags
            .contains(AnimationEventFlags::EFFECT_WEAPON_ATTACK_HIT)
        {
            if let Ok(Command::Attack(command_attack)) = query_command.get(event.entity) {
                let hit_effect_file_id = query_equipment
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
                    })
                    .and_then(|effect_id| game_data.effect_database.get_effect(effect_id))
                    .and_then(|effect_data| effect_data.hit_normal);

                if let Some(hit_effect_file_id) = hit_effect_file_id {
                    spawn_effect_events.send(SpawnEffectEvent::AtEntity(
                        command_attack.target,
                        SpawnEffectData::with_file_id(hit_effect_file_id),
                    ));
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
                        SkillType::SelfBound
                        | SkillType::SelfBoundDuration
                        | SkillType::SelfStateDuration
                        | SkillType::SelfDamage => {
                            if let Some(effect_data) = skill_data
                                .bullet_effect_id
                                .and_then(|id| game_data.effect_database.get_effect(id))
                            {
                                if let Some(effect_file_id) = effect_data.bullet_normal {
                                    spawn_effect_events.send(SpawnEffectEvent::OnDummyBone(
                                        event.entity,
                                        skill_data.bullet_link_dummy_bone_id as usize,
                                        SpawnEffectData::with_file_id(effect_file_id),
                                    ));
                                }
                            }

                            if let Some(hit_effect_file_id) = skill_data.hit_effect_file_id {
                                spawn_effect_events.send(SpawnEffectEvent::OnDummyBone(
                                    event.entity,
                                    skill_data.hit_link_dummy_bone_id as usize,
                                    SpawnEffectData::with_file_id(hit_effect_file_id),
                                ));
                            }
                        }
                        SkillType::FireBullet
                        | SkillType::TargetBound
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
                                    if let Some(projectile_effect_file_id) =
                                        effect_data.bullet_normal
                                    {
                                        spawn_projectile_events.send(SpawnProjectileEvent {
                                            source: event.entity,
                                            source_dummy_bone_id: Some(
                                                skill_data.bullet_link_dummy_bone_id as usize,
                                            ),
                                            target: SpawnProjectileTarget::Entity(target_entity),
                                            move_type: effect_data
                                                .bullet_move_type
                                                .as_ref()
                                                .cloned()
                                                .unwrap_or(EffectBulletMoveType::Linear),
                                            move_speed: MoveSpeed::new(
                                                effect_data.bullet_speed / 100.0,
                                            ),
                                            projectile_effect_file_id: Some(
                                                projectile_effect_file_id,
                                            ),
                                            hit_effect_file_id: skill_data.hit_effect_file_id,
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
        if let Some(dummy_bone_id) = casting_effect.effect_dummy_bone_id {
            spawn_effect_events.send(SpawnEffectEvent::OnDummyBone(
                entity,
                dummy_bone_id,
                SpawnEffectData::with_file_id(casting_effect.effect_file_id),
            ));
        } else {
            spawn_effect_events.send(SpawnEffectEvent::OnEntity(
                entity,
                SpawnEffectData::with_file_id(casting_effect.effect_file_id),
            ));
        }
    }
}
