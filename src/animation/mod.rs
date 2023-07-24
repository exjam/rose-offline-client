use bevy::{
    prelude::{
        AddAsset, App, IntoSystemConfigs, IntoSystemSetConfig, Plugin, PostUpdate, SystemSet,
    },
    transform::TransformSystem,
};

mod animation_state;
mod camera_animation;
mod mesh_animation;
mod skeletal_animation;
mod transform_animation;
mod zmo_asset_loader;

pub use animation_state::AnimationFrameEvent;
pub use camera_animation::CameraAnimation;
pub use mesh_animation::MeshAnimation;
pub use skeletal_animation::SkeletalAnimation;
pub use transform_animation::TransformAnimation;
pub use zmo_asset_loader::{
    ZmoAsset, ZmoAssetAnimationTexture, ZmoAssetBone, ZmoAssetLoader, ZmoTextureAssetLoader,
};

use animation_state::AnimationState;
use camera_animation::camera_animation_system;
use mesh_animation::mesh_animation_system;
use skeletal_animation::skeletal_animation_system;
use transform_animation::transform_animation_system;

#[derive(Default)]
pub struct RoseAnimationPlugin;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct RoseAnimationSystem;

impl Plugin for RoseAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<ZmoAsset>()
            .register_asset_reflect::<ZmoAsset>()
            .register_type::<ZmoAssetAnimationTexture>()
            .register_type::<ZmoAssetBone>()
            .init_asset_loader::<ZmoAssetLoader>()
            .init_asset_loader::<ZmoTextureAssetLoader>();

        app.add_event::<AnimationFrameEvent>();

        app.register_type::<AnimationState>()
            .register_type::<CameraAnimation>()
            .register_type::<MeshAnimation>()
            .register_type::<SkeletalAnimation>()
            .register_type::<TransformAnimation>();

        app.configure_set(
            PostUpdate,
            RoseAnimationSystem.before(TransformSystem::TransformPropagate),
        )
        .add_systems(
            PostUpdate,
            (
                camera_animation_system,
                mesh_animation_system,
                skeletal_animation_system,
                transform_animation_system,
            )
                .in_set(RoseAnimationSystem),
        );
    }
}
