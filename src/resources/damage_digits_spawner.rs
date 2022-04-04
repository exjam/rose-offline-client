use bevy::{
    math::Vec3,
    prelude::{
        AssetServer, Assets, Commands, ComputedVisibility, Entity, GlobalTransform, Handle,
        Transform, Visibility,
    },
    render::primitives::Aabb,
};

use crate::{
    components::{ActiveMotion, DamageDigits},
    render::{DamageDigitMaterial, DamageDigitRenderData},
    zmo_asset_loader::ZmoAsset,
};

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
                texture: asset_server.load("3DDATA/EFFECT/SPECIAL/DIGITNUMBER01.DDS"),
            }),
            texture_damage_player: damage_digit_materials.add(DamageDigitMaterial {
                texture: asset_server.load("3DDATA/EFFECT/SPECIAL/DIGITNUMBER02.DDS"),
            }),
            texture_miss: damage_digit_materials.add(DamageDigitMaterial {
                texture: asset_server.load("3DDATA/EFFECT/SPECIAL/DIGITNUMBERMISS.DDS"),
            }),
            motion: asset_server.load("3DDATA/EFFECT/SPECIAL/HIT_FIGURE_01.ZMO"),
        }
    }

    pub fn spawn(
        &self,
        commands: &mut Commands,
        damage: u32,
        is_damage_player: bool,
        model_height: f32,
    ) -> Option<Entity> {
        Some(
            commands
                .spawn_bundle((
                    DamageDigits {
                        damage,
                        model_height,
                    },
                    DamageDigitRenderData::new(4),
                    if damage == 0 {
                        self.texture_miss.clone_weak()
                    } else if is_damage_player {
                        self.texture_damage_player.clone_weak()
                    } else {
                        self.texture_damage.clone_weak()
                    },
                    ActiveMotion::new_once(self.motion.clone_weak()),
                    Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
                    GlobalTransform::default(),
                    Aabb::default(),
                    Visibility::default(),
                    ComputedVisibility::default(),
                ))
                .id(),
        )
    }
}
