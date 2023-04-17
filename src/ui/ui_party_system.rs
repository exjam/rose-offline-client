use bevy::{
    ecs::query::WorldQuery,
    prelude::{Assets, Entity, EventReader, EventWriter, Local, Query, Res, ResMut, With},
};
use bevy_egui::{egui, EguiContexts};

use rose_game_common::{
    components::{AbilityValues, CharacterInfo, HealthPoints, Level},
    messages::{
        client::ClientMessage, server::PartyMemberInfo, ClientEntityId, PartyRejectInviteReason,
    },
};

use crate::{
    components::{ClientEntity, ClientEntityName, PartyInfo, PartyOwner, PlayerCharacter},
    events::PartyEvent,
    resources::{ClientEntityList, GameConnection, SelectedTarget, UiResources},
    ui::{
        widgets::{Dialog, Gauge},
        UiSoundEvent,
    },
};

use super::{
    widgets::{DrawText, DrawWidget, LoadWidget},
    DataBindings, UiStateWindows,
};

const IID_BTN_ENTRUST: i32 = 11;
const IID_BTN_BAN: i32 = 12;
const IID_BTN_LEAVE: i32 = 13;
const IID_BTN_OPTION: i32 = 14;
const IID_PARTY_XP_GAUGE: i32 = 1001;
const IID_PARTY_MEMBER_HP_GAUGE: i32 = 1002;

#[derive(WorldQuery)]
pub struct PlayerQuery<'w> {
    _player_character: With<PlayerCharacter>,
    entity: Entity,
    ability_values: &'w AbilityValues,
    character_info: &'w CharacterInfo,
    health_points: &'w HealthPoints,
    level: &'w Level,
    party_info: Option<&'w PartyInfo>,
}

#[derive(WorldQuery)]
pub struct PartyMemberQuery<'w> {
    character_info: &'w CharacterInfo,
    ability_values: &'w AbilityValues,
    health_points: &'w HealthPoints,
    level: &'w Level,
}

pub struct PendingPartyInvite {
    is_create: bool,
    client_entity_id: ClientEntityId,
    name: String,
}

pub struct UiStatePartySystem {
    pending_invites: Vec<PendingPartyInvite>,
    party_xp_gauge: Gauge,
    party_member_health_gauge: Gauge,
    selected_party_member_index: Option<usize>,
}

impl Default for UiStatePartySystem {
    fn default() -> Self {
        Self {
            pending_invites: Default::default(),
            party_xp_gauge: Gauge {
                id: IID_PARTY_XP_GAUGE,
                x: 96.0,
                y: 34.0,
                width: 111.0,
                height: 9.0,
                module_id: 0,
                foreground_sprite_name: "UI18_GUAGE_PARTYLEVEL".into(),
                background_sprite_name: "UI18_GUAGE_PARTYLEVEL_BASE".into(),
                ..Default::default()
            },
            party_member_health_gauge: Gauge {
                id: IID_PARTY_MEMBER_HP_GAUGE,
                width: 119.0,
                height: 9.0,
                module_id: 0,
                foreground_sprite_name: "UI18_GUAGE_HP".into(),
                background_sprite_name: "UI18_GUAGE_HP_BASE".into(),
                ..Default::default()
            },
            selected_party_member_index: None,
        }
    }
}

