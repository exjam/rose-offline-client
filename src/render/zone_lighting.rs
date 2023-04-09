use bevy::{
    asset::load_internal_asset,
    ecs::{
        query::ROQueryItem,
        system::{lifetimeless::SRes, SystemParamItem},
    },
    math::{Vec3, Vec4},
    pbr::CascadeShadowConfig,
    prelude::{
        AmbientLight, App, Color, Commands, DirectionalLight, DirectionalLightBundle, EulerRot,
        FromWorld, HandleUntyped, IntoSystemAppConfig, IntoSystemConfig, Plugin, Quat,
        ReflectResource, Res, ResMut, Resource, Shader, Transform, World,
    },
    reflect::{FromReflect, Reflect, TypeUuid},
    render::{
        render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::{
            encase, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer,
            BufferBindingType, BufferDescriptor, BufferUsages, ShaderSize, ShaderStages,
            ShaderType,
        },
        renderer::{RenderDevice, RenderQueue},
        Extract, ExtractSchedule, RenderApp, RenderSet,
    },
};

pub const ZONE_LIGHTING_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 0x444949d32b35d5d9);

fn default_light_transform() -> Transform {
    Transform::from_rotation(Quat::from_euler(
        EulerRot::ZYX,
        0.0,
        std::f32::consts::PI * (2.0 / 3.0),
        -std::f32::consts::PI / 4.0,
    ))
}

#[derive(Default)]
pub struct ZoneLightingPlugin;

impl Plugin for ZoneLightingPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            ZONE_LIGHTING_SHADER_HANDLE,
            "shaders/zone_lighting.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<ZoneLighting>()
            .init_resource::<ZoneLighting>();

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ZoneLightingUniformMeta>()
                .add_system(extract_uniform_data.in_schedule(ExtractSchedule))
                .add_system(prepare_uniform_data.in_set(RenderSet::Prepare));
        }

        app.add_startup_system(spawn_lights);
    }
}

fn spawn_lights(mut commands: Commands) {
    commands.spawn(DirectionalLightBundle {
        transform: default_light_transform(),
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..Default::default()
        },
        cascade_shadow_config: CascadeShadowConfig {
            bounds: vec![10000.0],
            overlap_proportion: 2.0,
            minimum_distance: 0.1,
            manual_cascades: true,
        },
        ..Default::default()
    });

    commands.insert_resource(AmbientLight {
        color: Color::rgb(1.0, 1.0, 1.0),
        brightness: 0.9,
    });
}

#[derive(Resource, Reflect, FromReflect)]
#[reflect(Resource)]
pub struct ZoneLighting {
    pub map_ambient_color: Vec3,
    pub character_ambient_color: Vec3,
    pub character_diffuse_color: Vec3,
    pub light_direction: Vec3,

    pub color_fog_enabled: bool,
    pub fog_color: Vec3,
    pub fog_density: f32,
    pub fog_min_density: f32,
    pub fog_max_density: f32,

    pub alpha_fog_enabled: bool,
    pub fog_alpha_weight_start: f32,
    pub fog_alpha_weight_end: f32,
}

impl Default for ZoneLighting {
    fn default() -> Self {
        Self {
            map_ambient_color: Vec3::ONE,
            character_ambient_color: Vec3::ONE,
            character_diffuse_color: Vec3::ONE,
            light_direction: default_light_transform().back().normalize(),
            fog_color: Vec3::new(0.2, 0.2, 0.2),
            color_fog_enabled: true,
            fog_density: 0.0018,
            fog_min_density: 0.0,
            fog_max_density: 0.75,
            alpha_fog_enabled: true,
            fog_alpha_weight_start: 0.85,
            fog_alpha_weight_end: 0.98,
        }
    }
}

#[derive(Clone, ShaderType, Resource)]
pub struct ZoneLightingUniformData {
    pub map_ambient_color: Vec4,
    pub character_ambient_color: Vec4,
    pub character_diffuse_color: Vec4,
    pub light_direction: Vec4,

    pub fog_color: Vec4,
    pub fog_density: f32,
    pub fog_min_density: f32,
    pub fog_max_density: f32,

    // TODO: Calculate camera far plane based on alpha fog:
    // far = sqrt(log2(1.0 - fog_alpha_weight_end) / (-fog_density * fog_density * 1.442695))
    pub fog_alpha_weight_start: f32,
    pub fog_alpha_weight_end: f32,
}

#[derive(Resource)]
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

fn extract_uniform_data(mut commands: Commands, zone_lighting: Extract<Res<ZoneLighting>>) {
    commands.insert_resource(ZoneLightingUniformData {
        map_ambient_color: zone_lighting.map_ambient_color.extend(1.0),
        character_ambient_color: zone_lighting.character_ambient_color.extend(1.0),
        character_diffuse_color: zone_lighting.character_diffuse_color.extend(1.0),
        light_direction: zone_lighting.light_direction.extend(1.0),
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
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetZoneLightingBindGroup<I> {
    type Param = SRes<ZoneLightingUniformMeta>;
    type ItemWorldQuery = ();
    type ViewWorldQuery = ();

    fn render<'w>(
        _: &P,
        _: ROQueryItem<'w, Self::ViewWorldQuery>,
        _: ROQueryItem<'w, Self::ItemWorldQuery>,
        meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &meta.into_inner().bind_group, &[]);

        RenderCommandResult::Success
    }
}
