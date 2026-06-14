use bevy::prelude::IVec3;
use bevy::tasks::IoTaskPool;
use std::fs::File;
use std::io::{Read, Write};
use super::chunk::ChunkData;

pub fn load_chunk_from_disk(pos: IVec3) -> Option<ChunkData> {
    let path = format!("saves/chunk_{}_{}_{}.bin", pos.x, pos.y, pos.z);
    if let Ok(mut file) = File::open(&path) {
        let mut buffer = Vec::new();
        if file.read_to_end(&mut buffer).is_ok() {
            if let Ok(data) = bincode::deserialize(&buffer) {
                return Some(data);
            }
        }
    }
    None
}

pub fn save_chunk_to_disk(pos: IVec3, data: ChunkData) {
    let path = format!("saves/chunk_{}_{}_{}.bin", pos.x, pos.y, pos.z);
    IoTaskPool::get().spawn(async move {
        if let Ok(encoded) = bincode::serialize(&data) {
            if let Ok(mut file) = File::create(&path) {
                let _ = file.write_all(&encoded);
            }
        }
    }).detach();
}
