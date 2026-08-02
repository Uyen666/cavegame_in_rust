use bevy::prelude::*;
use crate::render::texture_array::load_texture_array_from_zip;
use crate::render::material::VoxelMaterial;

/// 遊戲中使用的所有方塊紋理
#[derive(Resource, Default)]
pub struct GameTextures {
    pub array_texture: Handle<Image>,
    pub material: Handle<VoxelMaterial>,
    pub fluid_material: Handle<VoxelMaterial>,
    pub ready: bool,
}

pub struct TexturePlugin;
impl Plugin for TexturePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameTextures>()
           .add_systems(PreStartup, load_textures);
    }
}

fn load_textures(
    mut gt: ResMut<GameTextures>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<VoxelMaterial>>,
    config: Res<crate::config::EngineConfig>,
) {
    let zip_path = "assets/resource_pack.zip";
    
    let image = load_texture_array_from_zip(zip_path).unwrap_or_else(|| {
        warn!("Failed to load {}, falling back to generated purple/black textures", zip_path);
        let mut fallback = Image::new_fill(
            bevy::render::render_resource::Extent3d { width: 16, height: 16 * 4, depth_or_array_layers: 1 },
            bevy::render::render_resource::TextureDimension::D2,
            &[255, 0, 255, 255],
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            bevy::render::render_asset::RenderAssetUsages::RENDER_WORLD | bevy::render::render_asset::RenderAssetUsages::MAIN_WORLD,
        );
        fallback.reinterpret_stacked_2d_as_array(4);
        fallback.sampler = bevy::render::texture::ImageSampler::Descriptor(bevy::render::texture::ImageSamplerDescriptor {
            address_mode_u: bevy::render::texture::ImageAddressMode::Repeat,
            address_mode_v: bevy::render::texture::ImageAddressMode::Repeat,
            address_mode_w: bevy::render::texture::ImageAddressMode::Repeat,
            mag_filter: bevy::render::texture::ImageFilterMode::Nearest,
            min_filter: bevy::render::texture::ImageFilterMode::Nearest,
            ..default()
        });
        fallback
    });

    let image_handle = images.add(image);
    
    let material_handle = materials.add(VoxelMaterial {
        texture_array: image_handle.clone(),
        env: crate::render::material::EnvironmentUniform {
            is_fluid: 0,
            fluid_scroll_speed: 0.0,
            sky_factor: 1.0,
        },
        alpha_mode: AlphaMode::Mask(0.5),
    });

    let fluid_material_handle = materials.add(VoxelMaterial {
        texture_array: image_handle.clone(),
        env: crate::render::material::EnvironmentUniform {
            is_fluid: 1,
            fluid_scroll_speed: config.fluid_scroll_speed,
            sky_factor: 1.0,
        },
        alpha_mode: AlphaMode::Blend,
    });

    gt.array_texture = image_handle;
    gt.material = material_handle;
    gt.fluid_material = fluid_material_handle;
    gt.ready = true;
    info!("Texture array loaded and VoxelMaterial created.");
}
