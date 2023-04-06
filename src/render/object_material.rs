use bevy::{
    asset::{load_internal_asset, Handle},
    ecs::{
        query::{QueryItem, ROQueryItem},
        system::{
            lifetimeless::{Read, SRes},
            SystemParamItem,
        },
    },
    math::Vec2,
    pbr::{
        AlphaMode, DrawPrepass, MeshPipelineKey, SetMaterialBindGroup, SetMeshBindGroup,
        SetMeshViewBindGroup,
    },
    prelude::{
        AddAsset, App, Component, FromWorld, HandleUntyped, Material, MaterialPlugin, Mesh, Plugin,
        Vec3, With, World,
    },
    reflect::{FromReflect, Reflect, TypeUuid},
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        mesh::{GpuBufferInfo, MeshVertexBufferLayout},
        prelude::Shader,
        render_asset::RenderAssets,
        render_phase::{
            PhaseItem, RenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            encase::ShaderType, AsBindGroup, BindGroupLayout, BlendComponent, BlendFactor,
            BlendOperation, BlendState, CompareFunction, RenderPipelineDescriptor, ShaderDefVal,
            SpecializedMeshPipelineError,
        },
        texture::Image,
    },
};

use rose_file_readers::{ZscMaterialBlend, ZscMaterialGlow};

use crate::render::{
    zone_lighting::{SetZoneLightingBindGroup, ZoneLightingUniformMeta},
    MESH_ATTRIBUTE_UV_1,
};

pub const OBJECT_MATERIAL_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 0xb7ebbc00ea16d3c7);

#[derive(Default)]
pub struct ObjectMaterialPlugin {
    pub prepass_enabled: bool,
}

impl Plugin for ObjectMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            OBJECT_MATERIAL_SHADER_HANDLE,
            "shaders/object_material.wgsl",
            Shader::from_wgsl
        );

        app.add_plugin(ExtractComponentPlugin::<ObjectMaterialClipFace>::extract_visible());

        app.register_type::<ObjectMaterial>();

        app.add_plugin(MaterialPlugin::<
            ObjectMaterial,
            DrawObjectMaterial,
            DrawPrepass<ObjectMaterial>,
        > {
            prepass_enabled: self.prepass_enabled,
            ..Default::default()
        });

        app.register_asset_reflect::<ObjectMaterial>();
        app.register_type::<ObjectMaterialClipFace>();
    }
}

#[derive(Copy, Clone, Component, Reflect)]
pub enum ObjectMaterialClipFace {
    First(u32),
    Last(u32),
}

impl ExtractComponent for ObjectMaterialClipFace {
    type Query = &'static Self;
    type Filter = With<Handle<ObjectMaterial>>;
    type Out = ObjectMaterialClipFace;

    fn extract_component(item: QueryItem<Self::Query>) -> Option<Self> {
        Some(*item)
    }
}

pub struct DrawObjectMesh;
impl<P: PhaseItem> RenderCommand<P> for DrawObjectMesh {
    type Param = SRes<RenderAssets<Mesh>>;
    type ItemWorldQuery = (Read<Handle<Mesh>>, Option<Read<ObjectMaterialClipFace>>);
    type ViewWorldQuery = ();

    #[inline]
    fn render<'w>(
        _: &P,
        _: ROQueryItem<'_, Self::ViewWorldQuery>,
        (mesh_handle, clip_face): ROQueryItem<'_, Self::ItemWorldQuery>,
        meshes: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let (start_index_offset, end_index_offset) = if let Some(clip_face) = clip_face {
            match clip_face {
                ObjectMaterialClipFace::First(num_faces) => (num_faces * 3, 0),
                ObjectMaterialClipFace::Last(num_faces) => (0, num_faces * 3),
            }
        } else {
            (0, 0)
        };

        if let Some(gpu_mesh) = meshes.into_inner().get(mesh_handle) {
            pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
            match &gpu_mesh.buffer_info {
                GpuBufferInfo::Indexed {
                    buffer,
                    index_format,
                    count,
                } => {
                    let start_index = start_index_offset;
                    let end_index = *count - end_index_offset;
                    pass.set_index_buffer(buffer.slice(..), 0, *index_format);
                    pass.draw_indexed(start_index..end_index, 0, 0..1);
                }
                GpuBufferInfo::NonIndexed { vertex_count } => {
                    let start_vertex = start_index_offset;
                    let end_vertex = *vertex_count - end_index_offset;
                    pass.draw(start_vertex..end_vertex, 0..1);
                }
            }
            RenderCommandResult::Success
        } else {
            RenderCommandResult::Failure
        }
    }
}

type DrawObjectMaterial = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMaterialBindGroup<ObjectMaterial, 1>,
    SetMeshBindGroup<2>,
    SetZoneLightingBindGroup<3>,
    DrawObjectMesh,
);

// NOTE: These must match the bit flags in shaders/object_material.wgsl!
bitflags::bitflags! {
    #[repr(transparent)]
    pub struct ObjectMaterialFlags: u32 {
        const ALPHA_MODE_OPAQUE          = (1 << 0);
        const ALPHA_MODE_MASK            = (1 << 1);
        const ALPHA_MODE_BLEND           = (1 << 2);
        const HAS_ALPHA_VALUE            = (1 << 3);
        const SPECULAR                   = (1 << 4);
        const NONE                       = 0;
    }
}

