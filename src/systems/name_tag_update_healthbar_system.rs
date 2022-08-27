use bevy::prelude::{Parent, Query};

use rose_game_common::components::{AbilityValues, HealthPoints};

use crate::{components::NameTagHealthbarForeground, render::WorldUiRect};

pub fn name_tag_update_healthbar_system(
    mut query_nametag_healthbar: Query<(&Parent, &NameTagHealthbarForeground, &mut WorldUiRect)>,
    query_parent: Query<&Parent>,
    query_health: Query<(&HealthPoints, &AbilityValues)>,
) {
    for (parent, name_tag_healthbar_fg, mut rect) in query_nametag_healthbar.iter_mut() {
        if let Ok((health_points, ability_values)) = query_parent
            .get(parent.get())
            .and_then(|parent| query_health.get(parent.get()))
        {
            let health_percent =
                (health_points.hp as f32 / ability_values.get_max_health() as f32).max(0.0);

            rect.uv_max.x = name_tag_healthbar_fg.uv_min_x
                + health_percent
                    * (name_tag_healthbar_fg.uv_max_x - name_tag_healthbar_fg.uv_min_x);
            rect.screen_size.x = name_tag_healthbar_fg.full_width * health_percent;
        }
    }
}
