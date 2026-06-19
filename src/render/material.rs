#![allow(dead_code)]
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};

#[allow(dead_code)]
#[derive(Clone, Default, ShaderType, Debug)]
pub struct EnvironmentUniform {
    pub fog_color: LinearRgba,
    pub camera_pos: Vec3,
    pub fog_start: f32,
    pub fog_end: f32,
    pub is_fluid: u32,
    pub fluid_scroll_speed: f32,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct VoxelMaterial {
    #[texture(0, dimension = "2d_array")]
    #[sampler(1)]
    pub texture_array: Handle<Image>,
    
    #[uniform(2)]
    pub env: EnvironmentUniform,

    pub alpha_mode: AlphaMode,
}

impl Material for VoxelMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/voxel.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "shaders/voxel.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }

    fn prepass_vertex_shader() -> ShaderRef {
        "shaders/voxel.wgsl".into()
    }

    fn prepass_fragment_shader() -> ShaderRef {
        "shaders/voxel.wgsl".into()
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline<Self>,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        layout: &bevy::render::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = Some(bevy::render::render_resource::Face::Back);
        
        let vertex_layout = layout.0.get_layout(&[
            crate::render::greedy::ATTRIBUTE_PACKED_DATA.at_shader_location(0),
            crate::render::greedy::ATTRIBUTE_FLOW_VECTOR.at_shader_location(1),
        ])?;
        
        // CRITICAL: Only overwrite the first buffer (mesh vertex data).
        // Bevy 0.14 uses subsequent buffers for GPU instancing (which expects Location[7] etc).
        descriptor.vertex.buffers[0] = vertex_layout;
        
        Ok(())
    }
}
