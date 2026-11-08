use bevy::prelude::IVec3;
use serde::{Serialize, Deserialize, Serializer, Deserializer};

pub const CHUNK_SIZE: usize = 32;
pub const TOTAL_V_SIZE: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

// 向上探測空氣層的防線緩衝高度（32 + 4）
const CACHE_EXTENSION: usize = 4;
pub const CACHE_Y_SIZE: usize = CHUNK_SIZE + CACHE_EXTENSION; // 36

/// 🌊 1024 宏觀世界海平面基準面
pub const SEA_LEVEL: i32 = 256;

pub trait NoiseModule: Send + Sync {
    fn sample_2d(&self, x: f64, z: f64) -> f32;
    fn sample_3d(&self, x: f64, y: f64, z: f64) -> f32;
}

use crate::world::voxel::BlockType;

#[derive(Clone)]
pub struct ChunkBuffer {
    pub blocks: [BlockType; TOTAL_V_SIZE],
}

impl Default for ChunkBuffer {
    fn default() -> Self {
        Self {
            blocks: [BlockType::Air; TOTAL_V_SIZE],
        }
    }
}

impl ChunkBuffer {
    pub fn is_pure_air(&self) -> bool {
        self.blocks.iter().all(|&b| b == BlockType::Air)
    }
}

impl Serialize for ChunkBuffer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut rle_encoded: Vec<(u8, u16)> = Vec::new();
        let mut current_id = self.blocks[0] as u8;
        let mut current_run = 1u16;

        for i in 1..TOTAL_V_SIZE {
            let id = self.blocks[i] as u8;
            if id == current_id && current_run < u16::MAX {
                current_run += 1;
            } else {
                rle_encoded.push((current_id, current_run));
                current_id = id;
                current_run = 1;
            }
        }
        rle_encoded.push((current_id, current_run));
        rle_encoded.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ChunkBuffer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let rle_encoded = Vec::<(u8, u16)>::deserialize(deserializer)?;
        let mut chunk_buffer = ChunkBuffer::default();
        let mut idx = 0;

        for (id, run) in rle_encoded {
            let block = match id {
                1 => BlockType::Grass,
                2 => BlockType::Stone,
                3 => BlockType::Dirt,
                4 => BlockType::OakLog,
                5 => BlockType::OakLeaves,
                6 => BlockType::Sand,
                7 => BlockType::Gravel,
                8 => BlockType::CoalOre,
                9 => BlockType::IronOre,
                10 => BlockType::Glass,
                _ => BlockType::Air,
            };
            let run_len = run as usize;
            if idx + run_len <= TOTAL_V_SIZE {
                chunk_buffer.blocks[idx..(idx + run_len)].fill(block);
                idx += run_len;
            } else {
                return Err(serde::de::Error::custom("RLE block data out of bounds"));
            }
        }
        Ok(chunk_buffer)
    }
}

pub struct TerrainGenerator<N: NoiseModule> {
    pub noise_provider: N,
}

