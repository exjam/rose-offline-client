use bevy::{
    asset::Handle,
    math::Vec2,
    pbr::{AlphaMode, Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin},
    prelude::{App, Assets, HandleUntyped, Mesh, Plugin},
    reflect::TypeUuid,
    render::{
        mesh::MeshVertexBufferLayout,
        prelude::Shader,
        render_asset::RenderAssets,
        render_resource::{
            encase::ShaderType, AsBindGroup, AsBindGroupShaderType, CompareFunction,
            RenderPipelineDescriptor, ShaderRef, SpecializedMeshPipelineError,
        },
        texture::Image,
    },
};
use bevy_inspector_egui::Inspectable;

use crate::render::MESH_ATTRIBUTE_UV_1;

pub const OBJECT_MATERIAL_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 0xb7ebbc00ea16d3c7);

#[derive(Default)]
pub struct ObjectMaterialPlugin;

impl Plugin for ObjectMaterialPlugin {
    fn build(&self, app: &mut App) {
        let mut shader_assets = app.world.resource_mut::<Assets<Shader>>();
        shader_assets.set_untracked(
            OBJECT_MATERIAL_SHADER_HANDLE,
            Shader::from_wgsl(include_str!("shaders/object_material.wgsl")),
        );

        app.add_plugin(MaterialPlugin::<ObjectMaterial>::default());
    }
}

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

#[derive(AsBindGroup, Debug, Clone, TypeUuid, Inspectable)]
#[uuid = "62a496fa-33e8-41a8-9a44-237d70214227"]
#[bind_group_data(ObjectMaterialKey)]
#[uniform(0, ObjectMaterialUniformData)]
pub struct ObjectMaterial {
    #[texture(1)]
    #[sampler(2)]
    pub base_texture: Option<Handle<Image>>,

    pub alpha_value: Option<f32>,
    pub alpha_enabled: bool,
    pub alpha_test: Option<f32>,
    pub two_sided: bool,
    pub z_test_enabled: bool,
    pub z_write_enabled: bool,
    pub specular_enabled: bool,
    pub skinned: bool,

    // lightmap texture, uv offset, uv scale
    #[texture(3)]
    #[sampler(4)]
    pub lightmap_texture: Option<Handle<Image>>,
    pub lightmap_uv_offset: Vec2,
    pub lightmap_uv_scale: f32,
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
            specular_enabled: false,
            skinned: false,
            lightmap_texture: None,
            lightmap_uv_offset: Vec2::new(0.0, 0.0),
            lightmap_uv_scale: 1.0,
        }
    }
}

impl AsBindGroupShaderType<ObjectMaterialUniformData> for ObjectMaterial {
    fn as_bind_group_shader_type(
        &self,
        _images: &RenderAssets<Image>,
    ) -> ObjectMaterialUniformData {
        let mut flags = ObjectMaterialFlags::NONE;
        let mut alpha_cutoff = 0.5;
        let mut alpha_value = 1.0;

        if self.specular_enabled {
            flags |= ObjectMaterialFlags::ALPHA_MODE_OPAQUE | ObjectMaterialFlags::SPECULAR;
            alpha_cutoff = 1.0;
        } else {
            if self.alpha_enabled {
                flags |= ObjectMaterialFlags::ALPHA_MODE_BLEND;

                if let Some(alpha_ref) = self.alpha_test {
                    flags |= ObjectMaterialFlags::ALPHA_MODE_MASK;
                    alpha_cutoff = alpha_ref;
                }
            } else {
                flags |= ObjectMaterialFlags::ALPHA_MODE_OPAQUE;
            }

            if let Some(material_alpha_value) = self.alpha_value {
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
            lightmap_uv_offset: self.lightmap_uv_offset,
            lightmap_uv_scale: self.lightmap_uv_scale,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ObjectMaterialKey {
    has_lightmap: bool,
    two_sided: bool,
    z_test_enabled: bool,
    z_write_enabled: bool,
    skinned: bool,
}

impl From<&ObjectMaterial> for ObjectMaterialKey {
    fn from(material: &ObjectMaterial) -> Self {
        ObjectMaterialKey {
            has_lightmap: material.lightmap_texture.is_some(),
            two_sided: material.two_sided,
            z_test_enabled: material.z_test_enabled,
            z_write_enabled: material.z_write_enabled,
            skinned: material.skinned,
        }
    }
}

impl Material for ObjectMaterial {
    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayout,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let mut vertex_attributes = vec![
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
        ];

        if key.bind_group_data.has_lightmap {
            descriptor
                .vertex
                .shader_defs
                .push(String::from("HAS_OBJECT_LIGHTMAP"));
            descriptor
                .fragment
                .as_mut()
                .unwrap()
                .shader_defs
                .push(String::from("HAS_OBJECT_LIGHTMAP"));

            vertex_attributes.push(MESH_ATTRIBUTE_UV_1.at_shader_location(2));
        }

        if layout.contains(Mesh::ATTRIBUTE_JOINT_INDEX)
            && layout.contains(Mesh::ATTRIBUTE_JOINT_WEIGHT)
        {
            descriptor.vertex.shader_defs.push(String::from("SKINNED"));
            descriptor
                .fragment
                .as_mut()
                .unwrap()
                .shader_defs
                .push(String::from("SKINNED"));

            vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_INDEX.at_shader_location(3));
            vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_WEIGHT.at_shader_location(4));
        } else if key.bind_group_data.skinned {
            panic!("strange");
        }

        descriptor.vertex.buffers = vec![layout.get_layout(&vertex_attributes)?];

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
        ShaderRef::Handle(OBJECT_MATERIAL_SHADER_HANDLE.typed())
    }

    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(OBJECT_MATERIAL_SHADER_HANDLE.typed())
    }

    #[inline]
    fn alpha_mode(&self) -> AlphaMode {
        let mut alpha_mode = AlphaMode::Opaque;

        if !self.z_write_enabled {
            // When no depth write we need back to front rendering which only happens
            // in the the transparent pass, by returning AlphaMode::Blend here we tell
            // pbr::MeshPipeline to use the transparent pass
            alpha_mode = AlphaMode::Blend;
        } else if self.specular_enabled {
            alpha_mode = AlphaMode::Opaque;
        } else {
            if self.alpha_enabled {
                alpha_mode = AlphaMode::Blend;

                if let Some(alpha_ref) = self.alpha_test {
                    alpha_mode = AlphaMode::Mask(alpha_ref);
                }
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
}
