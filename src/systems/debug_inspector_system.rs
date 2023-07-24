use bevy::{
    input::Input,
    prelude::{
        App, Camera, Camera3d, GlobalTransform, KeyCode, Plugin, Query, Res, ResMut, Update, With,
    },
    window::{PrimaryWindow, Window},
};
use bevy_egui::EguiContexts;
use bevy_rapier3d::prelude::{CollisionGroups, Group, QueryFilter, RapierContext};

use rose_game_common::{components::*, messages::ClientEntityId};

use crate::{
    components::*,
    render::{ObjectMaterialBlend, ObjectMaterialGlow},
    resources::DebugInspector,
};

pub struct DebugInspectorPlugin;

impl Plugin for DebugInspectorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DebugInspector::default())
            .add_systems(Update, debug_inspector_picking_system);

        app.register_type::<rose_data::MotionId>()
            .register_type::<rose_data::NpcId>()
            .register_type::<rose_data::SkillId>()
            .register_type::<rose_data::WarpGateId>()
            .register_type::<rose_data::ZoneId>();

        app.register_type::<AbilityValues>()
            .register_type::<AbilityValuesAdjust>()
            .register_type::<BasicStats>()
            .register_type::<CharacterBlinkTimer>()
            .register_type::<CharacterGender>()
            .register_type::<CharacterInfo>()
            .register_type::<ClientEntity>()
            .register_type::<ClientEntityId>()
            .register_type::<ClientEntityName>()
            .register_type::<ClientEntityType>()
            .register_type::<ColliderEntity>()
            .register_type::<ColliderParent>()
            .register_type::<Command>()
            .register_type::<CommandAttack>()
            .register_type::<CommandCastSkill>()
            .register_type::<CommandCastSkillState>()
            .register_type::<CommandCastSkillTarget>()
            .register_type::<CommandEmote>()
            .register_type::<CommandMove>()
            .register_type::<CommandSit>()
            .register_type::<DamageCategory>()
            .register_type::<DamageType>()
            .register_type::<Dead>()
            .register_type::<DummyBoneOffset>()
            .register_type::<Effect>()
            .register_type::<EffectMesh>()
            .register_type::<EffectParticle>()
            .register_type::<EventObject>()
            .register_type::<ExperiencePoints>()
            .register_type::<FacingDirection>()
            .register_type::<HealthPoints>()
            .register_type::<Level>()
            .register_type::<ManaPoints>()
            .register_type::<ModelHeight>()
            .register_type::<MoveMode>()
            .register_type::<MoveSpeed>()
            .register_type::<NextCommand>()
            .register_type::<NightTimeEffect>()
            .register_type::<Npc>()
            .register_type::<ObjectMaterialBlend>()
            .register_type::<ObjectMaterialGlow>()
            .register_type::<PassiveRecoveryTime>()
            .register_type::<PersonalStore>()
            .register_type::<PersonalStoreModel>()
            .register_type::<PlayerCharacter>()
            .register_type::<Position>()
            .register_type::<SkillPoints>()
            .register_type::<SoundCategory>()
            .register_type::<Stamina>()
            .register_type::<StatPoints>()
            .register_type::<Team>()
            .register_type::<UnionMembership>()
            .register_type::<WarpObject>()
            .register_type::<ZoneObject>()
            .register_type::<ZoneObjectAnimatedObject>()
            .register_type::<ZoneObjectId>()
            .register_type::<ZoneObjectPart>()
            .register_type::<ZoneObjectPartCollisionShape>()
            .register_type::<ZoneObjectTerrain>();

        app.add_plugins(bevy_inspector_egui::DefaultInspectorConfigPlugin);
    }
}

#[allow(clippy::too_many_arguments)]
fn debug_inspector_picking_system(
    mut debug_inspector_state: ResMut<DebugInspector>,
    mut egui_ctx: EguiContexts,
    key_code_input: Res<Input<KeyCode>>,
    rapier_context: Res<RapierContext>,
    query_window: Query<&Window, With<PrimaryWindow>>,
    query_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
) {
    if !debug_inspector_state.enable_picking {
        // Picking disabled
        return;
    }

    let Ok(window) = query_window.get_single() else {
        return;
    };

    let cursor_position = window.cursor_position();
    if cursor_position.is_none() || egui_ctx.ctx_mut().wants_pointer_input() {
        // Mouse not in window, or is over UI
        return;
    }
    let cursor_position = cursor_position.unwrap();

    if key_code_input.just_pressed(KeyCode::P) {
        for (camera, camera_transform) in query_camera.iter() {
            if let Some(ray) = camera.viewport_to_world(camera_transform, cursor_position) {
                if let Some((collider_entity, _distance)) = rapier_context.cast_ray(
                    ray.origin,
                    ray.direction,
                    10000000.0,
                    false,
                    QueryFilter::new().groups(CollisionGroups::new(
                        COLLISION_FILTER_INSPECTABLE,
                        Group::all(),
                    )),
                ) {
                    debug_inspector_state.entity = Some(collider_entity);
                }
            }
        }
    }
}
