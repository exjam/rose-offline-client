use bevy::{
    math::Vec3Swizzles,
    prelude::{Entity, EventReader, Query, Res, With},
};

use rose_data::{SkillBasicCommand, SkillType};
use rose_game_common::{
    components::{Hotbar, HotbarSlot, ItemDrop, SkillList, Team},
    messages::client::{Attack, ClientMessage, Move},
};

use crate::{
    components::{ClientEntity, PlayerCharacter, Position, SelectedTarget},
    events::PlayerCommandEvent,
    resources::{GameConnection, GameData},
};

#[allow(clippy::too_many_arguments)]
pub fn player_command_system(
    mut player_command_events: EventReader<PlayerCommandEvent>,
    query_client_entity: Query<(&ClientEntity, &Team)>,
    query_player: Query<
        (
            Entity,
            &Hotbar,
            &Position,
            &SkillList,
            &Team,
            Option<&SelectedTarget>,
        ),
        With<PlayerCharacter>,
    >,
    query_dropped_items: Query<(&ClientEntity, &Position), With<ItemDrop>>,
    game_connection: Option<Res<GameConnection>>,
    game_data: Res<GameData>,
) {
    let query_player_result = query_player.get_single();
    if query_player_result.is_err() {
        return;
    }
    let (
        _player_entity,
        player_hotbar,
        player_position,
        player_skill_list,
        player_team,
        player_selected_target,
    ) = query_player_result.unwrap();

    for mut event in player_command_events.iter().cloned() {
        if let PlayerCommandEvent::UseHotbar(page, index) = event {
            if let Some(hotbar_slot) = player_hotbar
                .pages
                .get(page)
                .and_then(|page| page.get(index))
                .and_then(|slot| slot.as_ref())
            {
                match hotbar_slot {
                    HotbarSlot::Skill(skill_slot) => {
                        event = PlayerCommandEvent::UseSkill(*skill_slot);
                    }
                    unimplemented => {
                        log::warn!("Unimplemented use hotbar slot {:?}", unimplemented);
                    }
                }
            }
        }

        match event {
            PlayerCommandEvent::UseSkill(skill_slot) => {
                if let Some(skill_data) = player_skill_list
                    .get_skill(skill_slot)
                    .and_then(|skill_id| game_data.skills.get_skill(skill_id))
                {
                    match skill_data.skill_type {
                        SkillType::BasicAction => match &skill_data.basic_command {
                            Some(SkillBasicCommand::Sit) => {
                                if let Some(game_connection) = game_connection.as_ref() {
                                    game_connection
                                        .client_message_tx
                                        .send(ClientMessage::SitToggle)
                                        .ok();
                                }
                            }
                            Some(SkillBasicCommand::PickupItem) => {
                                let mut nearest_item_drop = None;

                                for (item_client_entity, item_position) in
                                    query_dropped_items.iter()
                                {
                                    let distance = item_position
                                        .position
                                        .xy()
                                        .distance_squared(player_position.position.xy());

                                    if nearest_item_drop
                                        .as_ref()
                                        .map_or(true, |(nearest_distance, _, _)| {
                                            distance < *nearest_distance
                                        })
                                    {
                                        nearest_item_drop =
                                            Some((distance, item_position, item_client_entity.id));
                                    }
                                }

                                if let Some((_, target_position, target_entity_id)) =
                                    nearest_item_drop
                                {
                                    if let Some(game_connection) = game_connection.as_ref() {
                                        game_connection
                                            .client_message_tx
                                            .send(ClientMessage::Move(Move {
                                                target_entity_id: Some(target_entity_id),
                                                x: target_position.position.x,
                                                y: target_position.position.y,
                                                z: target_position.position.z as u16,
                                            }))
                                            .ok();
                                    }
                                }
                            }
                            Some(SkillBasicCommand::Attack) => {
                                if let Some(player_selected_target) = player_selected_target {
                                    if let Ok((target_client_entity, target_team)) =
                                        query_client_entity.get(player_selected_target.entity)
                                    {
                                        if target_team.id != Team::DEFAULT_NPC_TEAM_ID
                                            && target_team.id != player_team.id
                                        {
                                            if let Some(game_connection) = game_connection.as_ref()
                                            {
                                                game_connection
                                                    .client_message_tx
                                                    .send(ClientMessage::Attack(Attack {
                                                        target_entity_id: target_client_entity.id,
                                                    }))
                                                    .ok();
                                            }
                                        }
                                    }
                                }
                            }
                            /*
                            Some(SkillBasicCommand::AutoTarget) => {}
                            Some(SkillBasicCommand::Jump) => {}
                            Some(SkillBasicCommand::AirJump) => {}
                            Some(SkillBasicCommand::DriveVehicle) => {}
                            Some(SkillBasicCommand::AddFriend) => {}
                            Some(SkillBasicCommand::PartyInvite) => {}
                            Some(SkillBasicCommand::Trade) => {}
                            Some(SkillBasicCommand::PrivateStore) => {}
                            Some(SkillBasicCommand::SelfTarget) => {}
                            Some(SkillBasicCommand::VehiclePassengerInvite) => {}
                            */
                            Some(unimplemented) => {
                                log::warn!(
                                    "Unimplemented skill basic command type: {:?}",
                                    unimplemented
                                );
                            }
                            None => {}
                        },

                        SkillType::Emote => {
                            if let Some(motion_id) = skill_data.action_motion_id {
                                if let Some(game_connection) = game_connection.as_ref() {
                                    game_connection
                                        .client_message_tx
                                        .send(ClientMessage::UseEmote(motion_id, true))
                                        .ok();
                                }
                            }
                        }

                        SkillType::CreateWindow => {
                            log::warn!("Unimplemented skill type: {:?}", skill_data.skill_type);
                        }

                        SkillType::SelfBoundDuration
                        | SkillType::SelfBound
                        | SkillType::SelfStateDuration
                        | SkillType::SummonPet
                        | SkillType::SelfDamage => {
                            log::warn!(
                                "Unimplemented self skill type: {:?}",
                                skill_data.skill_type
                            );
                        }

                        SkillType::AreaTarget => {
                            log::warn!(
                                "Unimplemented target position skill type: {:?}",
                                skill_data.skill_type
                            );
                        }

                        SkillType::EnforceWeapon
                        | SkillType::Immediate
                        | SkillType::TargetBound
                        | SkillType::TargetBoundDuration
                        | SkillType::TargetStateDuration
                        | SkillType::SelfAndTarget
                        | SkillType::Resurrection
                        | SkillType::EnforceBullet
                        | SkillType::FireBullet => {
                            log::warn!(
                                "Unimplemented target entity skill type: {:?}",
                                skill_data.skill_type
                            );
                        }

                        SkillType::Passive => {} // Do nothing for passive skills
                        SkillType::Warp => {} // Warp skill is only used on items, so we should never hit it here
                    }
                }
            }
            PlayerCommandEvent::Attack(entity) => {
                if let Ok((target_client_entity, target_team)) = query_client_entity.get(entity) {
                    if target_team.id != Team::DEFAULT_NPC_TEAM_ID
                        && target_team.id != player_team.id
                    {
                        if let Some(game_connection) = game_connection.as_ref() {
                            game_connection
                                .client_message_tx
                                .send(ClientMessage::Attack(Attack {
                                    target_entity_id: target_client_entity.id,
                                }))
                                .ok();
                        }
                    }
                }
            }
            PlayerCommandEvent::Move(position, target_entity) => {
                let target_entity_id = target_entity
                    .and_then(|target_entity| query_client_entity.get(target_entity).ok())
                    .map(|(target_client_entity, _target_team)| target_client_entity.id);

                if let Some(game_connection) = game_connection.as_ref() {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::Move(Move {
                            target_entity_id,
                            x: position.position.x,
                            y: position.position.y,
                            z: position.position.z as u16,
                        }))
                        .ok();
                }
            }
            PlayerCommandEvent::UseHotbar(_, _) => {} // Handled above
        }
    }
}
