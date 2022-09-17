use bevy::{
    ecs::system::EntityCommands,
    prelude::{despawn_with_children_recursive, Component, Deref, DerefMut, Entity, World},
};
use enum_map::Enum;

#[derive(Copy, Clone, Enum)]
pub enum NameTagType {
    Character,
    Monster,
    Npc,
}

#[derive(Component)]
pub struct NameTag {
    pub name_tag_type: NameTagType,
}

#[derive(Component)]
pub struct NameTagName;

#[derive(Component)]
pub struct NameTagTargetMark;

#[derive(Component)]
pub struct NameTagHealthbarForeground {
    pub uv_min_x: f32,
    pub uv_max_x: f32,
    pub full_width: f32,
}

#[derive(Component)]
pub struct NameTagHealthbarBackground;

#[derive(Component, Deref, DerefMut)]
pub struct NameTagEntity(pub Entity);

pub trait RemoveNameTagCommand {
    fn remove_and_despawn_name_tag(&mut self) -> &mut Self;
}

impl<'w, 's, 'a> RemoveNameTagCommand for EntityCommands<'w, 's, 'a> {
    fn remove_and_despawn_name_tag(&mut self) -> &mut Self {
        let entity = self.id();

        self.commands().add(move |world: &mut World| {
            let mut world_entity = world.entity_mut(entity);
            if let Some(nametag_entity) = world_entity.get::<NameTagEntity>() {
                let nametag_entity = nametag_entity.0;
                world_entity.remove::<NameTagEntity>();
                despawn_with_children_recursive(world, nametag_entity);
            }
        });

        self
    }
}
