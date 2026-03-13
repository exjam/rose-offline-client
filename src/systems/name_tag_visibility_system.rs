use bevy::{
    ecs::query::WorldQuery,
    prelude::{Children, Entity, Local, Or, Query, Res, ResMut, Visibility, With},
};
use rose_game_common::components::Npc;

use crate::{
    components::{
        Dead, NameTag, NameTagEntity, NameTagHealthbarBackground, NameTagHealthbarForeground,
        NameTagTargetMark, NameTagType, PlayerCharacter,
    },
    resources::SelectedTarget,
    Config,
};

#[derive(Default)]
pub struct NameTagVisibility {
    pub hover: Option<Entity>,
    pub selected: Option<Entity>,
}

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct NameTagQuery<'w> {
    name_tag: &'w NameTag,
    children: &'w Children,
}

#[derive(WorldQuery)]
pub struct PlayerQuery {
    entity: Entity,
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

pub fn name_tag_visibility_system(
    mut state: Local<NameTagVisibility>,
    mut selected_target: ResMut<SelectedTarget>,
    mut query_visibility: Query<&mut Visibility>,
    query_name_tag: Query<NameTagQuery>,
    query_name_tag_entity: Query<&NameTagEntity>,
    query_name_tag_selected: Query<
        Entity,
        Or<(
            With<NameTagTargetMark>,
            With<NameTagHealthbarBackground>,
            With<NameTagHealthbarForeground>,
        )>,
    >,
    query_npc_dead: Query<&Dead, With<Npc>>,
    query_player: Query<PlayerQuery, With<PlayerCharacter>>,
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

    if state.hover != hover_name_tag_entity {
        if let Some(previous_entity) = state.hover.take() {
            if let Ok(name_tag) = query_name_tag.get(previous_entity) {
                // Restore unselected visibility
                if let Ok(mut visibility) = query_visibility.get_mut(previous_entity) {
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
            }
        }

        state.hover = hover_name_tag_entity;
    }

    if let Some(entity) = hover_name_tag_entity {
        // Name tag is always visible when hovered
        if let Ok(mut visibility) = query_visibility.get_mut(entity) {
            *visibility = Visibility::Inherited;
        }
    }

    if state.selected != selected_name_tag_entity {
        if let Some(previous_entity) = state.selected.take() {
            if let Ok(name_tag) = query_name_tag.get(previous_entity) {
                // Restore unselected visibility
                if let Ok(mut visibility) = query_visibility.get_mut(previous_entity) {
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
                    if query_name_tag_selected.contains(child) {
                        if let Ok(mut visibility) = query_visibility.get_mut(child) {
                            *visibility = Visibility::Hidden;
                        }
                    }
                }
            }
        }

        state.selected = selected_name_tag_entity;
    }

    if let Some(entity) = selected_name_tag_entity {
        if let Ok(name_tag) = query_name_tag.get(entity) {
            // Name tag is always visible when selected
            if let Ok(mut visibility) = query_visibility.get_mut(entity) {
                *visibility = Visibility::Inherited;
            }

            // All name tag children are visible when selected
            for &child in name_tag.children.iter() {
                if let Ok(mut visibility) = query_visibility.get_mut(child) {
                    *visibility = Visibility::Inherited;
                }
            }
        }
    }
}
