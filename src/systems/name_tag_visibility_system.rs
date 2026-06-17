use crate::{
    components::{
        ClientEntity, Dead, NameTag, NameTagEntity, NameTagHealthbarBackground,
        NameTagHealthbarForeground, NameTagTargetMark, NameTagType, PartyInfo, PlayerCharacter,
    },
    resources::SelectedTarget,
    Config,
};
use bevy::{
    ecs::query::WorldQuery,
    prelude::{Children, Entity, Local, Or, Query, Res, ResMut, Visibility, With},
};
use rose_game_common::components::Npc;

#[derive(Default)]
pub struct NameTagVisibility {
    pub hover: Option<Entity>,
    pub hover_name_tag: Option<Entity>,
    pub selected: Option<Entity>,
    pub selected_name_tag: Option<Entity>,
}

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct NameTagQuery<'w> {
    name_tag: &'w NameTag,
    children: &'w Children,
}

#[derive(WorldQuery)]
pub struct PlayerQuery<'w> {
    entity: Entity,
    party_info: Option<&'w PartyInfo>,
}

#[derive(WorldQuery)]
pub struct HealthBarQuery<'w> {
    pub foreground: Option<&'w NameTagHealthbarForeground>,
    pub background: Option<&'w NameTagHealthbarBackground>,
}

pub fn can_show_name_tag(
    player_entity: Option<Entity>,
    name_tag_entity: Entity,
    name_tag_type: NameTagType,
    config: &Config,
) -> bool {
    config.interface.name_tag_settings.show_all[name_tag_type]
        || player_entity.is_some_and(|player| player == name_tag_entity)
}

fn is_party_member(
    party_info: Option<&PartyInfo>,
    name_tag_client_entity: Option<&ClientEntity>,
) -> bool {
    let Some(party_info) = party_info else {
        return false;
    };

    let Some(name_tag_client_entity) = name_tag_client_entity else {
        return false;
    };

    party_info
        .members
        .iter()
        .find(|it| {
            it.get_client_entity_id()
                .is_some_and(|it| it == name_tag_client_entity.id)
        })
        .is_some()
}

pub fn can_show_full_name_tag(
    player_entity: Option<Entity>,
    party_info: Option<&PartyInfo>,
    entity: Entity,
    client_entity: Option<&ClientEntity>,
    selected: bool,
    name_tag_type: NameTagType,
    healthbar: Option<HealthBarQueryItem>,
    config: &Config,
) -> bool {
    if let Some(healthbar) = healthbar {
        if healthbar.foreground.is_none() && healthbar.background.is_none() {
            return selected;
        }
    }

    if player_entity.is_some_and(|it| it == entity) {
        return true;
    }

    if matches!(name_tag_type, NameTagType::Character) {
        return is_party_member(party_info, client_entity) && config.interface.party_hp_gauge;
    }

    selected
}

