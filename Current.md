## 1. Cavegame Current Architecture Ground Truth本檔案為 Cavegame 體素引擎的當前最新完全體架構藍圖。此文件做為 AI Agent (Google Antigravity) 的最高指導原則。在進行任何代碼編寫、重構或除錯前，必須嚴格遵循本文件定義之資料結構、渲染管線與物理規則，嚴禁退回舊版不相容之架構。
## 1. ⚙️ 開發核心與模組結構
**開發語言**：Rust (Stable 分支，天生具備無垃圾回收 GC-free 特性，精確控制內存生命週期)。
**遊戲引擎**：Bevy 0.14 (基於高效能多執行緒排程的 ECS 實體組件系統)。
**圖形 API**：wgpu (低階圖形驅動，Windows 平台自動掛載 Vulkan 或 DirectX 12 核心)。

### 📂 目錄結構與分工
```text
src/
├── main.rs         (狀態機初始化與全域 Plugin 註冊)
├── phys/           (一級物理模組，專職處理 AABB 空間幾何與分軸碰撞消解)
├── player/         (玩家控制器、滑鼠第一人稱視角、動態破壞/放置方塊互動)
├── render/         (核心渲染管線：greedy.rs 貪婪網格、textures.rs 材質載入)
├── ui/             (hud.rs 準星、debug.rs F3 定時除錯疊加層、預留主選單介面)
└── world/          (核心世界資料結構)
    ├── storage.rs  (體素調色盤、Serde/Bincode 非同步硬碟讀寫)
    ├── gen/        (地形生成演算法：flat.rs 超平坦、perlin.rs 3D 密度雜訊預留)
    └── mod.rs      (區塊 3D 動態加載/卸載生命週期調度)
```

## 2. 🎨 進階圖形渲染與材質管線
為了將硬體算力榨乾，專案棄用傳統低效渲染，全面攻頂現代體素優化技術：

* **單一區塊單一網格 (1 Draw Call per Chunk)**：所有方塊材質在區塊網格化時融合成同一個物理 Mesh 實體，透過自訂頂點屬性 `Mesh::ATTRIBUTE_TEXTURE_INDEX` 傳遞材質 Layer ID。
* **貪婪網格化演算法 (Greedy Meshing)**：掃描 3D 空間切面，自動剔除被實心方塊阻擋的內部面。相鄰且屬性相同的面融合成巨大的矩形多邊形（Quad），頂點數暴減 80%~99%。
* **ECS 安全的兩階段分離渲染 (Two-Pass Iteration)**：網格生成分為兩階段運作。第一階段收集所有需要更新的 Entity 及其周遭狀態，繞過 Mutex 借用衝突；第二階段統一網格化建構，徹底符合 Bevy ECS 安全規範。
* **正統父子場景層級 (Transform Hierarchies)**：
  * **父實體 (Chunk Entity)**：對於非空氣區塊，統一強制掛載 `SpatialBundle`，其世界座標精確鎖定為 `transform: chunk_pos * CHUNK_SIZE`。純空氣區塊則**完全不產生**實體，達成零 Transform 開銷。
  * **子實體 (Mesh Child)**：視覺網格實體作為父實體的子節點，其 `Transform` 保持 `default()`。頂點相對座標嚴格維持在 `0..32` 的局部空間內，最終位置由 Bevy 自動透過 `chunk_pos * 32 + vertex(0..32)` 公式傳播。
* **跨區界幾何擁有權短路 (Geometry Ownership Short-Circuit)**：在 Greedy Meshing 進行鄰居探測時，嚴格實施索引判定（例如 `slice >= 0`）。只有當實心方塊真正屬於「當前區塊的合法範圍」時，才允許產生網格面。這徹底防止了相鄰區塊雙向重複繪製同一邊界，消滅了交界處的雙重疊加與幽靈夾層面。
* **跨區塊全域探測與聯動更新 (Global Check & Remesh Propagation)**：
  * **動態破壞/放置**：系統依賴絕對座標全域查詢 `world.get_block_global`，當玩家在區塊交界處放置或破壞方塊時，透過 6 向邊界偵測自動將相鄰區塊標記為 Dirty，確保跨區塊接縫的 Face Culling 即時無縫重算。
  * **加載期連動 (Race Condition 修正)**：當全新的區塊完成非同步生成並插入 `WorldManager` 的瞬間，主執行緒會立即主動探測 6 個相鄰軸向的舊區塊。若存在則強制將其標記為 `is_dirty = true`，強迫舊區塊重新網格化並剔除過期的邊界殘留牆，達成無瑕疵的地形接縫。
* **2D 紋理陣列與自訂 WGSL 著色器**：採用 `D2Array` 管理材質層級，配合動態 UV 縮放防拉伸溢色。手寫 `voxel.wgsl` 並實作 Bevy Material 接口，解決 Bind Group 衝突，確保 Wgpu Validation 零錯誤。
* **幾何繞向完美對齊**：6 個面的頂點嚴格遵循 CCW 繞向，所有軸向遵循 `rev = normal < 0` 統一規則。

