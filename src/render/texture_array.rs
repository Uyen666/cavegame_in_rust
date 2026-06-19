use bevy::prelude::*;
use bevy::render::render_resource::{TextureDimension, Extent3d, TextureFormat};
use bevy::render::texture::{ImageSampler, ImageSamplerDescriptor, ImageAddressMode, ImageFilterMode};
use std::fs::File;
use std::io::Read;
use zip::ZipArchive;
use image::imageops::FilterType;

pub const TEXTURES: &[&str] = &[
    "stone",             // 0
    "dirt",              // 1
    "grass_block_top",   // 2
    "grass_block_side",  // 3
    "water_still",       // 4
];



pub fn load_texture_array_from_zip(zip_path: &str) -> Option<Image> {
    let file = File::open(zip_path).ok()?;
    let mut archive = ZipArchive::new(file).ok()?;

    let texture_size = 16; // 強制為 16x16
    let mut layer_data = Vec::new();
    
    let base_path1 = "assets/minecraft/textures/block/";
    let base_path2 = "assets/minecraft/textures/blocks/"; // 舊版
    
    for tex_name in TEXTURES {
        let path1 = format!("{}{}.png", base_path1, tex_name);
        let path2 = format!("{}{}.png", base_path2, tex_name);
        
        let mut img_bytes = Vec::new();
        let mut read_success = false;
        
        if let Ok(mut zf) = archive.by_name(&path1) {
            if zf.read_to_end(&mut img_bytes).is_ok() {
                read_success = true;
            }
        }
        
        if !read_success {
            if let Ok(mut zf) = archive.by_name(&path2) {
                if zf.read_to_end(&mut img_bytes).is_ok() {
                    read_success = true;
                }
            }
        }
            
        let mut rgba_image = if read_success {
            match image::load_from_memory(&img_bytes) {
                Ok(img) => img.into_rgba8(),
                Err(e) => {
                    warn!("Failed to parse {}: {}", tex_name, e);
                    create_fallback_texture(texture_size)
                }
            }
        } else {
            warn!("Texture not found in zip: {}.png", tex_name);
            create_fallback_texture(texture_size)
        };
        
        // 嚴格限制解析度，避免 GPU Panic
        if rgba_image.width() != texture_size || rgba_image.height() != texture_size {
            rgba_image = image::imageops::resize(&rgba_image, texture_size, texture_size, FilterType::Nearest);
        }
        
        layer_data.extend_from_slice(rgba_image.as_raw());
    }

    let mut image = Image::new(
        Extent3d {
            width: texture_size,
            height: texture_size * (TEXTURES.len() as u32),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        layer_data,
        TextureFormat::Rgba8UnormSrgb,
        bevy::render::render_asset::RenderAssetUsages::RENDER_WORLD | bevy::render::render_asset::RenderAssetUsages::MAIN_WORLD,
    );

    image.reinterpret_stacked_2d_as_array(TEXTURES.len() as u32);

    // 採樣器設置為 Repeat 與 Nearest
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Nearest,
        min_filter: ImageFilterMode::Nearest,
        ..default()
    });

    Some(image)
}

fn create_fallback_texture(size: u32) -> image::RgbaImage {
    let mut fallback = image::RgbaImage::new(size, size);
    for y in 0..size {
        for x in 0..size {
            let color = if (x / (size / 2) + y / (size / 2)) % 2 == 0 {
                image::Rgba([255, 0, 255, 255]) // 紫色
            } else {
                image::Rgba([0, 0, 0, 255])     // 黑色
            };
            fallback.put_pixel(x, y, color);
        }
    }
    fallback
}
