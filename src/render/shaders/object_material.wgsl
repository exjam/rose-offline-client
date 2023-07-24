#import bevy_pbr::mesh_types Mesh, SkinnedMesh
#import bevy_pbr::mesh_view_bindings view
#import bevy_pbr::mesh_bindings mesh
#import bevy_pbr::mesh_functions mesh_position_local_to_world, mesh_normal_local_to_world, mesh_position_world_to_clip
#import bevy_pbr::shadows fetch_directional_shadow
#import rose_client::zone_lighting apply_zone_lighting

#ifdef SKINNED
#import bevy_pbr::skinning skin_normals, skin_model
#endif

@group(1) @binding(0)
var<uniform> material: StaticMeshMaterialData;
@group(1) @binding(1)
var base_texture: texture_2d<f32>;
@group(1) @binding(2)
var base_sampler: sampler;
@group(1) @binding(3)
var lightmap_texture: texture_2d<f32>;
@group(1) @binding(4)
var lightmap_sampler: sampler;
@group(1) @binding(5)
var specular_texture: texture_2d<f32>;
@group(1) @binding(6)
var specular_sampler: sampler;

struct Vertex {
#ifdef VERTEX_POSITIONS
    @location(0) position: vec3<f32>,
#endif

#ifdef VERTEX_NORMALS
    @location(1) normal: vec3<f32>,
#endif

#ifdef VERTEX_UVS
    @location(2) uv: vec2<f32>,
#endif

#ifdef VERTEX_UVS_LIGHTMAP
    @location(3) lightmap_uv: vec2<f32>,
#endif

#ifdef SKINNED
    @location(4) joint_indices: vec4<u32>,
    @location(5) joint_weights: vec4<f32>,
#endif
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,

    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,

#ifdef VERTEX_UVS
    @location(2) uv: vec2<f32>,
#endif

#ifdef VERTEX_UVS_LIGHTMAP
    @location(3) lightmap_uv: vec2<f32>,
#endif
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

#ifdef SKINNED
    var model = skin_model(vertex.joint_indices, vertex.joint_weights);
#else
    var model = mesh.model;
#endif

#ifdef VERTEX_NORMALS
#ifdef SKINNED
    out.world_normal = skin_normals(model, vertex.normal);
#else
    out.world_normal = mesh_normal_local_to_world(vertex.normal);
#endif
#endif

#ifdef VERTEX_POSITIONS
    out.world_position = mesh_position_local_to_world(model, vec4<f32>(vertex.position, 1.0));
    out.clip_position = mesh_position_world_to_clip(out.world_position);
#endif

#ifdef VERTEX_UVS
    out.uv = vertex.uv;
#endif

#ifdef VERTEX_UVS_LIGHTMAP
    out.lightmap_uv = vertex.lightmap_uv;
#endif

    return out;
}

struct StaticMeshMaterialData {
    flags: u32,
    alpha_cutoff: f32,
    alpha_value: f32,
    lightmap_uv_offset: vec2<f32>,
    lightmap_uv_scale: f32,
};

const OBJECT_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE: u32              = 1u;
const OBJECT_MATERIAL_FLAGS_ALPHA_MODE_MASK: u32                = 2u;
const OBJECT_MATERIAL_FLAGS_ALPHA_MODE_BLEND: u32               = 4u;
const OBJECT_MATERIAL_FLAGS_HAS_ALPHA_VALUE: u32                = 8u;
const OBJECT_MATERIAL_FLAGS_SPECULAR: u32                       = 16u;

struct FragmentInput {
    @builtin(position) frag_coord: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,

#ifdef VERTEX_UVS
    @location(2) uv: vec2<f32>,
#endif

#ifdef VERTEX_UVS_LIGHTMAP
    @location(3) lightmap_uv: vec2<f32>,
#endif
};

#ifdef DEPTH_PREPASS

@fragment
fn fragment(in: FragmentInput) {
    var output_color: vec4<f32> = textureSample(base_texture, base_sampler, in.uv);
    if ((material.flags & OBJECT_MATERIAL_FLAGS_ALPHA_MODE_MASK) != 0u) {
        if (output_color.a < material.alpha_cutoff) {
            discard;
        }
    }
}

#else // ifdef DEPTH_PREPASS

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    var output_color: vec4<f32> = textureSample(base_texture, base_sampler, in.uv);
    let view_z = dot(vec4<f32>(
        view.inverse_view[0].z,
        view.inverse_view[1].z,
        view.inverse_view[2].z,
        view.inverse_view[3].z
    ), in.world_position);

#ifdef VERTEX_UVS_LIGHTMAP
    let shadow = fetch_directional_shadow(0u, in.world_position, in.world_normal, view_z);
    output_color = vec4<f32>(output_color.xyz * (shadow * 0.2 + 0.8), output_color.w);

    var lightmap = textureSample(lightmap_texture, lightmap_sampler, (in.lightmap_uv + material.lightmap_uv_offset) * material.lightmap_uv_scale);
    output_color = vec4<f32>(output_color.xyz * lightmap.xyz * 2.0, output_color.w);
#endif

    if ((material.flags & OBJECT_MATERIAL_FLAGS_SPECULAR) != 0u) {
        let N = normalize(in.world_normal);
        let V = normalize(view.world_position.xyz - in.world_position.xyz);
        let R = reflect(-V, N);
        output_color = vec4<f32>(output_color.rgb + output_color.a * textureSample(specular_texture, specular_sampler, R.xy * 0.5 + vec2<f32>(0.5, 0.5)).rgb, output_color.a);
    }

    if ((material.flags & OBJECT_MATERIAL_FLAGS_HAS_ALPHA_VALUE) != 0u) {
        output_color.a = material.alpha_value;
    } else if ((material.flags & OBJECT_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE) != 0u) {
        // NOTE: If rendering as opaque, alpha should be ignored so set to 1.0
        output_color.a = 1.0;
    } else if ((material.flags & OBJECT_MATERIAL_FLAGS_ALPHA_MODE_MASK) != 0u) {
        if (output_color.a >= material.alpha_cutoff) {
            // NOTE: If rendering as masked alpha and >= the cutoff, render as fully opaque
            output_color.a = 1.0;
        } else {
            // NOTE: output_color.a < material.alpha_cutoff should not is not rendered
            // NOTE: This and any other discards mean that early-z testing cannot be done!
            discard;
        }
    }

    return apply_zone_lighting(in.world_position, in.world_normal, output_color, view_z);
}

#endif  // else ifdef DEPTH_PREPASS