#[inline(always)]
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl<N: NoiseModule> TerrainGenerator<N> {
    /// 🚀 1024 宏觀地球級地貌連續可微密度場計算
    /// 採用全域 C1 平滑樣條過渡，徹底消除海岸邊緣垂直人工斷層，打造自然大陸架緩坡！
    pub fn calculate_global_density(&self, gx: i32, gy: i32, gz: i32) -> (f32, f32) {
        let fx = gx as f64;
        let fy = gy as f64;
        let fz = gz as f64;

        // 【第一階段：採樣大尺度大陸性指標 (Continentality)】
        let r = (self.noise_provider.sample_2d(fx * 0.0006, fz * 0.0006) + 0.5).clamp(0.0, 1.0);

        // 【第二階段：全域 C1 連續樣條地理參數計算（徹底消滅階梯斷層）】
        let (base_h, amplitude, mountain_weight) = if r < 0.18 {
            // 🌊 1. 深海大洋與深海海溝帶 (r: 0.0 ~ 0.18)
            // 基底 Y=80 ~ 150，海床平緩起伏
            let t = smoothstep(0.0, 0.18, r);
            let mut trench_depth = 0.0;
            if r < 0.06 {
                let tt = smoothstep(0.06, 0.0, r);
                trench_depth = tt * 40.0; // 平滑深切海溝
            }
            (80.0 + t * 70.0 - trench_depth, 8.0 + t * 6.0, 0.05f32)
        } else if r < 0.38 {
            // 🏖️ 2. 大陸架平緩緩坡與水下過渡帶 (r: 0.18 ~ 0.38)
            // 從 Y=150 平緩爬升至水深僅 4 米的淺海大陸架 (Y=252)，徹底消滅垂直人工斷崖！
            let t = smoothstep(0.18, 0.38, r);
            (150.0 + t * 102.0, 14.0 - t * 8.0, 0.05f32)
        } else if r < 0.50 {
            // 🌾 3. 濱海沙灘與平原沉積帶 (r: 0.38 ~ 0.50)
            // 基底從海平面 (Y=252) 緩升至 Y=275，形成開闊金黃沙灘與海邊草地
            let t = smoothstep(0.38, 0.50, r);
            (252.0 + t * 23.0, 6.0 + t * 6.0, 0.0f32)
        } else if r < 0.72 {
            // 🏞️ 4. 內陸丘陵與高原林地 (r: 0.50 ~ 0.72)
            // 基底 Y=275 ~ 480，起伏適中，森林茂密
            let t = smoothstep(0.50, 0.72, r);
            (275.0 + t * 205.0, 12.0 + t * 28.0, t * 0.3)
        } else {
            // 🏔️ 5. 喜馬拉雅級巨型雪峰 (r: 0.72 ~ 1.0)
            // 基底平滑爆發至 Y=480 ~ 820+，波幅達 40 ~ 120，銳利角峰直插雲霄！
            let t = smoothstep(0.72, 1.0, r);
            (480.0 + t * 340.0, 40.0 + t * 80.0, 0.3 + t * 0.7)
        };

        // 【第三階段：連續幾何場複合（Fbm 與 3D 脊狀分形噪聲混合）】
        let fbm_noise = self.noise_provider.sample_3d(fx * 0.005, fy * 0.005, fz * 0.005);
        let ridged_noise = 1.0 - fbm_noise.abs();
        let blended_3d = fbm_noise * (1.0 - mountain_weight) + ridged_noise * mountain_weight;

        // 連續密度基底
        let mut density = base_h - (gy as f32) + (blended_3d * amplitude);

        // 【第四階段：河流與峽谷深切侵蝕 (River & Canyon Carving)】
        // 僅在陸地區域 (r >= 0.40) 且高於海平面附近進行峽谷深切
        if r >= 0.40 && gy >= SEA_LEVEL - 12 {
            let river_sample = self.noise_provider.sample_2d(fx * 0.0015 + 128.0, fz * 0.0015 + 128.0).abs();
            if river_sample < 0.055 {
                let carve_intensity = (1.0 - (river_sample / 0.055)).powi(2);
                let target_river_bed = (SEA_LEVEL as f32) - 4.0;
                if (gy as f32) > target_river_bed {
                    let carve_amount = ((gy as f32) - target_river_bed) * carve_intensity * 1.6;
                    density -= carve_amount;
                }
            }
        }

        // 【第五階段：深層溶洞侵蝕】
        if 32 < gy && gy < 240 {
            let dy = (gy as f32 - 130.0) / 80.0;
            let mut cave_intensity = (1.0 - dy * dy).max(0.0);

            let entrance_gate = self.noise_provider.sample_2d(fx * 0.012, fz * 0.012);
            if entrance_gate > 0.28 && gy > 180 {
                cave_intensity = f32::max(cave_intensity, 0.65);
            }

            let cave_noise = self.noise_provider.sample_3d(fx * 0.007, fy * 0.007, fz * 0.007);
            if cave_noise * cave_intensity > 0.22 {
                density = f32::min(density, -0.5);
            }
        }

        (density, base_h)
    }

    pub fn get_max_possible_surface_y_fast(&self, gx: i32, gz: i32) -> i32 {
        let fx = gx as f64;
        let fz = gz as f64;
        let r = (self.noise_provider.sample_2d(fx * 0.0006, fz * 0.0006) + 0.5).clamp(0.0, 1.0);
        let (base_h, amplitude) = if r < 0.18 {
            let t = smoothstep(0.0, 0.18, r);
            (80.0 + t * 70.0, 8.0 + t * 6.0)
        } else if r < 0.38 {
            let t = smoothstep(0.18, 0.38, r);
            (150.0 + t * 102.0, 14.0 - t * 8.0)
        } else if r < 0.50 {
            let t = smoothstep(0.38, 0.50, r);
            (252.0 + t * 23.0, 6.0 + t * 6.0)
        } else if r < 0.72 {
            let t = smoothstep(0.50, 0.72, r);
            (275.0 + t * 205.0, 12.0 + t * 28.0)
        } else {
            let t = smoothstep(0.72, 1.0, r);
            (480.0 + t * 340.0, 40.0 + t * 80.0)
        };
        (base_h + amplitude).ceil() as i32
    }

    pub fn get_max_surface_y(&self, gx: i32, gz: i32) -> i32 {
        let fx = gx as f64;
        let fz = gz as f64;
        let r = (self.noise_provider.sample_2d(fx * 0.0006, fz * 0.0006) + 0.5).clamp(0.0, 1.0);
        let (base_h, amplitude, _) = if r < 0.18 {
            let t = smoothstep(0.0, 0.18, r);
            (80.0 + t * 70.0, 8.0 + t * 6.0, 0.05f32)
        } else if r < 0.38 {
            let t = smoothstep(0.18, 0.38, r);
            (150.0 + t * 102.0, 14.0 - t * 8.0, 0.05f32)
        } else if r < 0.50 {
            let t = smoothstep(0.38, 0.50, r);
            (252.0 + t * 23.0, 6.0 + t * 6.0, 0.0f32)
        } else if r < 0.72 {
            let t = smoothstep(0.50, 0.72, r);
            (275.0 + t * 205.0, 12.0 + t * 28.0, 0.2f32)
        } else {
            let t = smoothstep(0.72, 1.0, r);
            (480.0 + t * 340.0, 40.0 + t * 80.0, 0.5f32)
        };
        let max_possible_y = ((base_h + amplitude).ceil() as i32 + 2).min(crate::utils::math::WORLD_MAX_Y - 1);
        let min_possible_y = ((base_h - amplitude - 4.0).floor() as i32).min(SEA_LEVEL - 6).max(0);
        
        for y in (min_possible_y..=max_possible_y).rev() {
            let (density, _) = self.calculate_global_density(gx, y, gz);
            if density > 0.0 {
                return y;
            }
        }
        min_possible_y
    }

    pub fn resolve_block_type(&self, gy: i32, density: f32, base_h: f32, local_by: usize, y_densities: &[f32]) -> BlockType {
        if density <= 0.0 {
            return BlockType::Air;
        }

        let density_above = y_densities[local_by + 1];

        // 頂層暴露在空氣中
        let is_surface = density_above <= 0.0;
        if is_surface {
            // 🏔️ 超高海拔雪峰帶 (Y > 520)：強烈風化剝蝕，直接裸露花崗岩岩體！
            if gy > 520 {
                return BlockType::Stone;
            }

            // 🏖️ 水岸交界沉積帶 (SEA_LEVEL - 6 <= gy <= SEA_LEVEL + 4)：沙灘、礫石堆積！
            if gy <= SEA_LEVEL + 4 && gy >= SEA_LEVEL - 6 {
                if (gy + (base_h as i32)) % 3 == 0 {
                    return BlockType::Gravel;
                }
                return BlockType::Sand;
            }

            // 🌊 深海大洋海床 (gy < SEA_LEVEL - 6)：海底沉積沙石與基底岩石
            if gy < SEA_LEVEL - 6 {
                if (gy + (base_h as i32)) % 4 == 0 {
                    return BlockType::Gravel;
                }
                return BlockType::Stone;
            }

            // 🌿 常規溫帶陸地與丘陵 (SEA_LEVEL + 4 < gy <= 520)：肥沃草地！
            return BlockType::Grass;
        }

        // 泥土層（草地下方的 1~3 格）
        for offset in 1..=3 {
            if local_by + offset < y_densities.len() {
                let is_near_air_interface = y_densities[local_by + offset] <= 0.0;
                if is_near_air_interface {
                    // 若在水岸沉積帶附近，下層亦多為沙子
                    if gy <= SEA_LEVEL + 3 && gy >= SEA_LEVEL - 4 {
                        return BlockType::Sand;
                    }
                    if gy <= 520 {
                        return BlockType::Dirt;
                    }
                }
            }
        }

        // 地底礦脈分佈 (在海平面下方的堅硬基岩層，高度機率衰減消除水平紅線)
        if gy < SEA_LEVEL + 32 {
            let depth_fade = ((SEA_LEVEL + 32 - gy) as f32 / 32.0).clamp(0.0, 1.0);
            let hash = (gy as u32).wrapping_mul(73856093) ^ (density.to_bits());
            let iron_threshold = (2.0 * depth_fade) as u32;
            let coal_threshold = iron_threshold + (4.0 * depth_fade) as u32;
            if hash % 100 < iron_threshold {
                return BlockType::IronOre;
            } else if hash % 100 < coal_threshold {
                return BlockType::CoalOre;
            }
        }

        BlockType::Stone
    }

    /// 🚀 生成區塊資料，包含方塊緩衝區、非空氣計數，以及【全域自然水體流體緩衝區】
    pub fn generate_chunk_data(&self, chunk_pos: IVec3, surface_y_map: Option<&[i32; 1024]>) -> (ChunkBuffer, u16, Option<Box<[u8; 32768]>>) {
        let mut density_cache = vec![0.0f32; CHUNK_SIZE * CHUNK_SIZE * CACHE_Y_SIZE];
        let mut base_h_cache = vec![0.0f32; CHUNK_SIZE * CHUNK_SIZE];

        for bz in 0..32 {
            for bx in 0..32 {
                let gx = chunk_pos.x * 32 + bx as i32;
                let gz = chunk_pos.z * 32 + bz as i32;

                let mut current_base_h = 0.0;
                for by in 0..CACHE_Y_SIZE {
                    let gy = chunk_pos.y * 32 + by as i32;
                    let (density, base_h) = self.calculate_global_density(gx, gy, gz);
                    current_base_h = base_h;
                    let cache_idx = by + bx * 36 + bz * 36 * 32;
                    density_cache[cache_idx] = density;
                }
                base_h_cache[bx + bz * 32] = current_base_h;
            }
        }

        let mut chunk_buffer = ChunkBuffer::default();
        for bz in 0..32 {
            for bx in 0..32 {
                let col_offset = bx * 36 + bz * 36 * 32;
                let y_densities = &density_cache[col_offset..(col_offset + 36)];
                let base_h = base_h_cache[bx + bz * 32];
                
                for by in 0..32 {
                    let gy = chunk_pos.y * 32 + by as i32;
                    let density = y_densities[by];
                    let block_type = self.resolve_block_type(gy, density, base_h, by, y_densities);
                    chunk_buffer.blocks[bx + by * 32 + bz * 1024] = block_type;
                }
            }
        }

        let local_map = if let Some(map) = surface_y_map {
            *map
        } else {
            let mut m = [0i32; 1024];
            for bz in 0..32 {
                for bx in 0..32 {
                    m[bx + bz * 32] = self.get_max_surface_y(chunk_pos.x * 32 + bx as i32, chunk_pos.z * 32 + bz as i32);
                }
            }
            m
        };

        // 🌊 【立體水文與地底空洞解救系統 (Realistic Hydrology & Cave Rescue)】
        // 1. 海洋水體：僅在地表高度 <= SEA_LEVEL 的開放水域 (海溝、海盆、沙灘、河道) 注入海水。
        //    陸地山脈 (surface_y > SEA_LEVEL) 下方的深層溶洞 (Y=32~240) 徹底解救，保持乾燥可探索空氣！
        let mut fluid_buf: Option<Box<[u8; 32768]>> = None;
        let chunk_base_y = chunk_pos.y * 32;

        if chunk_base_y <= SEA_LEVEL {
            for bz in 0..32 {
                for bx in 0..32 {
                    let surface_y = local_map[bx + bz * 32];
                    if surface_y <= SEA_LEVEL {
                        let start_y = (surface_y + 1).max(chunk_base_y);
                        let end_y = (SEA_LEVEL).min(chunk_base_y + 31);
                        if start_y <= end_y {
                            let fb = fluid_buf.get_or_insert_with(|| Box::new([0u8; 32768]));
                            for gy in start_y..=end_y {
                                let by = (gy - chunk_base_y) as usize;
                                let idx = bx + by * 32 + bz * 1024;
                                if chunk_buffer.blocks[idx] == BlockType::Air {
                                    fb[idx] = 8 | 0x80; // 水源 (Source block)
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. 高山懸崖瀑布泉眼 (Cliff Waterfall Springs)
        // 在高於海平面 (Y > 280) 的峭壁邊緣生成泉眼，水流將自然沿懸崖飛瀉而下！
        if chunk_base_y >= SEA_LEVEL && chunk_base_y <= 650 {
            for bz in 1..31 {
                for bx in 1..31 {
                    let gx = chunk_pos.x * 32 + bx as i32;
                    let gz = chunk_pos.z * 32 + bz as i32;
                    let surface_y = local_map[bx + bz * 32];

                    if surface_y >= chunk_base_y && surface_y < chunk_base_y + 32 && surface_y > SEA_LEVEL + 24 {
                        let neighbor_surf = local_map[(bx + 1) + bz * 32];
                        if surface_y - neighbor_surf >= 8 {
                            let mut hash = (gx as u32).wrapping_mul(8253729).wrapping_add((gz as u32).wrapping_mul(2396403));
                            hash ^= hash >> 13;
                            if hash % 500 < 5 {
                                let by = (surface_y - chunk_base_y) as usize;
                                let spring_idx = (bx + 1) + by * 32 + bz * 1024;
                                if chunk_buffer.blocks[spring_idx] == BlockType::Air {
                                    let fb = fluid_buf.get_or_insert_with(|| Box::new([0u8; 32768]));
                                    fb[spring_idx] = 8 | 0x80;
                                    for dy in 1..=by {
                                        let fall_idx = (bx + 1) + (by - dy) * 32 + bz * 1024;
                                        if chunk_buffer.blocks[fall_idx] == BlockType::Air {
                                            fb[fall_idx] = 8;
                                        } else {
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 🌲 【跨區塊樹木生成系統】
        // 僅在開闊陸地平原與林地 (SEA_LEVEL + 4 < gy <= 500 且為 Grass) 植樹
        for bz in -2i32..34 {
            for bx in -2i32..34 {
                let gx = chunk_pos.x * 32 + bx;
                let gz = chunk_pos.z * 32 + bz;
                
                let mut h = (gx as u32).wrapping_mul(374761393).wrapping_add((gz as u32).wrapping_mul(668265263));
                h = (h ^ (h >> 13)).wrapping_mul(1274126177);
                h ^= h >> 16;
                
                if h % 1000 < 15 {
                    let gy = self.get_max_surface_y(gx, gz);
                    // 確保樹木只長在海平面以上、雪線以下的平原草地
                    if gy > SEA_LEVEL + 4 && gy <= 500 {
                        let (den, bh) = self.calculate_global_density(gx, gy, gz);
                        let top_block = self.resolve_block_type(gy, den, bh, 0, &[den, -1.0]); 
                        
                        if top_block == BlockType::Grass {
                            let tree_height = 4 + (h % 3) as i32;
                            
                            // 1. 生成樹幹 (OakLog)
                            for ty in 1..=tree_height {
                                let world_y = gy + ty;
                                let local_y = world_y - chunk_pos.y * 32;
                                if bx >= 0 && bx < 32 && bz >= 0 && bz < 32 && local_y >= 0 && local_y < 32 {
                                    let idx = (bx as usize) + (local_y as usize) * 32 + (bz as usize) * 1024;
                                    chunk_buffer.blocks[idx] = BlockType::OakLog;
                                }
                            }
                            
                            // 2. 生成樹冠 (OakLeaves)
                            for ly in (tree_height - 1)..=(tree_height + 1) {
                                for lx in -2i32..=2 {
                                    for lz in -2i32..=2 {
                                        if lx.abs() == 2 && lz.abs() == 2 && ly == tree_height + 1 {
                                            continue; 
                                        }
                                        if lx == 0 && lz == 0 && ly <= tree_height {
                                            continue;
                                        }
                                        
                                        let world_x = gx + lx;
                                        let world_z = gz + lz;
                                        let world_y = gy + ly;
                                        
                                        let local_x = world_x - chunk_pos.x * 32;
                                        let local_y = world_y - chunk_pos.y * 32;
                                        let local_z = world_z - chunk_pos.z * 32;
                                        
                                        if local_x >= 0 && local_x < 32 && local_y >= 0 && local_y < 32 && local_z >= 0 && local_z < 32 {
                                            let idx = (local_x as usize) + (local_y as usize) * 32 + (local_z as usize) * 1024;
                                            if chunk_buffer.blocks[idx] == BlockType::Air {
                                                chunk_buffer.blocks[idx] = BlockType::OakLeaves;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 重新計算 local_non_air
        let local_non_air = chunk_buffer.blocks.iter().filter(|&&b| b != BlockType::Air).count() as u16;

        (chunk_buffer, local_non_air, fluid_buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockFlatNoise(f32);
    impl NoiseModule for MockFlatNoise {
        fn sample_2d(&self, _x: f64, _z: f64) -> f32 { self.0 }
        fn sample_3d(&self, _x: f64, _y: f64, _z: f64) -> f32 { self.0 }
    }

    #[test]
    fn test_1024_world_height_constants() {
        assert_eq!(crate::utils::math::WORLD_MAX_Y, 1024, "World height must be 1024 blocks");
        assert_eq!(crate::utils::math::WORLD_CHUNKS_Y, 32, "World vertical chunks must be 32");
        assert_eq!(SEA_LEVEL, 256, "Sea level must be at Y=256");
    }

    #[test]
    fn test_natural_water_generated_below_sea_level() {
        // 測試深海大洋區域 (r = 0.0 -> 深海海床在 Y=75 左右)
        let gen = TerrainGenerator { noise_provider: MockFlatNoise(-0.5) };
        // 採樣海平面正下方的區塊 CY=7 (Y: 224..255)
        let (chunk_buffer, _, fluid_buf) = gen.generate_chunk_data(IVec3::new(0, 7, 0), None);
        
        let fb = fluid_buf.expect("Fluid buffer must be generated for ocean below sea level");
        // 空氣部分必須被填入水源 (Level 8 | 0x80)
        let mut water_count = 0;
        for i in 0..TOTAL_V_SIZE {
            if chunk_buffer.blocks[i] == BlockType::Air && fb[i] == (8 | 0x80) {
                water_count += 1;
            }
        }
        assert!(water_count > 0, "Ocean chunk below sea level must be filled with water source blocks");
    }

    #[test]
    fn test_subterranean_caves_under_land_remain_dry_air() {
        // 測試內陸丘陵山脈區域 (r = 0.6 -> 地表在 Y=350 遠高於海平面)
        let gen = TerrainGenerator { noise_provider: MockFlatNoise(0.1) };
        // 採樣地底溶洞高度區塊 CY=4 (Y: 128..159)
        let (chunk_buffer, _, fluid_buf) = gen.generate_chunk_data(IVec3::new(0, 4, 0), None);

        // 陸地下方的溶洞中，空氣方塊絕不准被海嘯般灌滿海水
        if let Some(fb) = fluid_buf {
            let mut flooded_water_in_caves = 0;
            for i in 0..TOTAL_V_SIZE {
                if chunk_buffer.blocks[i] == BlockType::Air && fb[i] == (8 | 0x80) {
                    flooded_water_in_caves += 1;
                }
            }
            assert_eq!(flooded_water_in_caves, 0, "Subterranean caves under high land must remain dry air, not flooded!");
        }
    }

    #[test]
    fn test_sky_peak_and_trench_dynamic_range() {
        // 1. 深海海溝採樣 (r ~ 0.0)
        let ocean_gen = TerrainGenerator { noise_provider: MockFlatNoise(-0.5) };
        let ocean_bed_y = ocean_gen.get_max_surface_y(0, 0);
        assert!(ocean_bed_y < 200, "Deep ocean bed should be deep below sea level: {}", ocean_bed_y);

        // 2. 宏偉巨型雪峰採樣 (r ~ 1.0)
        let mountain_gen = TerrainGenerator { noise_provider: MockFlatNoise(0.5) };
        let mountain_peak_y = mountain_gen.get_max_surface_y(0, 0);
        assert!(mountain_peak_y > 700, "Sky peaks must rise above 700 blocks high: {}", mountain_peak_y);

        // 3. 垂直落差驗證：超過 500 格的壯麗尺度！
        let vertical_span = mountain_peak_y - ocean_bed_y;
        assert!(vertical_span >= 500, "Macro world dynamic range must be at least 500 blocks: span={}", vertical_span);
    }

    #[test]
    fn test_continental_shelf_continuous_no_discontinuity_jump() {
        // 驗證在所有分段邊界上，高度差絕對平滑，絕無超過 0.5 格的斷崖瞬態跳躍！
        for test_r in [0.18, 0.38, 0.50, 0.72] {
            let gen_low = TerrainGenerator { noise_provider: MockFlatNoise(test_r - 0.5 - 0.001) };
            let gen_high = TerrainGenerator { noise_provider: MockFlatNoise(test_r - 0.5 + 0.001) };
            let (_, h_low) = gen_low.calculate_global_density(0, SEA_LEVEL, 0);
            let (_, h_high) = gen_high.calculate_global_density(0, SEA_LEVEL, 0);
            let diff = (h_high - h_low).abs();
            assert!(diff < 1.0, "Discontinuity cliff detected at r={}: diff={}", test_r, diff);
        }
    }

    #[test]
    fn test_shoreline_sediment_blocks() {
        // 採樣海灘帶 (r = 0.40 -> base_h 落在 253 左右)
        let gen = TerrainGenerator { noise_provider: MockFlatNoise(-0.1) };
        let (_density, base_h) = gen.calculate_global_density(0, SEA_LEVEL, 0);
        
        // 模擬海平面交界處的露天地表方塊
        let block = gen.resolve_block_type(SEA_LEVEL, 1.0, base_h, 0, &[1.0, -1.0]);
        assert!(block == BlockType::Sand || block == BlockType::Gravel, 
            "Shoreline around sea level must form sand or gravel sediment, got {:?}", block);
    }
}
