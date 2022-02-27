use bevy::{
    asset::{Asset, AssetServer, Handle},
    ecs::{
        entity::Entity,
        system::{
            lifetimeless::{Read, SQuery, SRes},
            SystemParamItem,
        },
    },
    prelude::Component,
    render::{
        mesh::MeshVertexBufferLayout,
        render_asset::{RenderAsset, RenderAssets},
        render_phase::{
            EntityRenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            BindGroup, BindGroupLayout, RenderPipelineDescriptor, Shader,
            SpecializedMeshPipelineError,
        },
        renderer::RenderDevice,
    },
};

use std::hash::Hash;
use std::marker::PhantomData;

use crate::mesh_pipeline::{DrawMesh, MeshPipelineKey, SetMeshBindGroup, SetMeshViewBindGroup};

/// Alpha mode
#[allow(dead_code)]
#[derive(Component, Debug, Copy, Clone, PartialEq)]
pub enum AlphaMode {
    Opaque,
    /// An alpha cutoff must be supplied where alpha values >= the cutoff
    /// will be fully opaque and < will be fully transparent
    Mask(f32),
    Blend,
}

impl Eq for AlphaMode {}

impl Default for AlphaMode {
    fn default() -> Self {
        AlphaMode::Opaque
    }
}

/// Materials are used alongside [`MaterialPlugin`] and [`MaterialMeshBundle`](crate::MaterialMeshBundle)
/// to spawn entities that are rendered with a specific [`Material`] type. They serve as an easy to use high level
/// way to render [`Mesh`] entities with custom shader logic. For materials that can specialize their [`RenderPipelineDescriptor`]
/// based on specific material values, see [`SpecializedMaterial`]. [`Material`] automatically implements [`SpecializedMaterial`]
/// and can be used anywhere that type is used (such as [`MaterialPlugin`]).
pub trait Material: Asset + RenderAsset {
    /// Returns this material's [`BindGroup`]. This should match the layout returned by [`Material::bind_group_layout`].
    fn bind_group(material: &<Self as RenderAsset>::PreparedAsset) -> &BindGroup;

    /// Returns this material's [`BindGroupLayout`]. This should match the [`BindGroup`] returned by [`Material::bind_group`].
    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout;

    /// Returns this material's vertex shader. If [`None`] is returned, the default mesh vertex shader will be used.
    /// Defaults to [`None`].
    #[allow(unused_variables)]
    fn vertex_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        None
    }

    /// Returns this material's fragment shader. If [`None`] is returned, the default mesh fragment shader will be used.
    /// Defaults to [`None`].
    #[allow(unused_variables)]
    fn fragment_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        None
    }

    /// Returns this material's [`AlphaMode`]. Defaults to [`AlphaMode::Opaque`].
    #[allow(unused_variables)]
    fn alpha_mode(material: &<Self as RenderAsset>::PreparedAsset) -> AlphaMode {
        AlphaMode::Opaque
    }

    /// The dynamic uniform indices to set for the given `material`'s [`BindGroup`].
    /// Defaults to an empty array / no dynamic uniform indices.
    #[allow(unused_variables)]
    #[inline]
    fn dynamic_uniform_indices(material: &<Self as RenderAsset>::PreparedAsset) -> &[u32] {
        &[]
    }

    /// Customizes the default [`RenderPipelineDescriptor`].
    #[allow(unused_variables)]
    #[inline]
    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayout,
    ) -> Result<(), SpecializedMeshPipelineError> {
        Ok(())
    }
}

impl<M: Material> SpecializedMaterial for M {
    type Key = ();

    #[inline]
    fn key(_material: &<Self as RenderAsset>::PreparedAsset) -> Self::Key {}

    #[inline]
    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        _key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<(), SpecializedMeshPipelineError> {
        <M as Material>::specialize(descriptor, layout)
    }

    #[inline]
    fn bind_group(material: &<Self as RenderAsset>::PreparedAsset) -> &BindGroup {
        <M as Material>::bind_group(material)
    }

