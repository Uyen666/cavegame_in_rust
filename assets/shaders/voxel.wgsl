#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_clip}

struct Vertex {
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
    fog_color: vec4<f32>,
    camera_pos: vec3<f32>,
    fog_start: f32,
    fog_end: f32,
    is_fluid: u32,
    time: f32,
    fluid_scroll_speed: f32,
};
@group(2) @binding(2) var<uniform> env: EnvironmentUniform;

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    
    let x = f32(vertex.packed_data & 0x3Fu);
    let y = f32((vertex.packed_data >> 6u) & 0x3Fu);
    let z = f32((vertex.packed_data >> 12u) & 0x3Fu);
    let face_id = (vertex.packed_data >> 18u) & 0x07u;
    var tex_id = (vertex.packed_data >> 21u) & 0x7Fu;
    let sky_light_int = (vertex.packed_data >> 28u) & 0x0Fu;

    var y_offset_down = 0.0;
    if env.is_fluid == 1u {
        tex_id = (vertex.packed_data >> 21u) & 0x0Fu;
        y_offset_down = f32((vertex.packed_data >> 25u) & 7u) / 8.0;
    }

    var local_pos = vec3<f32>(x, y - y_offset_down, z);
    
    // 防禦性 Clamp：確保流體頂點不會被錯誤的 offset 拉入虛空
    local_pos.y = max(local_pos.y, -1.0);
    
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
    
    let model = get_world_from_local(vertex.instance_index);
    out.world_position = model * vec4<f32>(local_pos, 1.0);
    out.world_normal = (model * vec4<f32>(normal, 0.0)).xyz;
    out.clip_position = mesh_position_local_to_clip(
        model,
        vec4<f32>(local_pos, 1.0)
    );
    out.uv = uv;
    out.texture_index = tex_id;
    out.sky_light = f32(sky_light_int);
    out.flow_vector = vertex.flow_vector;
    
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
    // Minimal prepass: just return black (depth is written automatically by GPU)
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}
#else
// Non-linear Sky Light Propagation Lighting
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
        
        animated_uv = fract(world_uv + env.time * env.fluid_scroll_speed * in.flow_vector);
    }

    // Sample texture array
    let tex_color = textureSample(array_texture, array_sampler, animated_uv, in.texture_index);
    
    let light_ratio = in.sky_light / 15.0;
    let shadow_intensity = 0.06 + (1.0 - 0.06) * (light_ratio * light_ratio);
    
    var final_rgb = tex_color.rgb * shadow_intensity;
    var final_alpha = tex_color.a;

    if env.is_fluid == 1u && in.texture_index == 4u {
        let water_tint = vec4<f32>(0.15, 0.35, 0.75, 0.55);
        final_rgb = final_rgb * water_tint.rgb;
        final_alpha = water_tint.a;
    }
    
    let dist = length(in.world_position.xyz - env.camera_pos);
    let fog_factor = smoothstep(env.fog_start, env.fog_end, dist);
    let fogged_color = mix(final_rgb, env.fog_color.rgb, fog_factor);
    
    return vec4<f32>(fogged_color, final_alpha);
}
#endif
