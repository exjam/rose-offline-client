use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    math::{Vec3, Vec4},
    prelude::{
        App, Assets, Commands, Entity, FromWorld, HandleUntyped, Plugin, Res, ResMut, Shader, World,
    },
    reflect::TypeUuid,
    render::{
        render_phase::{EntityRenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::{
            encase, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer,
            BufferBindingType, BufferDescriptor, BufferUsages, ShaderSize, ShaderStages,
            ShaderType,
        },
        renderer::{RenderDevice, RenderQueue},
        RenderApp, RenderStage,
    },
};

pub const ZONE_LIGHTING_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 0x444949d32b35d5d9);

#[derive(Default)]
pub struct ZoneLightingPlugin;

impl Plugin for ZoneLightingPlugin {
    fn build(&self, app: &mut App) {
        let mut shader_assets = app.world.resource_mut::<Assets<Shader>>();
        shader_assets.set_untracked(
            ZONE_LIGHTING_SHADER_HANDLE,
            Shader::from_wgsl(include_str!("shaders/zone_lighting.wgsl")),
        );

        app.world.insert_resource(ZoneLighting {
            map_ambient_color: Vec3::ONE,
            character_ambient_color: Vec3::ONE,
            character_diffuse_color: Vec3::ONE,
            fog_color: Vec3::new(0.2, 0.2, 0.2),
            color_fog_enabled: true,
            fog_density: 0.0018,
            fog_min_density: 0.0,
            fog_max_density: 0.75,
            alpha_fog_enabled: true,
            fog_alpha_weight_start: 0.85,
            fog_alpha_weight_end: 0.98,
            height_fog_enabled: false,
            fog_height_offset: 15.0,
            fog_height_falloff: 45.0,
        });

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ZoneLightingUniformMeta>()
                .add_system_to_stage(RenderStage::Extract, extract_uniform_data)
                .add_system_to_stage(RenderStage::Prepare, prepare_uniform_data);
        }
    }
}

pub struct ZoneLighting {
    pub map_ambient_color: Vec3,
    pub character_ambient_color: Vec3,
    pub character_diffuse_color: Vec3,

    pub color_fog_enabled: bool,
    pub fog_color: Vec3,
    pub fog_density: f32,
    pub fog_min_density: f32,
    pub fog_max_density: f32,

    pub alpha_fog_enabled: bool,
    pub fog_alpha_weight_start: f32,
    pub fog_alpha_weight_end: f32,

    pub height_fog_enabled: bool,
    pub fog_height_offset: f32,
    pub fog_height_falloff: f32,
}

#[derive(Clone, ShaderType)]
pub struct ZoneLightingUniformData {
    pub map_ambient_color: Vec4,
    pub character_ambient_color: Vec4,
    pub character_diffuse_color: Vec4,

    pub fog_color: Vec4,
    pub fog_density: f32,
    pub fog_min_density: f32,
    pub fog_max_density: f32,

    pub fog_height_offset: f32,
    pub fog_height_falloff: f32,

    // TODO: Calculate camera far plane based on alpha fog:
    // far = sqrt(log2(1.0 - fog_alpha_weight_end) / (-fog_density * fog_density * 1.442695))
    pub fog_alpha_weight_start: f32,
    pub fog_alpha_weight_end: f32,
}

pub struct ZoneLightingUniformMeta {
    buffer: Buffer,
    bind_group: BindGroup,
    pub bind_group_layout: BindGroupLayout,
}

impl FromWorld for ZoneLightingUniformMeta {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let buffer = render_device.create_buffer(&BufferDescriptor {
            size: ZoneLightingUniformData::min_size().get(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
            label: Some("zone_lighting_uniform_buffer"),
        });

        let bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(ZoneLightingUniformData::min_size()),
                    },
                    count: None,
                }],
                label: Some("zone_lighting_uniform_layout"),
            });

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        ZoneLightingUniformMeta {
            buffer,
            bind_group,
            bind_group_layout,
        }
    }
}

fn extract_uniform_data(mut commands: Commands, zone_lighting: Res<ZoneLighting>) {
    commands.insert_resource(ZoneLightingUniformData {
        map_ambient_color: zone_lighting.map_ambient_color.extend(1.0),
        character_ambient_color: zone_lighting.character_ambient_color.extend(1.0),
        character_diffuse_color: zone_lighting.character_diffuse_color.extend(1.0),
        fog_color: zone_lighting.fog_color.extend(1.0),
        fog_density: if zone_lighting.color_fog_enabled {
            zone_lighting.fog_density
        } else {
            0.0
        },
        fog_min_density: if zone_lighting.color_fog_enabled {
            zone_lighting.fog_min_density
        } else {
            0.0
        },
        fog_max_density: if zone_lighting.color_fog_enabled {
            zone_lighting.fog_max_density
        } else {
            0.0
        },
        fog_height_offset: if zone_lighting.height_fog_enabled {
            zone_lighting.fog_height_offset
        } else {
            99999999999.0
        },
        fog_height_falloff: if zone_lighting.height_fog_enabled {
            zone_lighting.fog_height_falloff
        } else {
            99999999999.0
        },
        fog_alpha_weight_start: if zone_lighting.alpha_fog_enabled {
            zone_lighting.fog_alpha_weight_start
        } else {
            99999999999.0
        },
        fog_alpha_weight_end: if zone_lighting.alpha_fog_enabled {
            zone_lighting.fog_alpha_weight_end
        } else {
            99999999999.0
        },
    });
}

fn prepare_uniform_data(
    uniform_data: Res<ZoneLightingUniformData>,
    uniform_meta: ResMut<ZoneLightingUniformMeta>,
    render_queue: Res<RenderQueue>,
) {
    let byte_buffer = [0u8; ZoneLightingUniformData::SHADER_SIZE.get() as usize];
    let mut buffer = encase::UniformBuffer::new(byte_buffer);
    buffer.write(uniform_data.as_ref()).unwrap();

    render_queue.write_buffer(&uniform_meta.buffer, 0, buffer.as_ref());
}

pub struct SetZoneLightingBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetZoneLightingBindGroup<I> {
    type Param = SRes<ZoneLightingUniformMeta>;

    fn render<'w>(
        _view: Entity,
        _item: Entity,
        meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &meta.into_inner().bind_group, &[]);

        RenderCommandResult::Success
    }
}
