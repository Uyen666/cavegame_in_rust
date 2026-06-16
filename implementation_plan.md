# 一維無狀態地形生成器核心管線對齊 (Pipeline Plumbing)

這項重構將徹底汰換舊有的 `Palette` 調色盤壓縮結構，全面切換至 `ChunkBuffer` 扁平化陣列，以完全發揮 `TerrainGenerator` 的 O(1) 記憶體效能。這將牽動儲存系統 (`storage.rs`)、資料結構 (`chunk.rs`)、生成系統 (`mod.rs`) 與網格產生器 (`greedy.rs`)。

## User Review Required

> [!WARNING]
> **儲存系統與 Serialization (存檔相容性)**
> `ChunkBuffer` 使用 `[BlockType; 32768]` 原生陣列。然而標準的 `serde` 預設並不直接支援大於 32 長度的原生陣列 `Serialize/Deserialize` 衍生。為了維持您指定的 `[BlockType; TOTAL_V_SIZE]` 結構並保證磁碟存寫順利，我將在 `generator.rs` 為 `ChunkBuffer` 實作自訂的 `Serialize` 與 `Deserialize`，將底層記憶體視為切片進行序列化。這將導致舊存檔失效（由於架構已徹底革新，舊存檔無法也不該被相容）。

> [!IMPORTANT]
> **純空氣判定 (is_pure_air)**
> 舊版的 `Palette::is_pure_air()` 將被移除。我會在 `ChunkBuffer` 中新增一個 `is_pure_air(&self)` 的方法，透過高效的迭代器 `.iter().all(|&b| b == BlockType::Air)` 來進行 32768 次 O(N) 掃描，因為記憶體連續，這項檢查在現代 CPU 上仍然極快。

## Proposed Changes

### 核心模組

#### [MODIFY] [generator.rs](file:///c:/Users/rock9/Desktop/RUST/Cavegame/src/world/generator.rs)
- 移除頂部的 `#![allow(dead_code)]`。
- 幫 `BlockType` 加上 `Serialize, Deserialize` 衍生巨集。
- 幫 `ChunkBuffer` 手動實作 `Serialize` 與 `Deserialize`，並新增 `is_pure_air(&self) -> bool` 方法。

#### [MODIFY] [chunk.rs](file:///c:/Users/rock9/Desktop/RUST/Cavegame/src/world/chunk.rs)
- 移除所有 `Palette` 的 Import 與依賴。
- `ChunkData` 與 `Chunk` 結構體內的 `palette` 欄位全面替換為 `pub buffer: ChunkBuffer`。
- `Chunk::new` 初始化時改用 `ChunkBuffer::default()`。
- `Chunk::get_block` 與 `Chunk::set_block` 徹底重構，直接套用一維步長：`self.buffer.blocks[x + y * 32 + z * 1024]`。

#### [MODIFY] [storage.rs](file:///c:/Users/rock9/Desktop/RUST/Cavegame/src/world/storage.rs)
- 將存檔檢查的 `data.palette.is_pure_air()` 更新為 `data.buffer.is_pure_air()`。

#### [MODIFY] [mod.rs](file:///c:/Users/rock9/Desktop/RUST/Cavegame/src/world/mod.rs)
- 將所有 `ChunkEntry` 內的 `palette` 替換為 `buffer`。
- 更新 `get_block_global` 邏輯，改用 `entry.buffer.blocks[idx]` 來讀取方塊。
- 升級 `AsyncComputeTaskPool` 內的生成邏輯，實例化 `TerrainGenerator` 並且調用 `generate_chunk_data` 產出 `ChunkBuffer` 傳回主執行緒。

#### [MODIFY] [greedy.rs](file:///c:/Users/rock9/Desktop/RUST/Cavegame/src/render/greedy.rs)
- 確認 `CHUNK_SIZE as i32` 的邊界型別轉換正確。
- 確認最內層迴圈使用 `current_chunk.get_block(lx[0] as usize, lx[1] as usize, lx[2] as usize)` 高速讀取，並且已抽離實體借用 `current_chunk` 至外部。

## Verification Plan
1. `cargo check` 確保所有檔案 0 錯誤、0 警告。
2. 啟動遊戲後，確認新的 `TerrainGenerator` 生成出平滑、無懸崖長草的隨機地形。
3. 玩家離開某個區塊時，硬碟 `saves/` 依然能夠正常序列化並儲存 `ChunkBuffer`，純空氣區塊也能被正確偵測與拋棄。
