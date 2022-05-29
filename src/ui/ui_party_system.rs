use bevy::{
    ecs::query::WorldQuery,
    prelude::{EventReader, Local, Query, Res, ResMut, With},
};
use bevy_egui::{egui, EguiContext};

use rose_game_common::{
    components::{AbilityValues, CharacterInfo, HealthPoints, Level},
    messages::{
        client::ClientMessage, server::PartyMemberInfo, ClientEntityId, PartyRejectInviteReason,
    },
};

use crate::{
    components::{ClientEntity, ClientEntityName, PartyMembership, PartyOwner, PlayerCharacter},
    events::PartyEvent,
    resources::{ClientEntityList, GameConnection},
};

#[derive(WorldQuery)]
pub struct PlayerQuery<'w> {
    pub _player_character: With<PlayerCharacter>,
    pub ability_values: &'w AbilityValues,
    pub character_info: &'w CharacterInfo,
    pub health_points: &'w HealthPoints,
    pub level: &'w Level,
    pub party_membership: &'w PartyMembership,
}

#[derive(WorldQuery)]
pub struct PartyMemberQuery<'w> {
    pub character_info: &'w CharacterInfo,
    pub ability_values: &'w AbilityValues,
    pub health_points: &'w HealthPoints,
    pub level: &'w Level,
}

pub struct PendingPartyInvite {
    pub is_create: bool,
    pub client_entity_id: ClientEntityId,
    pub name: String,
}

#[derive(Default)]
pub struct UiStatePartySystem {
    pub pending_invites: Vec<PendingPartyInvite>,
}

