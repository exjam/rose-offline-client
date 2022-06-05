#import bevy_pbr::mesh_types
#import bevy_pbr::mesh_view_bindings

[[group(2), binding(0)]]
var<uniform> mesh: Mesh;

struct Vertex {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] uv: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] uv: vec2<f32>;
};

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    let untranslated_inv_view =  mat4x4<f32>(view.inverse_view.x.xyzw,
                                             view.inverse_view.y.xyzw,
                                             view.inverse_view.z.xyzw,
                                             vec4<f32>(0.0, 0.0, 0.0, 1.0));
    let untranslated_proj = view.projection * untranslated_inv_view;
    let untranslated_model = mat4x4<f32>(mesh.model.x.xyzw,
                                         mesh.model.y.xyzw,
                                         mesh.model.z.xyzw,
                                         vec4<f32>(0.0, 0.0, 0.0, 1.0));
    let pos = untranslated_proj * untranslated_model * vec4<f32>(vertex.position, 1.0);

    var out: VertexOutput;
    out.clip_position = pos.xyww;
    out.uv = vertex.uv;
    return out;
}

[[group(1), binding(0)]]
var sky_texture_day: texture_2d<f32>;
[[group(1), binding(1)]]
var sky_sampler_day: sampler;
[[group(1), binding(2)]]
var sky_texture_night: texture_2d<f32>;
[[group(1), binding(3)]]
var sky_sampler_night: sampler;

struct SkyData {
    day_weight: f32;
};
[[group(3), binding(0)]]
var<uniform> sky_data: SkyData;

struct FragmentInput {
    [[builtin(position)]] frag_coord: vec4<f32>;
    [[location(0)]] uv: vec2<f32>;
};

[[stage(fragment)]]
fn fragment(in: FragmentInput) -> [[location(0)]] vec4<f32> {
    var color_day: vec4<f32> = textureSample(sky_texture_day, sky_sampler_day, in.uv);
    var color_night: vec4<f32> = textureSample(sky_texture_night, sky_sampler_night, in.uv);
    var output_color: vec4<f32> = pow(mix(color_night, color_day, sky_data.day_weight), vec4<f32>(2.2));
    output_color.a = 1.0;
    return output_color;
}
