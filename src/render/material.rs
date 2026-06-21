#![allow(dead_code)]
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};

#[allow(dead_code)]
#[derive(Clone, Default, ShaderType, Debug)]
pub struct EnvironmentUniform {
    pub is_fluid: u32,
    pub fluid_scroll_speed: f32,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(u32)]
pub struct VoxelMaterial {
    #[texture(0, dimension = "2d_array")]
    #[sampler(1)]
    pub texture_array: Handle<Image>,
    
    #[uniform(2)]
    pub env: EnvironmentUniform,

    pub alpha_mode: AlphaMode,
}

impl From<&VoxelMaterial> for u32 {
    fn from(material: &VoxelMaterial) -> u32 {
        material.env.is_fluid
    }
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
        if _key.bind_group_data == 1 {
            descriptor.primitive.cull_mode = None; // 🚀 流體：關閉背面剔除，允許水底往上看見水面
        } else {
            descriptor.primitive.cull_mode = Some(bevy::render::render_resource::Face::Back); // 普通固體：嚴格背面剔除
        }
        
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
