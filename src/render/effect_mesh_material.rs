use bevy::{
    asset::{load_internal_asset, Handle},
    ecs::{
        query::{QueryItem, ROQueryItem},
        system::{
            lifetimeless::{Read, SRes},
            SystemParamItem,
        },
    },
    pbr::{
        AlphaMode, DrawPrepass, Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin,
        MeshPipelineKey, SetMaterialBindGroup, SetMeshBindGroup, SetMeshViewBindGroup,
    },
    prelude::{App, Component, FromWorld, HandleUntyped, Mesh, Plugin, With, World},
    reflect::{Reflect, TypePath, TypeUuid},
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        mesh::{GpuBufferInfo, MeshVertexBufferLayout},
        prelude::Shader,
        render_asset::RenderAssets,
        render_phase::{
            PhaseItem, RenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            encase::{self, ShaderType},
            AsBindGroup, AsBindGroupShaderType, BindGroupLayout, BlendComponent, BlendFactor,
            BlendOperation, BlendState, CompareFunction, PushConstantRange,
            RenderPipelineDescriptor, ShaderDefVal, ShaderRef, ShaderSize, ShaderStages,
            SpecializedMeshPipelineError,
        },
        texture::Image,
    },
};

use crate::render::zone_lighting::{SetZoneLightingBindGroup, ZoneLightingUniformMeta};

pub const EFFECT_MESH_MATERIAL_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 0x90d5233c3001d33e);

#[derive(Default)]
pub struct EffectMeshMaterialPlugin {
    pub prepass_enabled: bool,
}

impl Plugin for EffectMeshMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            EFFECT_MESH_MATERIAL_SHADER_HANDLE,
            "shaders/effect_mesh_material.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins(
            ExtractComponentPlugin::<EffectMeshAnimationRenderState>::extract_visible(),
        );
        app.register_type::<EffectMeshAnimationRenderState>();

        app.add_plugins(MaterialPlugin::<
            EffectMeshMaterial,
            DrawEffectMeshMaterial,
            DrawPrepass<EffectMeshMaterial>,
        > {
            prepass_enabled: self.prepass_enabled,
            ..Default::default()
        });
        //TODO? .register_asset_reflect::<EffectMeshMaterial>();
    }
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct EffectMeshMaterialFlags: u32 {
        const ALPHA_MODE_OPAQUE         = (1 << 0);
        const ALPHA_MODE_MASK           = (1 << 1);
        const NONE                      = 0;
    }
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct EffectMeshAnimationFlags: u32 {
        const ANIMATE_POSITION          = (1 << 0);
        const ANIMATE_NORMALS           = (1 << 1);
        const ANIMATE_UV                = (1 << 2);
        const ANIMATE_ALPHA             = (1 << 3);
        const NONE                      = 0;
    }
}

#[derive(Copy, Clone, Default, Component, ShaderType, Reflect)]
pub struct EffectMeshAnimationRenderState {
    pub flags: u32,
    pub current_next_frame: u32,
    pub next_weight: f32,
    pub alpha: f32,
}

impl ExtractComponent for EffectMeshAnimationRenderState {
    type Query = &'static Self;
    type Filter = With<Handle<EffectMeshMaterial>>;
    type Out = Self;

    fn extract_component(item: QueryItem<Self::Query>) -> Option<Self> {
        Some(*item)
    }
}

#[derive(Clone, ShaderType)]
pub struct EffectMeshMaterialUniformData {
    pub flags: u32,
    pub alpha_cutoff: f32,
}

#[derive(AsBindGroup, Debug, Clone, TypeUuid, TypePath)]
#[uuid = "9ac3266d-1aa6-4f67-ade4-e3765fd0b1a1"]
#[bind_group_data(EffectMeshMaterialKey)]
#[uniform(0, EffectMeshMaterialUniformData)]
pub struct EffectMeshMaterial {
    #[texture(1)]
    #[sampler(2)]
    pub base_texture: Option<Handle<Image>>,

    #[texture(3)]
    #[sampler(4)]
    pub animation_texture: Option<Handle<Image>>,

