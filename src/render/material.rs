use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct VoxelMaterial {
    #[texture(0, dimension = "2d_array")]
    #[sampler(1)]
    pub texture_array: Handle<Image>,
}

impl Material for VoxelMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/voxel.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "shaders/voxel.wgsl".into()
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
        ])?;
        
        // CRITICAL: Only overwrite the first buffer (mesh vertex data).
        // Bevy 0.14 uses subsequent buffers for GPU instancing (which expects Location[7] etc).
        descriptor.vertex.buffers[0] = vertex_layout;
        
        Ok(())
    }
}
