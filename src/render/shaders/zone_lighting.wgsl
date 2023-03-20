#define_import_path rose_client::zone_lighting

struct ZoneLighting {
    map_ambient_color: vec4<f32>,
    character_ambient_color: vec4<f32>,
    character_diffuse_color: vec4<f32>,
    fog_color: vec4<f32>,
    fog_density: f32,
    fog_min_density: f32,
    fog_max_density: f32,
    fog_height_offset: f32,
    fog_height_falloff: f32,
    fog_alpha_range_start: f32,
    fog_alpha_range_end: f32,
};

#ifdef ZONE_LIGHTING_GROUP_2
@group(2) @binding(0)
var<uniform> zone_lighting: ZoneLighting;
#else
@group(3) @binding(0)
var<uniform> zone_lighting: ZoneLighting;
#endif

fn apply_zone_lighting_fog(world_position: vec4<f32>, fragment_color: vec4<f32>, view_z: f32) -> vec4<f32> {
    var fog_amount: f32 = clamp(1.0 - exp2(-zone_lighting.fog_density * zone_lighting.fog_density * view_z * view_z * 1.442695), 0.0, 1.0);

    if (world_position.y >= zone_lighting.fog_height_offset) {
        fog_amount = fog_amount * clamp(1.0 - (world_position.y - zone_lighting.fog_height_offset) / zone_lighting.fog_height_falloff, 0.0, 1.0);
    }

    var fog_color: vec4<f32> = vec4<f32>(mix(fragment_color.rgb, zone_lighting.fog_color.rgb, clamp(fog_amount, zone_lighting.fog_min_density, zone_lighting.fog_max_density)), fragment_color.a);

    if (fog_amount >= zone_lighting.fog_alpha_range_end) {
        discard;
    } else if (fog_amount >= zone_lighting.fog_alpha_range_start) {
        fog_color.a = fog_color.a *(1.0 - (fog_amount - zone_lighting.fog_alpha_range_start) / (zone_lighting.fog_alpha_range_end - zone_lighting.fog_alpha_range_start));
    }

    return fog_color;
}

fn apply_zone_lighting(world_position: vec4<f32>, fragment_color: vec4<f32>, view_z: f32) -> vec4<f32> {
    var fog_amount: f32 = clamp(1.0 - exp2(-zone_lighting.fog_density * zone_lighting.fog_density * view_z * view_z * 1.442695), 0.0, 1.0);

    if (world_position.y >= zone_lighting.fog_height_offset) {
        fog_amount = fog_amount * clamp(1.0 - (world_position.y - zone_lighting.fog_height_offset) / zone_lighting.fog_height_falloff, 0.0, 1.0);
    }

#ifdef ZONE_LIGHTING_CHARACTER
    let lit_color = vec4<f32>(fragment_color.rgb * zone_lighting.character_ambient_color.rgb, fragment_color.a);
#else
    let lit_color = vec4<f32>(fragment_color.rgb * zone_lighting.map_ambient_color.rgb, fragment_color.a);
#endif

    return apply_zone_lighting_fog(world_position, lit_color, view_z);
}