pub fn ui_party_system(
    mut ui_state: Local<UiStatePartySystem>,
    mut egui_context: ResMut<EguiContext>,
    query_player: Query<PlayerQuery>,
    query_party_member: Query<PartyMemberQuery>,
    query_invite: Query<(&ClientEntity, &ClientEntityName)>,
    mut party_events: EventReader<PartyEvent>,
    game_connection: Option<Res<GameConnection>>,
    client_entity_list: Res<ClientEntityList>,
) {
    let player = query_player.single();

    // Add any new incoming invites
    for event in party_events.iter() {
        match *event {
            PartyEvent::InvitedCreate(entity) => {
                if let Ok((client_entity, client_entity_name)) = query_invite.get(entity) {
                    ui_state.pending_invites.push(PendingPartyInvite {
                        is_create: true,
                        client_entity_id: client_entity.id,
                        name: client_entity_name.name.clone(),
                    });
                }
            }
            PartyEvent::InvitedJoin(entity) => {
                if let Ok((client_entity, client_entity_name)) = query_invite.get(entity) {
                    ui_state.pending_invites.push(PendingPartyInvite {
                        is_create: false,
                        client_entity_id: client_entity.id,
                        name: client_entity_name.name.clone(),
                    });
                }
            }
        }
    }

    let mut i = 0;
    while i != ui_state.pending_invites.len() {
        let mut window_open = true;
        let mut accepted = false;
        let mut rejected = false;
        let pending_invite = &ui_state.pending_invites[i];

        if player.party_membership.is_none() {
            egui::Window::new("Party Invite")
                .id(egui::Id::new(format!(
                    "party_invite_{}",
                    &pending_invite.name
                )))
                .collapsible(false)
                .open(&mut window_open)
                .show(egui_context.ctx_mut(), |ui| {
                    ui.label(format!(
                        "{} has invited you to {} a party",
                        &pending_invite.name,
                        if pending_invite.is_create {
                            "create"
                        } else {
                            "join"
                        }
                    ));

                    if ui.button("Accept").clicked() {
                        accepted = true;
                    }

                    if ui.button("Reject").clicked() {
                        rejected = true;
                    }
                });
        } else {
            rejected = true;
        }

        if !window_open {
            rejected = true;
        }

        if accepted {
            if let Some(game_connection) = &game_connection {
                if pending_invite.is_create {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::PartyAcceptCreateInvite(
                            pending_invite.client_entity_id,
                        ))
                        .ok();
                } else {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::PartyAcceptJoinInvite(
                            pending_invite.client_entity_id,
                        ))
                        .ok();
                }
            }

            ui_state.pending_invites.remove(i);
            continue;
        } else if rejected {
            if let Some(game_connection) = &game_connection {
                game_connection
                    .client_message_tx
                    .send(ClientMessage::PartyRejectInvite(
                        PartyRejectInviteReason::Reject,
                        pending_invite.client_entity_id,
                    ))
                    .ok();
            }

            ui_state.pending_invites.remove(i);
            continue;
        }

        i += 1;
    }

    if let PartyMembership::Member(party_info) = player.party_membership {
        let style = egui_context.ctx_mut().style();
        let window_frame = egui::Frame::window(&style).fill(egui::Color32::from_rgba_unmultiplied(
            style.visuals.widgets.noninteractive.bg_fill.r(),
            style.visuals.widgets.noninteractive.bg_fill.g(),
            style.visuals.widgets.noninteractive.bg_fill.b(),
            128,
        ));

        egui::Window::new("Party")
            .anchor(egui::Align2::LEFT_CENTER, [10.0, -100.0])
            .collapsible(false)
            .title_bar(false)
            .frame(window_frame)
            .show(egui_context.ctx_mut(), |ui| {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.colored_label(egui::Color32::WHITE, &player.character_info.name);

                        ui.with_layout(egui::Layout::right_to_left(), |ui| {
                            if ui.button("Leave").clicked() {
                                if let Some(game_connection) = &game_connection {
                                    game_connection
                                        .client_message_tx
                                        .send(ClientMessage::PartyLeave)
                                        .ok();
                                }
                            }
                        });
                    });

                    ui.colored_label(
                        egui::Color32::WHITE,
                        format!("Level {} Visitor", player.level.level), // TODO: Use character_info.job
                    );
                    ui.scope(|ui| {
                        ui.style_mut().visuals.selection.bg_fill = egui::Color32::DARK_RED;
                        ui.add(
                            egui::ProgressBar::new(
                                player.health_points.hp as f32
                                    / player.ability_values.get_max_health() as f32,
                            )
                            .text(format!(
                                "{} / {}",
                                player.health_points.hp,
                                player.ability_values.get_max_health()
                            )),
                        )
                    });
                });

                for member in party_info.members.iter() {
                    ui.group(|ui| {
                        match member {
                            PartyMemberInfo::Online(member_info) => {
                                ui.horizontal(|ui| {
                                    ui.colored_label(egui::Color32::WHITE, &member_info.name);

                                    ui.with_layout(egui::Layout::right_to_left(), |ui| {
                                        if matches!(party_info.owner, PartyOwner::Player)
                                            && ui.button("Kick").clicked()
                                        {
                                            if let Some(game_connection) = &game_connection {
                                                game_connection
                                                    .client_message_tx
                                                    .send(ClientMessage::PartyKick(
                                                        member_info.character_id,
                                                    ))
                                                    .ok();
                                            }
                                        }
                                    });
                                });

                                if let Some(party_member) = client_entity_list
                                    .get(member_info.entity_id)
                                    .and_then(|entity| query_party_member.get(entity).ok())
                                {
                                    ui.colored_label(
                                        egui::Color32::WHITE,
                                        format!("Level {} Visitor", party_member.level.level),
                                    ); // TODO: Use character_info.job
                                    ui.scope(|ui| {
                                        ui.style_mut().visuals.selection.bg_fill =
                                            egui::Color32::DARK_RED;
                                        ui.add(
                                            egui::ProgressBar::new(
                                                party_member.health_points.hp as f32
                                                    / party_member.ability_values.get_max_health()
                                                        as f32,
                                            )
                                            .text(
                                                format!(
                                                    "{} / {}",
                                                    party_member.health_points.hp,
                                                    party_member.ability_values.get_max_health()
                                                ),
                                            ),
                                        )
                                    });
                                }
                            }
                            PartyMemberInfo::Offline(member_info) => {
                                ui.horizontal(|ui| {
                                    ui.colored_label(egui::Color32::WHITE, &member_info.name);

                                    ui.with_layout(egui::Layout::right_to_left(), |ui| {
                                        if matches!(party_info.owner, PartyOwner::Player)
                                            && ui.button("Kick").clicked()
                                        {
                                            if let Some(game_connection) = &game_connection {
                                                game_connection
                                                    .client_message_tx
                                                    .send(ClientMessage::PartyKick(
                                                        member_info.character_id,
                                                    ))
                                                    .ok();
                                            }
                                        }
                                    });
                                });

                                ui.colored_label(egui::Color32::WHITE, "Offline");
                            }
                        }
                    });
                }
            });
    }
}