pub fn ui_party_system(
    mut ui_state: Local<UiStatePartySystem>,
    mut ui_state_windows: ResMut<UiStateWindows>,
    mut ui_sound_events: EventWriter<UiSoundEvent>,
    mut egui_context: EguiContexts,
    query_player: Query<PlayerQuery>,
    query_party_member: Query<PartyMemberQuery>,
    query_invite: Query<(&ClientEntity, &ClientEntityName)>,
    mut party_events: EventReader<PartyEvent>,
    game_connection: Option<Res<GameConnection>>,
    client_entity_list: Res<ClientEntityList>,
    ui_resources: Res<UiResources>,
    dialog_assets: Res<Assets<Dialog>>,
    mut selected_target: ResMut<SelectedTarget>,
) {
    let player = if let Ok(player) = query_player.get_single() {
        player
    } else {
        return;
    };

    // Add any new incoming invites
    for event in party_events.iter() {
        match *event {
            PartyEvent::InvitedCreate(entity) => {
                if let Ok((client_entity, client_entity_name)) = query_invite.get(entity) {
                    ui_state.pending_invites.push(PendingPartyInvite {
                        is_create: true,
                        client_entity_id: client_entity.id,
                        name: client_entity_name.to_string(),
                    });
                }
            }
            PartyEvent::InvitedJoin(entity) => {
                if let Ok((client_entity, client_entity_name)) = query_invite.get(entity) {
                    ui_state.pending_invites.push(PendingPartyInvite {
                        is_create: false,
                        client_entity_id: client_entity.id,
                        name: client_entity_name.to_string(),
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

        if player.party_info.is_none() {
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

    let dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_party) {
        if ui_state.party_xp_gauge.foreground_sprite.is_none() {
            ui_state.party_xp_gauge.load_widget(&ui_resources);
        }

        if ui_state
            .party_member_health_gauge
            .foreground_sprite
            .is_none()
        {
            ui_state
                .party_member_health_gauge
                .load_widget(&ui_resources);
        }

        dialog
    } else {
        return;
    };

    let mut response_entrust_button = None;
    let mut response_kick_button = None;
    let mut response_leave_button = None;
    let mut response_option_button = None;

    ui_state_windows.party_open = player.party_info.is_some();

    if let Some(party_info) = player.party_info {
        let player_is_owner = matches!(party_info.owner, PartyOwner::Player);

        egui::Window::new("Party2")
            .anchor(egui::Align2::RIGHT_CENTER, [0.0, 0.0])
            .frame(egui::Frame::none())
            .title_bar(false)
            .resizable(false)
            .default_width(dialog.width)
            .default_height(dialog.height)
            .show(egui_context.ctx_mut(), |ui| {
                dialog.draw(
                    ui,
                    DataBindings {
                        sound_events: Some(&mut ui_sound_events),
                        gauge: &mut [(IID_PARTY_XP_GAUGE, &0.5, "50%")],
                        response: &mut [
                            (IID_BTN_ENTRUST, &mut response_entrust_button),
                            (IID_BTN_BAN, &mut response_kick_button),
                            (IID_BTN_LEAVE, &mut response_leave_button),
                            (IID_BTN_OPTION, &mut response_option_button),
                        ],
                        visible: &mut [
                            (IID_BTN_BAN, player_is_owner),
                            (IID_BTN_ENTRUST, player_is_owner),
                        ],
                        ..Default::default()
                    },
                    |ui, bindings| {
                        ui.add_label_at(
                            egui::pos2(35.0, 7.0),
                            egui::RichText::new("Party").color(egui::Color32::BLACK),
                        );

                        ui.add_label_at(egui::pos2(17.0, 34.0), format!("Party Level: {}", 1));

                        ui_state.party_xp_gauge.draw_widget(ui, bindings);

                        ui.vertical(|ui| {
                            for (index, member) in party_info.members.iter().enumerate() {
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::vec2(220.0, 45.0),
                                    egui::Sense::click(),
                                );
                                {
                                    let ui = &mut ui.child_ui(rect, egui::Layout::default());
                                    let selected =
                                        ui_state.selected_party_member_index == Some(index);
                                    let (online, name) = match member {
                                        PartyMemberInfo::Online(member_info) => {
                                            if let Some(party_member) = client_entity_list
                                                .get(member_info.entity_id)
                                                .and_then(|entity| {
                                                    query_party_member.get(entity).ok()
                                                })
                                            {
                                                let hp_percent = party_member.health_points.hp
                                                    as f32
                                                    / party_member.ability_values.get_max_health()
                                                        as f32;

                                                ui_state.party_member_health_gauge.x = 220.0
                                                    - ui_state.party_member_health_gauge.width;
                                                ui_state.party_member_health_gauge.y = 25.0;
                                                ui_state.party_member_health_gauge.draw_widget(
                                                    ui,
                                                    &mut DataBindings {
                                                        gauge: &mut [(
                                                            IID_PARTY_MEMBER_HP_GAUGE,
                                                            &hp_percent,
                                                            &format!("{:.2}%", 100.0 * hp_percent),
                                                        )],
                                                        ..Default::default()
                                                    },
                                                );
                                            }

                                            (true, &member_info.name)
                                        }
                                        PartyMemberInfo::Offline(member_info) => {
                                            (false, &member_info.name)
                                        }
                                    };

                                    ui.add_label_at(
                                        egui::pos2(4.0, 26.0),
                                        egui::RichText::new(name).color(egui::Color32::BLACK),
                                    );
                                    ui.add_label_at(
                                        egui::pos2(3.0, 25.0),
                                        egui::RichText::new(name).color(if selected {
                                            egui::Color32::RED
                                        } else if online {
                                            egui::Color32::WHITE
                                        } else {
                                            egui::Color32::GRAY
                                        }),
                                    );
                                }

                                if response.clicked() {
                                    if let Some(entity) = member
                                        .get_client_entity_id()
                                        .and_then(|entity_id| client_entity_list.get(entity_id))
                                    {
                                        selected_target.selected = Some(entity);
                                    }

                                    ui_state.selected_party_member_index = Some(index);
                                }
                            }
                        });
                    },
                );
            });

        if player_is_owner {
            if let Some(selected_party_member) = ui_state
                .selected_party_member_index
                .and_then(|index| party_info.members.get(index))
            {
                if player.character_info.unique_id != selected_party_member.get_character_id() {
                    if response_kick_button.as_ref().map_or(false, |x| x.clicked()) {
                        if let Some(game_connection) = &game_connection {
                            game_connection
                                .client_message_tx
                                .send(ClientMessage::PartyKick(
                                    selected_party_member.get_character_id(),
                                ))
                                .ok();
                        }
                    }

                    if let Some(selected_client_entity_id) =
                        selected_party_member.get_client_entity_id()
                    {
                        if response_entrust_button
                            .as_ref()
                            .map_or(false, |x| x.clicked())
                        {
                            if let Some(game_connection) = &game_connection {
                                game_connection
                                    .client_message_tx
                                    .send(ClientMessage::PartyChangeOwner(
                                        selected_client_entity_id,
                                    ))
                                    .ok();
                            }
                        }
                    }
                }
            }
        }

        if response_leave_button
            .as_ref()
            .map_or(false, |x| x.clicked())
        {
            if let Some(game_connection) = &game_connection {
                game_connection
                    .client_message_tx
                    .send(ClientMessage::PartyLeave)
                    .ok();
            }
        }

        if response_option_button
            .as_ref()
            .map_or(false, |x| x.clicked())
        {
            ui_state_windows.party_options_open = !ui_state_windows.party_options_open;
        }

        if let Some(button) = response_entrust_button {
            button.on_hover_text("Entrust as Leader");
        }

        if let Some(button) = response_kick_button {
            button.on_hover_text("Kick Member");
        }

        if let Some(button) = response_leave_button {
            button.on_hover_text("Leave Party");
        }

        if let Some(button) = response_option_button {
            button.on_hover_text("Party Options");
        }
    }
}
