#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_clip}
#import bevy_pbr::mesh_view_bindings as view_bindings

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) color: vec4<f32>,
};

struct LODEnvironmentUniform {
    near_voxel_radius: f32,
    fog_start: f32,
    fog_end: f32,
    _pad0: f32,
    fog_color: vec4<f32>,
};

@group(2) @binding(0) var<uniform> env: LODEnvironmentUniform;

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let model = get_world_from_local(input.instance_index);
    let world_pos = model * vec4<f32>(input.position, 1.0);
    out.world_position = world_pos;
    out.clip_position = mesh_position_local_to_clip(model, vec4<f32>(input.position, 1.0));
    out.color = input.color;
    return out;
}

#ifdef PREPASS_PIPELINE
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}
#else
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let cam_pos = view_bindings::view.world_position;
    let dist = length(in.world_position.xz - cam_pos.xz);

    // 🚀 核心防線：嚴格剔除近景體素半徑 (near_voxel_radius) 內的所有 LOD 像素！
    // 確保玩家近處 100% 只有純粹 3D 方塊，絕不允許任何平滑地表穿出樹木與草地！
    if dist < env.near_voxel_radius {
        discard;
    }

    // 遠景大氣迷霧平滑過渡至地平線天空色 (650m ~ 1,450m)
    let fog_range = max(env.fog_end - env.fog_start, 1.0);
    let fog_factor = clamp((dist - env.fog_start) / fog_range, 0.0, 1.0);
    let final_rgb = mix(in.color.rgb, env.fog_color.rgb, fog_factor);

    return vec4<f32>(final_rgb, 1.0);
}
#endif
