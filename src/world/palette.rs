use super::voxel::BlockType;
use crate::utils::math::CHUNK_VOLUME;

use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Palette {
    pub palette: Vec<BlockType>,
    pub data: Vec<u64>,
    pub bits_per_block: u8,
    pub mask: u64,
}

impl Palette {
    pub fn new() -> Self {
        Self {
            palette: vec![BlockType::Air],
            data: vec![0; (CHUNK_VOLUME + 63) / 64], // 1 bit per block minimum
            bits_per_block: 1,
            mask: 1,
        }
    }

    pub fn get(&self, index: usize) -> BlockType {
        let bit_index = index * (self.bits_per_block as usize);
        let u64_index = bit_index / 64;
        let bit_offset = bit_index % 64;
        
        let mut pal_idx = (self.data[u64_index] >> bit_offset) & self.mask;
        
        // Handle cross-boundary
        let bits_left = 64 - bit_offset;
        if bits_left < (self.bits_per_block as usize) {
            let remaining_bits = (self.bits_per_block as usize) - bits_left;
            pal_idx |= (self.data[u64_index + 1] & ((1 << remaining_bits) - 1)) << bits_left;
        }

        self.palette[pal_idx as usize]
    }

    pub fn set(&mut self, index: usize, block: BlockType) {
        let mut palette_idx = None;
        for (i, &b) in self.palette.iter().enumerate() {
            if b == block {
                palette_idx = Some(i as u64);
                break;
            }
        }

        let palette_idx = palette_idx.unwrap_or_else(|| {
            let i = self.palette.len() as u64;
            self.palette.push(block);
            
            // Check if we need to resize bit packing
            let required_bits = (64 - i.leading_zeros()) as u8;
            if required_bits > self.bits_per_block {
                self.resize_bits(required_bits);
            }
            i
        });

        self.write_index(index, palette_idx);
    }

    fn write_index(&mut self, index: usize, pal_idx: u64) {
        let bit_index = index * (self.bits_per_block as usize);
        let u64_index = bit_index / 64;
        let bit_offset = bit_index % 64;

        // Clear existing bits
        let clear_mask = !(self.mask << bit_offset);
        self.data[u64_index] &= clear_mask;
        
        // Write new bits
        self.data[u64_index] |= pal_idx << bit_offset;

        // Cross boundary
        let bits_left = 64 - bit_offset;
        if bits_left < (self.bits_per_block as usize) {
            let remaining_bits = (self.bits_per_block as usize) - bits_left;
            let clear_mask_next = !((1 << remaining_bits) - 1);
            self.data[u64_index + 1] &= clear_mask_next;
            self.data[u64_index + 1] |= pal_idx >> bits_left;
        }
    }

    fn resize_bits(&mut self, new_bits: u8) {
        let old_bits = self.bits_per_block;
        let old_data = std::mem::take(&mut self.data);
        
        self.bits_per_block = new_bits;
        self.mask = (1 << new_bits) - 1;
        
        let total_bits = CHUNK_VOLUME * (new_bits as usize);
        self.data = vec![0; (total_bits + 63) / 64];

        // Re-pack
        for i in 0..CHUNK_VOLUME {
            let bit_index = i * (old_bits as usize);
            let u64_index = bit_index / 64;
            let bit_offset = bit_index % 64;
            
            let mut pal_idx = (old_data[u64_index] >> bit_offset) & ((1 << old_bits) - 1);
            let bits_left = 64 - bit_offset;
            if bits_left < (old_bits as usize) {
                let remaining_bits = (old_bits as usize) - bits_left;
                pal_idx |= (old_data[u64_index + 1] & ((1 << remaining_bits) - 1)) << bits_left;
            }
            self.write_index(i, pal_idx);
        }
    }
}
