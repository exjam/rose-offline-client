use bevy::{
    math::Vec3,
    prelude::{
        Commands, ComputedVisibility, Entity, EventReader, EventWriter, GlobalTransform, Query,
        Res, Transform, Visibility,
    },
};
use rose_data::{AnimationEventFlags, EquipmentIndex, SkillData, SkillType};
use rose_game_common::components::{Equipment, MoveSpeed, Npc, Target};

use crate::{
    components::{Command, CommandCastSkillTarget, Projectile},
    events::{AnimationFrameEvent, SpawnEffectData, SpawnEffectEvent},
    resources::{ClientEntityList, GameData},
};

pub fn animation_effect_system(
    mut commands: Commands,
    mut animation_frame_events: EventReader<AnimationFrameEvent>,
    mut spawn_effect_events: EventWriter<SpawnEffectEvent>,
    query_command: Query<&Command>,
    query_equipment: Query<&Equipment>,
    query_npc: Query<&Npc>,
    query_transform: Query<&GlobalTransform>,
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
                        SkillType::TargetBound
                        | SkillType::TargetBoundDuration
                        | SkillType::TargetStateDuration
                        | SkillType::Resurrection => {
                            if let Some(CommandCastSkillTarget::Entity(target_entity)) =
                                command_cast_skill.skill_target
                            {
                                if let Ok(source_transform) = query_transform.get(event.entity) {
                                    if let Some(effect_data) = skill_data
                                        .bullet_effect_id
                                        .and_then(|id| game_data.effect_database.get_effect(id))
                                    {
                                        if let Some(projectile_effect_file_id) =
                                            effect_data.bullet_normal
                                        {
                                            // TODO: effect_data.bullet_move_type;
                                            let projectile_entity = commands
                                                .spawn_bundle((
                                                    Projectile::new(
                                                        event.entity,
                                                        skill_data.hit_effect_file_id,
                                                    ),
                                                    Target::new(target_entity),
                                                    MoveSpeed::new(
                                                        effect_data.bullet_speed / 100.0,
                                                    ),
                                                    Transform::from_translation(
                                                        source_transform.translation
                                                            + Vec3::new(0.0, 0.5, 0.0),
                                                    ),
                                                    GlobalTransform::default(),
                                                    Visibility::default(),
                                                    ComputedVisibility::default(),
                                                ))
                                                .id();

                                            spawn_effect_events.send(SpawnEffectEvent::OnEntity(
                                                projectile_entity,
                                                SpawnEffectData::with_file_id(
                                                    projectile_effect_file_id,
                                                ),
                                            ));
                                        }
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
