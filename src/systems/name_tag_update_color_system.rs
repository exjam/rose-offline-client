use bevy::{
    ecs::query::WorldQuery,
    prelude::{Changed, Children, Color, Entity, Or, Parent, Query, With},
};

use rose_game_common::components::{Level, Team};

use crate::{
    components::{NameTag, NameTagName, NameTagType, PlayerCharacter},
    render::WorldUiRect,
    systems::name_tag_system::get_monster_name_tag_color,
};

#[derive(WorldQuery)]
pub struct PlayerQuery<'w> {
    level: &'w Level,
    team: &'w Team,
}

pub fn name_tag_update_color_system(
    query_player: Query<PlayerQuery, (With<PlayerCharacter>, Or<(Changed<Level>, Changed<Team>)>)>,
    query_nametags: Query<(&Parent, &NameTag, &Children)>,
    query_level: Query<&Level>,
    query_team: Query<&Team>,
    mut query_name_rects: Query<&mut WorldUiRect, With<NameTagName>>,
) {
    let player = if let Ok(player) = query_player.get_single() {
        player
    } else {
        return;
    };

    for (parent, nametag, children) in query_nametags.iter() {
        let color = match nametag.name_tag_type {
            NameTagType::Npc => continue,
            NameTagType::Character => {
                if query_team
                    .get(parent.get())
                    .map_or(false, |team| team.id != player.team.id)
                {
                    Color::RED
                } else {
                    Color::WHITE
                }
            }
            NameTagType::Monster => {
                let color = get_monster_name_tag_color(
                    Some(player.level),
                    query_level.get(parent.get()).ok(),
                    query_team.get(parent.get()).ok(),
                )
                .to_array();

                Color::rgb_linear(
                    color[0] as f32 / 255.0,
                    color[1] as f32 / 255.0,
                    color[2] as f32 / 255.0,
                )
            }
        };

        for &child_entity in children.iter() {
            if let Ok(mut rect) = query_name_rects.get_mut(child_entity) {
                rect.color = color;
            }
        }
    }
}
