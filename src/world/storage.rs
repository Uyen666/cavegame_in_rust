use bevy::prelude::IVec3;
use bevy::tasks::IoTaskPool;
use std::fs::File;
use std::io::{Read, Write};
use super::chunk::ChunkData;
use super::voxel::BlockType;

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

/// 將區塊寫入硬碟，或當區塊是純空氣時清理舊存檔。
/// 全部操作在 IoTaskPool 背景執行緒執行，絕不阻塞主執行緒。
pub fn save_chunk_to_disk(pos: IVec3, data: ChunkData) {
    let path = format!("saves/chunk_{}_{}_{}.bin", pos.x, pos.y, pos.z);

    // 純空氣判定：改用嚴格的 3D 體素遍歷檢查，確保被挖空的區塊也能被正確攔截
    let is_pure_air = data.palette.is_pure_air();

    IoTaskPool::get().spawn(async move {
        if is_pure_air {
            // 🚀 純空氣區塊：拒絕寫入新存檔，並清理硬碟上的舊存檔（若存在）
            // 防止 Save Bloating，同時杜絕下次載入時「歷史幽靈復原」
            let p = std::path::Path::new(&path);
            if p.exists() {
                let _ = std::fs::remove_file(p);
            }
        } else {
            // 非空氣區塊：正常序列化寫入
            if let Ok(encoded) = bincode::serialize(&data) {
                if let Ok(mut file) = File::create(&path) {
                    let _ = file.write_all(&encoded);
                }
            }
        }
    }).detach();
}
