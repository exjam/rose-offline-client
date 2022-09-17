use bevy::{
    hierarchy::DespawnRecursiveExt,
    prelude::{AssetServer, Assets, Changed, Commands, Entity, Query, Res, ResMut},
};

use crate::{
    components::{PersonalStore, PersonalStoreModel, RemoveColliderCommand},
    model_loader::ModelLoader,
    render::ObjectMaterial,
};

pub fn personal_store_model_system(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            Option<&PersonalStore>,
            Option<&mut PersonalStoreModel>,
        ),
        Changed<PersonalStore>,
    >,
    asset_server: Res<AssetServer>,
    model_loader: Res<ModelLoader>,
    mut object_materials: ResMut<Assets<ObjectMaterial>>,
) {
    for (entity, personal_store, personal_store_model) in query.iter_mut() {
        if let Some(personal_store) = personal_store {
            if let Some(personal_store_model) = personal_store_model.as_ref() {
                if personal_store.skin == personal_store_model.skin {
                    // Nothing changed
                    continue;
                }

                // Despawn previous model
                commands
                    .entity(personal_store_model.model)
                    .despawn_recursive();
            }

            // Spawn new model
            let new_personal_store_model = model_loader.spawn_personal_store_model(
                &mut commands,
                &asset_server,
                &mut object_materials,
                entity,
                personal_store.skin,
            );

            if let Some(mut personal_store_model) = personal_store_model {
                *personal_store_model = new_personal_store_model;
            } else {
                commands.entity(entity).insert(new_personal_store_model);
            }

            commands.entity(entity).remove_and_despawn_collider();
        } else if let Some(personal_store_model) = personal_store_model {
            // Despawn and remove model
            commands
                .entity(personal_store_model.model)
                .despawn_recursive();
            commands
                .entity(entity)
                .remove::<PersonalStoreModel>()
                .remove_and_despawn_collider();
        }
    }
}