#[derive(Clone, ShaderType)]
pub struct ObjectMaterialUniformData {
    pub flags: u32,
    pub alpha_cutoff: f32,
    pub alpha_value: f32,
    pub lightmap_uv_offset: Vec2,
    pub lightmap_uv_scale: f32,
}

impl From<&ObjectMaterial> for ObjectMaterialUniformData {
    fn from(material: &ObjectMaterial) -> ObjectMaterialUniformData {
        let mut flags = ObjectMaterialFlags::NONE;
        let mut alpha_cutoff = 0.5;
        let mut alpha_value = 1.0;

        if material.specular_texture.is_some() {
            flags |= ObjectMaterialFlags::ALPHA_MODE_OPAQUE | ObjectMaterialFlags::SPECULAR;
            alpha_cutoff = 1.0;
        } else {
            if material.alpha_enabled {
                flags |= ObjectMaterialFlags::ALPHA_MODE_BLEND;

                if let Some(alpha_ref) = material.alpha_test {
                    flags |= ObjectMaterialFlags::ALPHA_MODE_MASK;
                    alpha_cutoff = alpha_ref;
                }
            } else {
                flags |= ObjectMaterialFlags::ALPHA_MODE_OPAQUE;
            }

            if let Some(material_alpha_value) = material.alpha_value {
                if material_alpha_value == 1.0 {
                    flags |= ObjectMaterialFlags::ALPHA_MODE_OPAQUE;
                } else {
                    flags |= ObjectMaterialFlags::HAS_ALPHA_VALUE;
                    alpha_value = material_alpha_value;
                }
            }
        }

        ObjectMaterialUniformData {
            flags: flags.bits(),
            alpha_cutoff,
            alpha_value,
            lightmap_uv_offset: material.lightmap_uv_offset,
            lightmap_uv_scale: material.lightmap_uv_scale,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, Reflect, FromReflect)]
pub enum ObjectMaterialBlend {
    #[default]
    Normal,
    Lighten,
}

impl From<ZscMaterialBlend> for ObjectMaterialBlend {
    fn from(zsc: ZscMaterialBlend) -> Self {
        match zsc {
            ZscMaterialBlend::Normal => ObjectMaterialBlend::Normal,
            ZscMaterialBlend::Lighten => ObjectMaterialBlend::Lighten,
        }
    }
}

#[derive(Copy, Clone, Debug, Reflect, FromReflect)]
pub enum ObjectMaterialGlow {
    Simple(Vec3),
    Light(Vec3),
    Texture(Vec3),
    TextureLight(Vec3),
    Alpha(Vec3),
}

impl From<ZscMaterialGlow> for ObjectMaterialGlow {
    fn from(zsc: ZscMaterialGlow) -> Self {
        match zsc {
            ZscMaterialGlow::Simple(value) => {
                ObjectMaterialGlow::Simple(Vec3::new(value.x, value.y, value.z))
            }
            ZscMaterialGlow::Light(value) => {
                ObjectMaterialGlow::Light(Vec3::new(value.x, value.y, value.z))
            }
            ZscMaterialGlow::Texture(value) => {
                ObjectMaterialGlow::Texture(Vec3::new(value.x, value.y, value.z))
            }
            ZscMaterialGlow::TextureLight(value) => {
                ObjectMaterialGlow::TextureLight(Vec3::new(value.x, value.y, value.z))
            }
            ZscMaterialGlow::Alpha(value) => {
                ObjectMaterialGlow::Alpha(Vec3::new(value.x, value.y, value.z))
            }
        }
    }
}

#[derive(Debug, Clone, TypeUuid, Reflect, FromReflect, AsBindGroup)]
#[uniform(0, ObjectMaterialUniformData)]
#[bind_group_data(ObjectMaterialKey)]
#[uuid = "62a496fa-33e8-41a8-9a44-237d70214227"]
pub struct ObjectMaterial {
    #[texture(1)]
    #[sampler(2)]
    pub base_texture: Option<Handle<Image>>,

    #[texture(3)]
    #[sampler(4)]
    pub lightmap_texture: Option<Handle<Image>>,
    pub lightmap_uv_offset: Vec2,
    pub lightmap_uv_scale: f32,

    #[texture(5)]
    #[sampler(6)]
    pub specular_texture: Option<Handle<Image>>,

    pub alpha_value: Option<f32>,
    pub alpha_enabled: bool,
    pub alpha_test: Option<f32>,
    pub two_sided: bool,
    pub z_test_enabled: bool,
    pub z_write_enabled: bool,
    pub skinned: bool,
    pub blend: ObjectMaterialBlend,
    pub glow: Option<ObjectMaterialGlow>,
}

#[derive(Clone)]
pub struct ObjectMaterialPipelineData {
    pub zone_lighting_layout: BindGroupLayout,
}

impl FromWorld for ObjectMaterialPipelineData {
    fn from_world(world: &mut World) -> Self {
        ObjectMaterialPipelineData {
            zone_lighting_layout: world
                .resource::<ZoneLightingUniformMeta>()
                .bind_group_layout
                .clone(),
        }
    }
}

impl Material for ObjectMaterial {
    type PipelineData = ObjectMaterialPipelineData;

