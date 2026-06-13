use bevy::prelude::*;
use bevy::render::texture::{ImageSampler, ImageSamplerDescriptor, ImageAddressMode, ImageFilterMode};

/// 遊戲中使用的所有方塊紋理
#[derive(Resource, Default)]
pub struct GameTextures {
    pub grass: Handle<Image>,
    pub stone: Handle<Image>,
    pub ready: bool,
}

pub struct TexturePlugin;
impl Plugin for TexturePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameTextures>()
           .add_systems(PreStartup, load_textures)
           .add_systems(Update, setup_samplers);
    }
}

fn load_textures(
    mut gt: ResMut<GameTextures>,
    asset_server: Res<AssetServer>,
) {
    gt.grass = asset_server.load("textures/grass.png");
    gt.stone = asset_server.load("textures/stone.png");
}

/// 等所有紋理都載入完成後，把採樣器設為 Repeat（UV 超過 1.0 就自動 tile）
fn setup_samplers(
    mut gt: ResMut<GameTextures>,
    mut images: ResMut<Assets<Image>>,
) {
    if gt.ready { return; }

    // 必須兩張都載入完才算 ready
    let repeat_sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Nearest, // pixel-art 銳利感，像 Minecraft
        min_filter: ImageFilterMode::Nearest,
        ..default()
    });

    let Some(grass_img) = images.get_mut(&gt.grass) else { return; };
    grass_img.sampler = repeat_sampler.clone();

    let Some(stone_img) = images.get_mut(&gt.stone) else { return; };
    stone_img.sampler = repeat_sampler;

    gt.ready = true;
    info!("All block textures ready (Repeat sampler applied).");
}