    pub alpha_enabled: bool,
    pub alpha_test: bool,
    pub two_sided: bool,
    pub z_test_enabled: bool,
    pub z_write_enabled: bool,
    pub blend_op: BlendOperation,
    pub src_blend_factor: BlendFactor,
    pub dst_blend_factor: BlendFactor,
}

impl AsBindGroupShaderType<EffectMeshMaterialUniformData> for EffectMeshMaterial {
    fn as_bind_group_shader_type(
        &self,
        _images: &RenderAssets<Image>,
    ) -> EffectMeshMaterialUniformData {
        let mut flags = EffectMeshMaterialFlags::NONE;
        if self.alpha_test {
            flags |= EffectMeshMaterialFlags::ALPHA_MODE_MASK;
        } else if !self.alpha_enabled {
            flags |= EffectMeshMaterialFlags::ALPHA_MODE_OPAQUE;
        }

        EffectMeshMaterialUniformData {
            flags: flags.bits(),
            alpha_cutoff: 0.5,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct EffectMeshMaterialKey {
    has_animation_texture: bool,
    alpha_enabled: bool,
    alpha_test: bool,
    two_sided: bool,
    z_test_enabled: bool,
    z_write_enabled: bool,
    blend_op: BlendOperation,
    src_blend_factor: BlendFactor,
    dst_blend_factor: BlendFactor,
}

impl From<&EffectMeshMaterial> for EffectMeshMaterialKey {
    fn from(material: &EffectMeshMaterial) -> Self {
        EffectMeshMaterialKey {
            has_animation_texture: material.animation_texture.is_some(),
            alpha_enabled: material.alpha_enabled,
            alpha_test: material.alpha_test,
            two_sided: material.two_sided,
            z_test_enabled: material.z_test_enabled,
            z_write_enabled: material.z_write_enabled,
            blend_op: material.blend_op,
            src_blend_factor: material.src_blend_factor,
            dst_blend_factor: material.dst_blend_factor,
        }
    }
}

#[derive(Clone)]
pub struct EffectMeshMaterialPipelineData {
    pub zone_lighting_layout: BindGroupLayout,
}

impl FromWorld for EffectMeshMaterialPipelineData {
    fn from_world(world: &mut World) -> Self {
        EffectMeshMaterialPipelineData {
            zone_lighting_layout: world
                .resource::<ZoneLightingUniformMeta>()
                .bind_group_layout
                .clone(),
        }
    }
}

impl Material for EffectMeshMaterial {
    type PipelineData = EffectMeshMaterialPipelineData;

    fn specialize(
        pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayout,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if key.bind_group_data.two_sided {
            descriptor.primitive.cull_mode = None;
        }

        descriptor
            .depth_stencil
            .as_mut()
            .unwrap()
            .depth_write_enabled = key.bind_group_data.z_write_enabled;

        if !key.bind_group_data.z_test_enabled {
            descriptor.depth_stencil.as_mut().unwrap().depth_compare = CompareFunction::Always;
        }

        if key.bind_group_data.has_animation_texture {
            descriptor
                .vertex
                .shader_defs
                .push(ShaderDefVal::Bool("HAS_ANIMATION_TEXTURE".into(), true));

            if let Some(fragment) = descriptor.fragment.as_mut() {
                fragment
                    .shader_defs
                    .push(ShaderDefVal::Bool("HAS_ANIMATION_TEXTURE".into(), true));
            }

            descriptor.push_constant_ranges.push(PushConstantRange {
                stages: ShaderStages::VERTEX_FRAGMENT,
                range: 0..EffectMeshAnimationRenderState::SHADER_SIZE.get() as u32,
            });
        }

        if key.mesh_key.contains(MeshPipelineKey::DEPTH_PREPASS)
            || key.mesh_key.contains(MeshPipelineKey::NORMAL_PREPASS)
        {
            return Ok(());
        }

        if matches!(key.bind_group_data.blend_op, BlendOperation::Add) {
            // Do not apply color fog to additive blended mesh
            if let Some(fragment) = descriptor.fragment.as_mut() {
                fragment.shader_defs.push(ShaderDefVal::Bool(
                    "ZONE_LIGHTING_DISABLE_COLOR_FOG".into(),
                    true,
                ));
            }
        }

        descriptor
            .layout
            .insert(3, pipeline.data.zone_lighting_layout.clone());

        if layout.contains(Mesh::ATTRIBUTE_NORMAL) {
            let vertex_attributes = [
                Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
                Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
                Mesh::ATTRIBUTE_NORMAL.at_shader_location(2),
            ];
            descriptor.vertex.buffers = vec![layout.get_layout(&vertex_attributes)?];
        } else {
            let vertex_attributes = [
                Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
                Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
            ];
            descriptor.vertex.buffers = vec![layout.get_layout(&vertex_attributes)?];
        }

        if let Some(fragment) = descriptor.fragment.as_mut() {
            for color_target_state in fragment.targets.iter_mut().filter_map(|x| x.as_mut()) {
                color_target_state.blend = Some(BlendState {
                    color: BlendComponent {
                        src_factor: key.bind_group_data.src_blend_factor,
                        dst_factor: key.bind_group_data.dst_blend_factor,
                        operation: key.bind_group_data.blend_op,
                    },
                    alpha: BlendComponent {
                        src_factor: key.bind_group_data.src_blend_factor,
                        dst_factor: key.bind_group_data.dst_blend_factor,
                        operation: key.bind_group_data.blend_op,
                    },
                });
            }
        }

        Ok(())
    }

    fn vertex_shader() -> ShaderRef {
        ShaderRef::Handle(EFFECT_MESH_MATERIAL_SHADER_HANDLE.typed())
    }

    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(EFFECT_MESH_MATERIAL_SHADER_HANDLE.typed())
    }

    #[inline]
    fn alpha_mode(&self) -> AlphaMode {
        if self.alpha_enabled || !self.z_write_enabled {
            // When no depth write we need back to front rendering which only happens
            // in the the transparent pass, by returning AlphaMode::Blend here we tell
            // pbr::MeshPipeline to use the transparent pass
            AlphaMode::Blend
        } else if self.alpha_test {
            AlphaMode::Mask(0.5)
        } else {
            AlphaMode::Opaque
        }
    }
}

pub struct DrawEffectMesh;
impl<P: PhaseItem> RenderCommand<P> for DrawEffectMesh {
    type Param = SRes<RenderAssets<Mesh>>;
    type ItemWorldQuery = (
        Read<Handle<Mesh>>,
        Option<Read<EffectMeshAnimationRenderState>>,
    );
    type ViewWorldQuery = ();

    #[inline]
    fn render<'w>(
        _: &P,
        _: ROQueryItem<'_, Self::ViewWorldQuery>,
        (mesh_handle, animation_state): ROQueryItem<'_, Self::ItemWorldQuery>,
        meshes: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        if let Some(animation_state) = animation_state {
            let byte_buffer = [0u8; EffectMeshAnimationRenderState::SHADER_SIZE.get() as usize];
            let mut buffer = encase::StorageBuffer::new(byte_buffer);
            buffer.write(animation_state).unwrap();
            pass.set_push_constants(ShaderStages::VERTEX_FRAGMENT, 0, buffer.as_ref());
        }

        if let Some(gpu_mesh) = meshes.into_inner().get(mesh_handle) {
            pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));

            match &gpu_mesh.buffer_info {
                GpuBufferInfo::Indexed {
                    buffer,
                    index_format,
                    count,
                } => {
                    pass.set_index_buffer(buffer.slice(..), 0, *index_format);
                    pass.draw_indexed(0..*count, 0, 0..1);
                }
                GpuBufferInfo::NonIndexed => {
                    pass.draw(0..gpu_mesh.vertex_count, 0..1);
                }
            }
            RenderCommandResult::Success
        } else {
            RenderCommandResult::Failure
        }
    }
}

type DrawEffectMeshMaterial = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMaterialBindGroup<EffectMeshMaterial, 1>,
    SetMeshBindGroup<2>,
    SetZoneLightingBindGroup<3>,
    DrawEffectMesh,
);