    fn vertex_shader() -> bevy::render::render_resource::ShaderRef {
        OBJECT_MATERIAL_SHADER_HANDLE.typed().into()
    }

    fn fragment_shader() -> bevy::render::render_resource::ShaderRef {
        OBJECT_MATERIAL_SHADER_HANDLE.typed().into()
    }

    fn alpha_mode(&self) -> bevy::prelude::AlphaMode {
        let mut alpha_mode;

        if self.specular_texture.is_some() {
            alpha_mode = AlphaMode::Opaque;
        } else {
            if self.alpha_enabled {
                alpha_mode = AlphaMode::Blend;

                if let Some(alpha_ref) = self.alpha_test {
                    alpha_mode = AlphaMode::Mask(alpha_ref);
                }
            } else {
                alpha_mode = AlphaMode::Opaque;
            }

            if let Some(material_alpha_value) = self.alpha_value {
                if material_alpha_value == 1.0 {
                    alpha_mode = AlphaMode::Opaque;
                } else {
                    alpha_mode = AlphaMode::Blend;
                }
            }
        }

        alpha_mode
    }

    fn specialize(
        object_material_pipeline_data: &bevy::pbr::MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayout,
        key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor
            .depth_stencil
            .as_mut()
            .unwrap()
            .depth_write_enabled = key.bind_group_data.z_write_enabled;

        if !key.bind_group_data.z_test_enabled {
            descriptor.depth_stencil.as_mut().unwrap().depth_compare = CompareFunction::Always;
        }

        if key.bind_group_data.two_sided {
            descriptor.primitive.cull_mode = None;
        }

        if key.mesh_key.contains(MeshPipelineKey::DEPTH_PREPASS)
            || key.mesh_key.contains(MeshPipelineKey::NORMAL_PREPASS)
        {
            return Ok(());
        }

        descriptor.layout.insert(
            3,
            object_material_pipeline_data
                .data
                .zone_lighting_layout
                .clone(),
        );

        let mut vertex_attributes = vec![
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
        ];

        if key.bind_group_data.has_lightmap {
            descriptor
                .vertex
                .shader_defs
                .push(ShaderDefVal::Bool("HAS_OBJECT_LIGHTMAP".into(), true));

            if let Some(fragment) = descriptor.fragment.as_mut() {
                fragment
                    .shader_defs
                    .push(ShaderDefVal::Bool("HAS_OBJECT_LIGHTMAP".into(), true));
            }

            vertex_attributes.push(MESH_ATTRIBUTE_UV_1.at_shader_location(3));
        } else if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment
                .shader_defs
                .push(ShaderDefVal::Bool("ZONE_LIGHTING_CHARACTER".into(), true));
        }

        if layout.contains(Mesh::ATTRIBUTE_JOINT_INDEX)
            && layout.contains(Mesh::ATTRIBUTE_JOINT_WEIGHT)
        {
            descriptor
                .vertex
                .shader_defs
                .push(ShaderDefVal::Bool("SKINNED".into(), true));

            if let Some(fragment) = descriptor.fragment.as_mut() {
                fragment
                    .shader_defs
                    .push(ShaderDefVal::Bool("SKINNED".into(), true));
            }

            vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_INDEX.at_shader_location(4));
            vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_WEIGHT.at_shader_location(5));
        }

        descriptor.vertex.buffers = vec![layout.get_layout(&vertex_attributes)?];

        if let Some(fragment) = descriptor.fragment.as_mut() {
            for color_target_state in fragment.targets.iter_mut().filter_map(|x| x.as_mut()) {
                color_target_state.blend = Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::SrcAlpha,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::SrcAlpha,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                        operation: BlendOperation::Add,
                    },
                });
            }
        }

        Ok(())
    }
}

impl Default for ObjectMaterial {
    fn default() -> Self {
        Self {
            base_texture: None,
            alpha_value: None,
            alpha_enabled: false,
            alpha_test: None,
            two_sided: false,
            z_test_enabled: true,
            z_write_enabled: true,
            specular_texture: None,
            skinned: false,
            blend: ObjectMaterialBlend::Normal,
            glow: None,
            lightmap_texture: None,
            lightmap_uv_offset: Vec2::new(0.0, 0.0),
            lightmap_uv_scale: 1.0,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ObjectMaterialKey {
    has_lightmap: bool,
    two_sided: bool,
    z_test_enabled: bool,
    z_write_enabled: bool,
}

impl From<&ObjectMaterial> for ObjectMaterialKey {
    fn from(material: &ObjectMaterial) -> Self {
        ObjectMaterialKey {
            has_lightmap: material.lightmap_texture.is_some(),
            two_sided: material.two_sided,
            z_test_enabled: material.z_test_enabled,
            z_write_enabled: material.z_write_enabled,
        }
    }
}
