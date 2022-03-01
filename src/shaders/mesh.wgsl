#import mesh_pipeline::mesh_view_bind_group
#import mesh_pipeline::mesh_struct

struct Vertex {
    [[location(0)]] position: vec3<f32>;

#ifdef HAS_UV1
    [[location(1)]] uv1: vec2<f32>;
#endif

#ifdef HAS_UV2
    [[location(2)]] uv2: vec2<f32>;
#endif

#ifdef HAS_TILE_INFO
    [[location(3)]] terrain_tile_info: vec3<i32>;
#endif
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] world_position: vec4<f32>;

#ifdef HAS_UV1
    [[location(1)]] uv1: vec2<f32>;
#endif

#ifdef HAS_UV2
    [[location(2)]] uv2: vec2<f32>;
#endif
};

[[group(2), binding(0)]]
var<uniform> mesh: Mesh;

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    let world_position = mesh.model * vec4<f32>(vertex.position, 1.0);

    var out: VertexOutput;

#ifdef HAS_UV1
    out.uv1 = vertex.uv1;
#endif

#ifdef HAS_UV2
    out.uv2 = vertex.uv2;
#endif

    out.world_position = world_position;
    out.clip_position = view.view_proj * world_position;
    return out;
}

struct FragmentInput {
    [[builtin(front_facing)]] is_front: bool;
    [[location(0)]] world_position: vec4<f32>;
#ifdef HAS_UV1
    [[location(1)]] uv1: vec2<f32>;
#endif
#ifdef HAS_UV2
    [[location(2)]] uv2: vec2<f32>;
#endif
};

[[stage(fragment)]]
fn fragment(in: FragmentInput) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(1.0, 0.0, 1.0, 1.0);
}