#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_clip}

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(5) color: vec4<f32>,
    @location(6) texture_index: u32, // Custom attribute
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) texture_index: u32,
};

@group(2) @binding(0) var array_texture: texture_2d_array<f32>;
@group(2) @binding(1) var array_sampler: sampler;

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    
    let model = get_world_from_local(vertex.instance_index);
    out.world_position = model * vec4<f32>(vertex.position, 1.0);
    out.world_normal = (model * vec4<f32>(vertex.normal, 0.0)).xyz;
    out.clip_position = mesh_position_local_to_clip(
        model,
        vec4<f32>(vertex.position, 1.0)
    );
    out.uv = vertex.uv;
    out.color = vertex.color;
    out.texture_index = vertex.texture_index;
    
    return out;
}

struct FragmentInput {
    @builtin(front_facing) is_front: bool,
    @builtin(position) frag_coord: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) texture_index: u32,
};

#ifdef PREPASS_PIPELINE
@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    // Minimal prepass: just return black (depth is written automatically by GPU)
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}
#else
// A very basic Lambertian lighting + ambient to simulate PBR-like looks without the huge PBR overhead
@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    // Sample texture array
    let tex_color = textureSample(array_texture, array_sampler, in.uv, in.texture_index);
    
    // Very simple directional lighting (sun)
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let ambient = 0.4;
    let normal = normalize(in.world_normal);
    
    let diffuse = max(dot(normal, light_dir), 0.0);
    let lighting = ambient + diffuse * 0.6;
    
    let final_color = tex_color.rgb * in.color.rgb * lighting;
    
    return vec4<f32>(final_color, tex_color.a);
}
#endif
