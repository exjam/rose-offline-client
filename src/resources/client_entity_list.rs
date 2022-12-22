use bevy::prelude::{Entity, Resource};

use rose_data::ZoneId;
use rose_game_common::messages::ClientEntityId;

#[derive(Resource)]
pub struct ClientEntityList {
    pub client_entities: Vec<Option<Entity>>,
    pub player_entity: Option<Entity>,
    pub player_entity_id: Option<ClientEntityId>,
    pub zone_id: Option<ZoneId>,
}

impl Default for ClientEntityList {
    fn default() -> Self {
        Self {
            client_entities: vec![None; u16::MAX as usize],
            player_entity: None,
            player_entity_id: None,
            zone_id: None,
        }
    }
}

impl ClientEntityList {
    pub fn add(&mut self, id: ClientEntityId, entity: Entity) {
        self.client_entities[id.0] = Some(entity);
    }

    pub fn remove(&mut self, id: ClientEntityId) {
        self.client_entities[id.0] = None;
    }

    pub fn clear(&mut self) {
        self.client_entities.fill(None);
    }

    pub fn get(&self, id: ClientEntityId) -> Option<Entity> {
        self.client_entities[id.0]
    }
}
