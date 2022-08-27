use bevy::{
    ecs::query::WorldQuery,
    prelude::{Children, Entity, Local, Or, Query, Res, ResMut, Visibility, With},
};
use rose_game_common::components::Npc;

use crate::{
    components::{
        Command, NameTag, NameTagEntity, NameTagHealthbarBackground, NameTagHealthbarForeground,
        NameTagTargetMark,
    },
    resources::{NameTagSettings, SelectedTarget},
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
    query_npc_command: Query<&Command, With<Npc>>,
    name_tag_settings: Res<NameTagSettings>,
) {
    if selected_target
        .selected
        .and_then(|entity| query_npc_command.get(entity).ok())
        .map_or(false, |command| command.is_die())
    {
        // Cannot select dead NPCs
        selected_target.selected = None;
    }

    if selected_target
        .hover
        .and_then(|entity| query_npc_command.get(entity).ok())
        .map_or(false, |command| command.is_die())
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
                    visibility.is_visible =
                        name_tag_settings.show_all[name_tag.name_tag.name_tag_type];
                }
            }
        }

        state.hover = hover_name_tag_entity;
    }

    if let Some(entity) = hover_name_tag_entity {
        // Name tag is always visible when hovered
        if let Ok(mut visibility) = query_visibility.get_mut(entity) {
            visibility.is_visible = true;
        }
    }

    if state.selected != selected_name_tag_entity {
        if let Some(previous_entity) = state.selected.take() {
            if let Ok(name_tag) = query_name_tag.get(previous_entity) {
                // Restore unselected visibility
                if let Ok(mut visibility) = query_visibility.get_mut(previous_entity) {
                    visibility.is_visible =
                        name_tag_settings.show_all[name_tag.name_tag.name_tag_type];
                }

                // Hide the name tag elements which should only be visible when selected
                for &child in name_tag.children.iter() {
                    if query_name_tag_selected.contains(child) {
                        if let Ok(mut visibility) = query_visibility.get_mut(child) {
                            visibility.is_visible = false;
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
                visibility.is_visible = true;
            }

            // All name tag children are visible when selected
            for &child in name_tag.children.iter() {
                if let Ok(mut visibility) = query_visibility.get_mut(child) {
                    visibility.is_visible = true;
                }
            }
        }
    }
}
