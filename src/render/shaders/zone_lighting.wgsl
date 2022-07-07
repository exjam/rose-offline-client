#define_import_path rose_client::zone_lighting

struct ZoneLighting {
    map_ambient_color: vec4<f32>;
    character_ambient_color: vec4<f32>;
    character_diffuse_color: vec4<f32>;
    fog_color: vec4<f32>;
    fog_density: f32;
    fog_alpha_range_start: f32;
    fog_alpha_range_end: f32;
};

[[group(3), binding(0)]]
var<uniform> zone_lighting: ZoneLighting;

fn apply_zone_lighting(world_position: vec4<f32>, fragment_color: vec4<f32>) -> vec4<f32> {
    let view_z = dot(vec4<f32>(
        view.inverse_view[0].z,
        view.inverse_view[1].z,
        view.inverse_view[2].z,
        view.inverse_view[3].z
    ), world_position);

    let fog_amount = clamp(1.0 - exp2(-zone_lighting.fog_density * zone_lighting.fog_density * view_z * view_z * 1.442695), 0.0, 1.0);

#ifdef ZONE_LIGHTING_CHARACTER
    let lit_color = fragment_color.rgb * zone_lighting.character_ambient_color.rgb;
#else
    let lit_color = fragment_color.rgb * zone_lighting.map_ambient_color.rgb;
#endif

    let fog_color = vec4<f32>(mix(lit_color, zone_lighting.fog_color.rgb, fog_amount), fragment_color.a);

    return fog_color;
}