## 3. 🧱 物理碰撞與玩家移動系統
* **座標慣例**：嚴格維持 Bevy 預設的 Y-up 座標系（X, Z 為水平，Y 為高度）。
* **離散軸向分離碰撞**：玩家速度向量在每影格拆解為嚴格獨立的三階段結算（X 軸位移 → Z 軸位移 → Y 軸位移）。撞擊方塊被水平彈回時，強迫維持 0.001 格安全距離外推，根除 False Grounding 浮點數微觀滲透 Bug。
* **玩家 AABB 尺寸與速度**：水平半徑 0.3。站立高度 1.8 格，蹲下（按住 左 Ctrl）動態縮小為 1.2 格。
* **安全防禦機制與旁觀者模式**：
  * **第一幀地形安全鎖**：玩家腳下的區塊若尚未完成非同步載入，系統會強制凍結重力與位移，防止墜入虛空。
  * **反卡死救援**：玩家無法在自己身體 AABB 內放置新方塊；若因地形意外卡入實心方塊，系統會在單影格內自動將玩家向上溫和頂開救援。
  * **F4 旁觀者穿牆模式 (Spectator Mode)**：透過 F4 鍵可即時切換旁觀者模式。此模式在物理引擎最頂端執行短路接管，無視地形安全鎖、AABB 碰撞與重力牽引，允許玩家透過 W/A/S/D 與 Space/Shift 進行絕對 3D 自由飛行，速度可透過 Ctrl 疾跑翻倍。

## 4. 🌍 動態 3D 無限世界與持久化存檔
* **空間結構與全域高度換算**：以 32x32x32 的方塊組成 Chunk。全域使用 HashMap 進行 3D 區塊 `IVec3::new(cx, cy, cz)` 管理。地形生成算法（如 Perlin Noise）嚴格使用 `global_y = chunk_pos.y * 32 + local_y` 的全域絕對高度對齊地層，避免地貌在不同垂直區塊被重複複製。
* **垂直 3D 動態加載 (3D Render Distance)**：每影格以玩家為中心，向外加載 5x5 的水平範圍，並且垂直覆蓋 4 層區塊（CY = 0 到 3，對應高度 0 到 128）。當超出卸載距離或垂直邊界時觸發 GPU 網格實體銷毀與內存回收。
* **非同步環形螺旋加載管線 (Async Ring-Sorted Loading Pipeline)**：放棄傳統同步巢狀迴圈，改採 3D 距離平方進行排序（`diff.x^2 + diff.y^2 + diff.z^2`），確保玩家腳下與視角前方的區塊享有最高加載權重。主執行緒透過 `.take(4)` 每影格限流派發最多 4 個任務，由背景的 `AsyncComputeTaskPool` 執行 Perlin Noise 或硬碟讀取，徹底杜絕 CPU 瞬間負載超載所引發的 Stuttering（掉幀）。
* **資料與實體徹底解耦 (Data & Entity Decoupling)**：純空氣區塊僅作為包含調色盤的資料 `ChunkEntry` 留在 HashMap 中，**不佔用任何 Bevy ECS 實體**。當玩家在純空區塊放置第一顆實心方塊時，才會觸發**延遲生成 (Lazy Spawning)** 動態建立網格實體。
* **體素調色盤與完美存檔防禦 (Save Bloating Defense & Perfect Unload)**：使用 Palette Compression 將資料壓縮，髒區塊卸載時交由 IoTaskPool 異步寫入硬碟。若判定該區塊已被挖空退化為「純空氣」（透過嚴格的 32,768 體素遍歷檢查 `is_pure_air` 排除調色盤殘留），系統不僅拒絕產生新存檔，還會在背景自動刪除硬碟上的歷史殘留檔案。同時，卸載輪詢具備**完美閉環**：只要超出渲染距離，無論區塊是否修改過，系統必定無條件執行 `despawn_recursive` 銷毀視覺實體並從 HashMap 移除，從根源杜絕存檔無限膨脹與記憶體釘子戶。

## 5. 🎛️ 遊戲狀態機與 UI/Debug 系統
* **GameState 狀態控制**：切分 `MainMenu`、`InGame` 等狀態，退回選單時背景世界與物理會凍結。
* **F3 Debug HUD 異步更新分流**：使用 `Timer::from_seconds(0.5)` 控制 F3 疊加層上的 FPS 與 Frame Time 刷新頻率，解決文字因幀率震盪而閃爍看不清的問題；而玩家座標等空間數據則無延遲即時更新。
* **F3 + C 區塊邊界檢視 (Chunk Borders)**：實作了致敬 Minecraft 的除錯快捷鍵。在 F3 開啟狀態下按下 C 鍵，可透過獨立且零干擾的 `Bevy Gizmos` 系統，精準繪製出對齊世界座標的 32x32x32 黑色區塊邊界，大幅協助空間除錯。