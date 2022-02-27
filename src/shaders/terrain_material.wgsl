#import mesh_pipeline::mesh_view_bind_group
#import mesh_pipeline::mesh_struct

[[group(2), binding(0)]]
var<uniform> mesh: Mesh;

[[group(1), binding(0)]]
var lightmap_texture: texture_2d<f32>;
[[group(1), binding(1)]]
var lightmap_sampler: sampler;
[[group(1), binding(2)]]
var tile_array_texture: texture_2d_array<f32>;
[[group(1), binding(3)]]
var tile_array_sampler: sampler;

struct FragmentInput {
    [[builtin(front_facing)]] is_front: bool;
    [[builtin(position)]] frag_coord: vec4<f32>;
    [[location(0)]] world_position: vec4<f32>;
    [[location(1)]] uv1: vec2<f32>;
    [[location(2)]] uv2: vec2<f32>;
    [[location(3)]] terrain_tile_info: vec3<i32>;
};

[[stage(fragment)]]
fn fragment(in: FragmentInput) -> [[location(0)]] vec4<f32> {
    var lightmap: vec4<f32> = textureSample(lightmap_texture, lightmap_sampler, in.uv1);
    var tile_layer1_id: i32 = in.terrain_tile_info.x;
    var tile_layer2_id: i32 = in.terrain_tile_info.y;
    var tile_rotation: i32 = in.terrain_tile_info.z;
    var layer1: vec4<f32> = textureSample(tile_array_texture, tile_array_sampler, in.uv2, tile_layer1_id);

    var layer2_uv: vec2<f32> = in.uv2;
    if (tile_rotation == 2) {
        layer2_uv.x = 1.0 - layer2_uv.x;
    } else if (tile_rotation == 3) {
        layer2_uv.y = 1.0 - layer2_uv.y;
    } else if (tile_rotation == 4) {
        layer2_uv.x = 1.0 - layer2_uv.x;
        layer2_uv.y = 1.0 - layer2_uv.y;
    } else if (tile_rotation == 5) {
        var x: f32 = layer2_uv.x;
        layer2_uv.x = layer2_uv.y;
        layer2_uv.y = 1.0 - x;
    } else if (tile_rotation == 6) {
        var x: f32 = layer2_uv.x;
        layer2_uv.x = layer2_uv.y;
        layer2_uv.y = x;
    }
    var layer2: vec4<f32> = textureSample(tile_array_texture, tile_array_sampler, layer2_uv, tile_layer2_id);

    var output_color: vec4<f32> = mix(layer1, layer2, layer2.a) * lightmap * 2.0;
    return output_color;
}