    #[inline]
    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
        <M as Material>::bind_group_layout(render_device)
    }

    #[inline]
    fn alpha_mode(material: &<Self as RenderAsset>::PreparedAsset) -> AlphaMode {
        <M as Material>::alpha_mode(material)
    }

    #[inline]
    fn vertex_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        <M as Material>::vertex_shader(asset_server)
    }

    #[inline]
    fn fragment_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        <M as Material>::fragment_shader(asset_server)
    }

    #[allow(unused_variables)]
    #[inline]
    fn dynamic_uniform_indices(material: &<Self as RenderAsset>::PreparedAsset) -> &[u32] {
        <M as Material>::dynamic_uniform_indices(material)
    }
}

/// Materials are used alongside [`MaterialPlugin`] and [`MaterialMeshBundle`](crate::MaterialMeshBundle)
/// to spawn entities that are rendered with a specific [`SpecializedMaterial`] type. They serve as an easy to use high level
/// way to render [`Mesh`] entities with custom shader logic. [`SpecializedMaterials`](SpecializedMaterial) use their [`SpecializedMaterial::Key`]
/// to customize their [`RenderPipelineDescriptor`] based on specific material values. The slightly simpler [`Material`] trait
/// should be used for materials that do not need specialization. [`Material`] types automatically implement [`SpecializedMaterial`].
pub trait SpecializedMaterial: Asset + RenderAsset {
    /// The key used to specialize this material's [`RenderPipelineDescriptor`].
    type Key: PartialEq + Eq + Hash + Clone + Send + Sync;

    /// Extract the [`SpecializedMaterial::Key`] for the "prepared" version of this material. This key will be
    /// passed in to the [`SpecializedMaterial::specialize`] function when compiling the [`RenderPipeline`](bevy_render::render_resource::RenderPipeline)
    /// for a given entity's material.
    fn key(material: &<Self as RenderAsset>::PreparedAsset) -> Self::Key;

    /// Specializes the given `descriptor` according to the given `key`.
    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<(), SpecializedMeshPipelineError>;

    /// Returns this material's [`BindGroup`]. This should match the layout returned by [`SpecializedMaterial::bind_group_layout`].
    fn bind_group(material: &<Self as RenderAsset>::PreparedAsset) -> &BindGroup;

    /// Returns this material's [`BindGroupLayout`]. This should match the [`BindGroup`] returned by [`SpecializedMaterial::bind_group`].
    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout;

    /// Returns this material's vertex shader. If [`None`] is returned, the default mesh vertex shader will be used.
    /// Defaults to [`None`].
    #[allow(unused_variables)]
    fn vertex_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        None
    }

    /// Returns this material's fragment shader. If [`None`] is returned, the default mesh fragment shader will be used.
    /// Defaults to [`None`].
    #[allow(unused_variables)]
    fn fragment_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        None
    }

    /// Returns this material's [`AlphaMode`]. Defaults to [`AlphaMode::Opaque`].
    #[allow(unused_variables)]
    fn alpha_mode(material: &<Self as RenderAsset>::PreparedAsset) -> AlphaMode {
        AlphaMode::Opaque
    }

    /// The dynamic uniform indices to set for the given `material`'s [`BindGroup`].
    /// Defaults to an empty array / no dynamic uniform indices.
    #[allow(unused_variables)]
    #[inline]
    fn dynamic_uniform_indices(material: &<Self as RenderAsset>::PreparedAsset) -> &[u32] {
        &[]
    }
}

#[derive(Eq, PartialEq, Clone, Hash)]
pub struct MaterialPipelineKey<T> {
    pub mesh_key: MeshPipelineKey,
    pub material_key: T,
}

pub struct SetMaterialBindGroup<M: SpecializedMaterial, const I: usize>(PhantomData<M>);
impl<M: SpecializedMaterial, const I: usize> EntityRenderCommand for SetMaterialBindGroup<M, I> {
    type Param = (SRes<RenderAssets<M>>, SQuery<Read<Handle<M>>>);
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (materials, query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let material_handle = query.get(item).unwrap();
        let material = materials.into_inner().get(material_handle).unwrap();
        pass.set_bind_group(
            I,
            M::bind_group(material),
            M::dynamic_uniform_indices(material),
        );
        RenderCommandResult::Success
    }
}

pub type DrawMaterial<M> = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMaterialBindGroup<M, 1>,
    SetMeshBindGroup<2>,
    DrawMesh,
);
