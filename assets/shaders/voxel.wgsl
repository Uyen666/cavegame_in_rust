#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_clip}
#import bevy_pbr::mesh_view_bindings as view_bindings

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) packed_data: u32,
    @location(1) flow_vector: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) @interpolate(perspective, center) uv: vec2<f32>,
    @location(3) texture_index: u32,
    @location(4) sky_light: f32,
    @location(5) flow_vector: vec2<f32>,
};

@group(2) @binding(0) var array_texture: texture_2d_array<f32>;
@group(2) @binding(1) var array_sampler: sampler;

struct EnvironmentUniform {
    is_fluid: u32,
    fluid_scroll_speed: f32,
};
@group(2) @binding(2) var<uniform> env: EnvironmentUniform;

@vertex
fn vertex(
    input: VertexInput,
    @builtin(vertex_index) vertex_index: u32
) -> VertexOutput {
    var out: VertexOutput;
    
    let x = f32(input.packed_data & 0x3Fu);
    let y = f32((input.packed_data >> 6u) & 0x3Fu);
    let z = f32((input.packed_data >> 12u) & 0x3Fu);
    let face_id = (input.packed_data >> 18u) & 0x7u;
    var tex_id = (input.packed_data >> 21u) & 0x7Fu;
    let sky_light_int = (input.packed_data >> 28u) & 0x0Fu;

    var y_offset_down = 0.0;
    if env.is_fluid == 1u {
        tex_id = (input.packed_data >> 21u) & 0x0Fu;
        y_offset_down = f32((input.packed_data >> 25u) & 7u) / 8.0;
    }

    let local_pos = vec3<f32>(x, max(y - y_offset_down, -1.0), z);
    
    var normal: vec3<f32>;
    var uv: vec2<f32>;
    
    switch (face_id) {
        case 0u: { normal = vec3<f32>(1.0, 0.0, 0.0);  uv = vec2<f32>(z, -y); }
        case 1u: { normal = vec3<f32>(-1.0, 0.0, 0.0); uv = vec2<f32>(z, -y); }
        case 2u: { normal = vec3<f32>(0.0, 1.0, 0.0);  uv = vec2<f32>(x, z); }
        case 3u: { normal = vec3<f32>(0.0, -1.0, 0.0); uv = vec2<f32>(x, z); }
        case 4u: { normal = vec3<f32>(0.0, 0.0, 1.0);  uv = vec2<f32>(x, -y); }
        case 5u: { normal = vec3<f32>(0.0, 0.0, -1.0); uv = vec2<f32>(x, -y); }
        default: { normal = vec3<f32>(0.0, 1.0, 0.0);  uv = vec2<f32>(0.0, 0.0); }
    }
    
    var tangent: vec3<f32>;
    var bitangent: vec3<f32>;
    switch (face_id) {
        case 0u, 1u: {
            tangent   = vec3<f32>(0.0, 1.0, 0.0);
            bitangent = vec3<f32>(0.0, 0.0, 1.0);
        }
        case 2u, 3u: {
            tangent   = vec3<f32>(1.0, 0.0, 0.0);
            bitangent = vec3<f32>(0.0, 0.0, 1.0);
        }
        default: {
            tangent   = vec3<f32>(1.0, 0.0, 0.0);
            bitangent = vec3<f32>(0.0, 1.0, 0.0);
        }
    }
    
    var final_pos = local_pos;
    if env.is_fluid == 0u {
        let vid = vertex_index % 4u;
        let quad_sign_x = select(-1.0, 1.0, (vid == 1u || vid == 2u));
        let quad_sign_y = select(-1.0, 1.0, (vid == 2u || vid == 3u));
        let edge_bias = tangent * quad_sign_x * 0.0005 + bitangent * quad_sign_y * 0.0005;
        final_pos = local_pos + edge_bias;
    }

    let model = get_world_from_local(input.instance_index);
    out.world_position = model * vec4<f32>(final_pos, 1.0);
    out.world_normal = (model * vec4<f32>(normal, 0.0)).xyz;
    out.clip_position = mesh_position_local_to_clip(
        model,
        vec4<f32>(final_pos, 1.0)
    );
    out.uv = uv;
    out.texture_index = tex_id;
    out.sky_light = f32(sky_light_int);
    out.flow_vector = input.flow_vector;
    
    return out;
}

struct FragmentInput {
    @builtin(front_facing) is_front: bool,
    @builtin(position) frag_coord: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) @interpolate(perspective, center) uv: vec2<f32>,
    @location(3) texture_index: u32,
    @location(4) sky_light: f32,
    @location(5) flow_vector: vec2<f32>,
};

#ifdef PREPASS_PIPELINE
@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}
#else
@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    var animated_uv = in.uv;
    if env.is_fluid == 1u && in.texture_index == 4u {
        var world_uv = in.uv;
        let abs_n = abs(in.world_normal);
        if abs_n.y > 0.5 {
            world_uv = fract(in.world_position.xz);
        } else if abs_n.x > 0.5 {
            world_uv = fract(vec2<f32>(in.world_position.z, -in.world_position.y));
        } else {
            world_uv = fract(vec2<f32>(in.world_position.x, -in.world_position.y));
        }
        animated_uv = fract(world_uv + view_bindings::globals.time * env.fluid_scroll_speed * in.flow_vector);
    }

    let tex_color = textureSample(array_texture, array_sampler, animated_uv, in.texture_index);
    let light_ratio = in.sky_light / 15.0;
    let shadow_intensity = 0.08 + (1.0 - 0.08) * (light_ratio * light_ratio);
    
    var final_rgb = tex_color.rgb * shadow_intensity;
    var final_alpha = 1.0; 

    if env.is_fluid == 1u && in.texture_index == 4u {
        let water_tint = vec4<f32>(0.15, 0.35, 0.75, 0.55);
        final_rgb = final_rgb * water_tint.rgb;
        final_alpha = water_tint.a;
    }
    
    return vec4<f32>(final_rgb, final_alpha);
}
#endif
