use bevy::{
    prelude::{
        AssetServer, Assets, BuildChildren, Commands, ComputedVisibility, GlobalTransform, Handle,
        Resource, Transform, Vec3, Visibility,
    },
    render::primitives::Aabb,
};

use crate::{
    animation::{TransformAnimation, ZmoAsset},
    components::DamageDigits,
    render::{DamageDigitMaterial, DamageDigitRenderData},
};

#[derive(Resource)]
pub struct DamageDigitsSpawner {
    pub texture_damage: Handle<DamageDigitMaterial>,
    pub texture_damage_player: Handle<DamageDigitMaterial>,
    pub texture_miss: Handle<DamageDigitMaterial>,
    pub motion: Handle<ZmoAsset>,
}

impl DamageDigitsSpawner {
    pub fn load(
        asset_server: &AssetServer,
        damage_digit_materials: &mut Assets<DamageDigitMaterial>,
    ) -> Self {
        Self {
            texture_damage: damage_digit_materials.add(DamageDigitMaterial {
                texture: asset_server.load("3DDATA/EFFECT/SPECIAL/DIGITNUMBER01.DDS.rgb_texture"),
            }),
            texture_damage_player: damage_digit_materials.add(DamageDigitMaterial {
                texture: asset_server.load("3DDATA/EFFECT/SPECIAL/DIGITNUMBER02.DDS.rgb_texture"),
            }),
            texture_miss: damage_digit_materials.add(DamageDigitMaterial {
                texture: asset_server.load("3DDATA/EFFECT/SPECIAL/DIGITNUMBERMISS.DDS.rgb_texture"),
            }),
            motion: asset_server.load("3DDATA/EFFECT/SPECIAL/HIT_FIGURE_01.ZMO"),
        }
    }

    pub fn spawn(
        &self,
        commands: &mut Commands,
        global_transform: &GlobalTransform,
        model_height: f32,
        damage: u32,
        is_damage_player: bool,
    ) {
        let (scale, _, translation) = global_transform.to_scale_rotation_translation();

        // We need to spawn inside a parent entity for positioning because the ActiveMotion will set the translation absolutely
        commands
            .spawn((
                Transform::from_translation(
                    translation + Vec3::new(0.0, model_height * scale.y, 0.0),
                ),
                GlobalTransform::default(),
                Visibility::default(),
                ComputedVisibility::default(),
            ))
            .with_children(|child_builder| {
                child_builder.spawn((
                    DamageDigits { damage },
                    DamageDigitRenderData::new(4),
                    if damage == 0 {
                        self.texture_miss.clone_weak()
                    } else if is_damage_player {
                        self.texture_damage_player.clone_weak()
                    } else {
                        self.texture_damage.clone_weak()
                    },
                    TransformAnimation::once(self.motion.clone_weak()),
                    Transform::default(),
                    GlobalTransform::default(),
                    Aabb::default(),
                    Visibility::default(),
                    ComputedVisibility::default(),
                ));
            });
    }
}