pub fn name_tag_visibility_system(
    mut state: Local<NameTagVisibility>,
    mut selected_target: ResMut<SelectedTarget>,
    mut query_visibility: Query<&mut Visibility>,
    query_healthbar: Query<HealthBarQuery>,
    query_name_tag: Query<NameTagQuery>,
    query_name_tag_entity: Query<&NameTagEntity>,
    query_name_tag_selected: Query<
        Entity,
        Or<(
            With<NameTagTargetMark>,
            With<NameTagHealthbarForeground>,
            With<NameTagHealthbarBackground>,
        )>,
    >,
    query_npc_dead: Query<&Dead, With<Npc>>,
    query_player: Query<PlayerQuery, With<PlayerCharacter>>,
    query_client_entity: Query<&ClientEntity>,
    config: Res<Config>,
) {
    let player = query_player.get_single().ok();

    if selected_target
        .selected
        .and_then(|entity| query_npc_dead.get(entity).ok())
        .is_some()
    {
        // Cannot select dead NPCs
        selected_target.selected = None;
    }

    if selected_target
        .hover
        .and_then(|entity| query_npc_dead.get(entity).ok())
        .is_some()
    {
        // Cannot hover dead NPCs
        selected_target.hover = None;
    }

    let hover_name_tag_entity = selected_target
        .hover
        .and_then(|entity| query_name_tag_entity.get(entity).ok())
        .map(|name_tag_entity| name_tag_entity.0);
    let selected_name_tag_entity = selected_target
        .selected
        .and_then(|entity| query_name_tag_entity.get(entity).ok())
        .map(|name_tag_entity| name_tag_entity.0);

    if state.hover_name_tag != hover_name_tag_entity {
        let mut handle_unhovered_entity = || {
            let Some(previous_entity) = state.hover.take() else {
                return;
            };

            let Some(previous_name_tag_entity) = state.hover_name_tag.take() else {
                return;
            };

            let Ok(name_tag) = query_name_tag.get(previous_name_tag_entity) else {
                return;
            };

            let Ok(mut visibility) = query_visibility.get_mut(previous_name_tag_entity) else {
                return;
            };

            // Restore unselected visibility
            if can_show_name_tag(
                player.as_ref().map(|player| player.entity),
                previous_entity,
                name_tag.name_tag.name_tag_type,
                &config,
            ) {
                *visibility = Visibility::Inherited;
            } else {
                *visibility = Visibility::Hidden;
            }
        };

        handle_unhovered_entity();
        state.hover = selected_target.hover;
        state.hover_name_tag = hover_name_tag_entity;
    }

    if let Some(entity) = hover_name_tag_entity {
        // Name tag is always visible when hovered
        if let Ok(mut visibility) = query_visibility.get_mut(entity) {
            *visibility = Visibility::Inherited;
        }
    }

    if state.selected_name_tag != selected_name_tag_entity {
        let mut handle_unselected_entity = || {
            let Some(previous_entity) = state.selected.take() else {
                return;
            };

            let Some(previous_name_tag_entity) = state.selected_name_tag.take() else {
                return;
            };

            let Ok(name_tag) = query_name_tag.get(previous_name_tag_entity) else {
                return;
            };

            // Restore unselected visibility
            if let Ok(mut visibility) = query_visibility.get_mut(previous_name_tag_entity) {
                if can_show_name_tag(
                    player.as_ref().map(|player| player.entity),
                    previous_entity,
                    name_tag.name_tag.name_tag_type,
                    &config,
                ) {
                    *visibility = Visibility::Inherited;
                } else {
                    *visibility = Visibility::Hidden;
                }
            }

            // Hide the name tag elements which should only be visible when selected
            for &child in name_tag.children.iter() {
                if !query_name_tag_selected.contains(child) {
                    continue;
                }

                let Ok(mut visibility) = query_visibility.get_mut(child) else {
                    continue;
                };

                if can_show_full_name_tag(
                    player.as_ref().map(|it| it.entity),
                    player.as_ref().and_then(|it| it.party_info),
                    previous_entity,
                    query_client_entity.get(previous_entity).ok(),
                    false,
                    name_tag.name_tag.name_tag_type,
                    query_healthbar.get(child).ok(),
                    &config,
                ) {
                    *visibility = Visibility::Inherited;
                } else {
                    *visibility = Visibility::Hidden;
                };
            }
        };

        handle_unselected_entity();
        state.selected = selected_target.selected;
        state.selected_name_tag = selected_name_tag_entity;
    }

    if let Some(selected_entity) = selected_target.selected {
        let mut handle_selected = || {
            let Some(selected_name_tag_entity) = selected_name_tag_entity else {
                return;
            };

            let Ok(name_tag) = query_name_tag.get(selected_name_tag_entity) else {
                return;
            };

            // Name tag is always visible when selected
            if let Ok(mut visibility) = query_visibility.get_mut(selected_name_tag_entity) {
                *visibility = Visibility::Inherited;
            }

            for &child in name_tag.children.iter() {
                let Ok(mut visibility) = query_visibility.get_mut(child) else {
                    continue;
                };

                if can_show_full_name_tag(
                    player.as_ref().map(|it| it.entity),
                    player.as_ref().and_then(|it| it.party_info),
                    selected_entity,
                    query_client_entity.get(selected_entity).ok(),
                    true,
                    name_tag.name_tag.name_tag_type,
                    query_healthbar.get(child).ok(),
                    &config,
                ) {
                    *visibility = Visibility::Inherited;
                } else {
                    *visibility = Visibility::Hidden;
                };
            }
        };

        handle_selected();
    }
}
