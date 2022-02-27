#import mesh_pipeline::mesh_view_bind_group
#import mesh_pipeline::mesh_struct

[[group(2), binding(0)]]
var<uniform> mesh: Mesh;

struct StaticMeshMaterialLightmapData {
    lightmap_uv_offset: vec2<f32>;
    lightmap_uv_scale: f32;
};

[[group(1), binding(0)]]
var base_texture: texture_2d<f32>;
[[group(1), binding(1)]]
var base_sampler: sampler;
[[group(1), binding(2)]]
var<uniform> lightmap_data: StaticMeshMaterialLightmapData;
[[group(1), binding(3)]]
var lightmap_texture: texture_2d<f32>;
[[group(1), binding(4)]]
var lightmap_sampler: sampler;

struct FragmentInput {
    [[builtin(front_facing)]] is_front: bool;
    [[builtin(position)]] frag_coord: vec4<f32>;
    [[location(0)]] world_position: vec4<f32>;
    [[location(1)]] uv1: vec2<f32>;
    [[location(2)]] uv2: vec2<f32>;
};

[[stage(fragment)]]
fn fragment(in: FragmentInput) -> [[location(0)]] vec4<f32> {
    var output_color: vec4<f32> = textureSample(base_texture, base_sampler, in.uv1);
#ifdef HAS_STATIC_MESH_LIGHTMAP
    output_color = output_color * textureSample(lightmap_texture, lightmap_sampler, (in.uv2 + lightmap_data.lightmap_uv_offset) * lightmap_data.lightmap_uv_scale) * 2.0;
#endif
    return output_color;
}
