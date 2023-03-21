use bevy::{
    asset::Handle,
    pbr::{AlphaMode, Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin},
    prelude::{App, Assets, HandleUntyped, Mesh, Plugin},
    reflect::TypeUuid,
    render::{
        mesh::MeshVertexBufferLayout,
        prelude::Shader,
        render_asset::RenderAssets,
        render_resource::{
            encase::ShaderType, AsBindGroup, AsBindGroupShaderType, BlendComponent, BlendFactor,
            BlendOperation, BlendState, CompareFunction, RenderPipelineDescriptor, ShaderRef,
            SpecializedMeshPipelineError,
        },
        texture::Image,
    },
};

pub const EFFECT_MESH_MATERIAL_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 0x90d5233c3001d33e);

#[derive(Default)]
pub struct EffectMeshMaterialPlugin;

impl Plugin for EffectMeshMaterialPlugin {
    fn build(&self, app: &mut App) {
        let mut shader_assets = app.world.resource_mut::<Assets<Shader>>();
        shader_assets.set_untracked(
            EFFECT_MESH_MATERIAL_SHADER_HANDLE,
            Shader::from_wgsl(include_str!("shaders/effect_mesh_material.wgsl")),
        );

        app.add_plugin(MaterialPlugin::<EffectMeshMaterial>::default());
    }
}

// NOTE: These must match the bit flags in shaders/effect_mesh_material.wgsl!
bitflags::bitflags! {
    #[repr(transparent)]
    pub struct EffectMeshMaterialFlags: u32 {
        const ALPHA_MODE_OPAQUE          = (1 << 0);
        const ALPHA_MODE_MASK            = (1 << 1);
        const NONE                       = 0;
    }
}

#[derive(Clone, ShaderType)]
pub struct EffectMeshMaterialUniformData {
    pub flags: u32,
    pub alpha_cutoff: f32,
}

#[derive(AsBindGroup, Debug, Clone, TypeUuid)]
#[uuid = "9ac3266d-1aa6-4f67-ade4-e3765fd0b1a1"]
#[bind_group_data(EffectMeshMaterialKey)]
#[uniform(0, EffectMeshMaterialUniformData)]
pub struct EffectMeshMaterial {
    #[texture(1)]
    #[sampler(2)]
    pub base_texture: Option<Handle<Image>>,

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

impl Material for EffectMeshMaterial {
    type PipelineData = ();

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayout,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_attributes = [
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
        ];

        descriptor.vertex.buffers = vec![layout.get_layout(&vertex_attributes)?];
        descriptor.fragment.as_mut().unwrap().targets[0]
            .as_mut()
            .unwrap()
            .blend = Some(BlendState {
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
