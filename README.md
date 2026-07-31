# ⛏️ CaveGame (體素遊戲引擎)

[![Rust](https://img.shields.io/badge/Language-Rust_2021-orange.svg?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![Bevy](https://img.shields.io/badge/Engine-Bevy_0.14-blue.svg?style=flat-square)](https://bevyengine.org/)
[![Graphics](https://img.shields.io/badge/API-wgpu_(Vulkan%2FDirectX12)-red.svg?style=flat-square)](https://wgpu.rs/)
[![Architecture](https://img.shields.io/badge/Architecture-ECS-brightgreen.svg?style=flat-square)]()

**CaveGame** 是一款基於 **Rust** 語言與 **Bevy 0.14** ECS 遊戲引擎開發的高效能 3D 體素（Voxel）遊戲引擎與開放世界開發專案。

本專案旨在榨乾現代硬體效能，全盤棄用傳統低效體素渲染與離散碰撞，採用 **無垃圾回收 (GC-Free)** 機制、**工業級梯度貪婪網格 (AO-Aware Gradient Greedy Meshing)**、** Swept AABB 連續碰撞消解 (CCD)**、**3D 環形非同步區塊加載管線** 以及 **智慧動態流體物理**，打造媲美原版 Minecraft 且極致絲滑的 3D 體素世界。

---

## 📌 目錄 (Table of Contents)
- [核心技術亮點](#-核心技術亮點-key-features)
- [系統架構與目錄結構](#-系統架構與目錄結構-system-architecture)
- [遊戲控制與操作說明](#-遊戲控制與操作說明-controls--usage)
- [快速開始與建置指南](#-快速開始與建置指南-getting-started)
- [核心模組技術細節](#-核心模組技術細節-technical-deep-dive)
- [🚀 未來發展規劃 (Roadmap)](#-未來發展規劃-future-roadmap)
- [📄 技術文件導覽](#-技術文件導覽-documentation)

---

## ⚡ 核心技術亮點 (Key Features)

### 🎨 1. 現代極致圖形渲染管線 (Modern Voxel Graphics Engine)
- **單一區塊單一 Draw Call (1 Draw Call per Chunk)**：所有方塊材質在區塊網格化時融合為單一 Mesh。
- **AO 輔助型梯度貪婪網格化 (AO-Aware Gradient Greedy Meshing)**：自動合併相同屬性與平滑雙線型光照梯度的相鄰多邊形，頂點數暴減 **80% ~ 99%**。
- **32-Bit 頂點位元壓縮 (Vertex Bit Packing)**：將 `x, y, z, face_id, tex_layer` 完美壓縮進單一 `u32` 插槽，降低內存與顯存頻寬消耗。
- **程序化 WGSL 著色器與 Quad Edge Padding**：在 GPU 頂點著色器實施 `0.0005` 格邊緣微外推，徹底絕殺 T-Junction 引發的邊界縫隙與閃光漏光。
- **3D 離屏渲染快捷列 UI**：使用 9 組獨立 3D 相機與 `VoxelMaterial` 渲染正宗 Minecraft 風格的多重貼圖 3D 立體方塊（草方塊具備頂面、側面、底面不同貼圖）。

### 🧱 2. 高精度 Swept AABB 物理與運動學 (Continuous Collision & Physics)
- ** Swept AABB 掃掠碰撞 (CCD)**：預先投射速度向量計算碰撞確切時間點 (Time of Impact)，徹底解決高速運動下的穿模（Tunneling）問題。
- **離散軸向獨立解算 (Axis-Separated Resolution)**：獨立進行 **X → Z → Y** 軸碰撞與位移，提供極致絲滑的「貼牆滑行 (Wall Sliding)」手感。
- **Jitter-Free 相機與進階動作**：在 `FixedUpdate` 剛性同步旋轉視角與位移；支援 **蹲下 (1.2m 蹲姿)**、**Bunny Hopping 陸地連跳** 與 **輸入緩衝**。
- **流體物理與海豚跳 (Fluid Buoyancy & Dolphin Jump)**：雙層腳底/頭部水體感知，支援水下阻力、浮力、水面爆發跳躍（海豚跳）與瀑布攀爬。

### 🌍 3. 3D 無限動態世界與手寫 RLE 存檔 (Infinite 3D World & Save System)
- **32x32x32 3D Chunk 網格**：全域使用 3D 座標 `IVec3(cx, cy, cz)` 動態加載與管理。
- **非同步環形螺旋加載管線 (Async Ring-Sorted Pipeline)**：基於 `AsyncComputeTaskPool` 進行多執行緒 Perlin Noise 地形雕刻與網格生成，零卡頓 (Stutter-Free)。
- **稀疏配置與真空剔除 (Sparse Chunk & Vacuum Culling)**：純空氣區塊完全不產生 Bevy ECS 實體，高空與深層無邊界區域記憶體零佔用。
- **RLE (Run-Length Encoding) 遊程編碼存檔**：手寫高效壓縮演算法，搭配背景 `IoTaskPool` 異步寫入硬碟，防止存檔無限膨脹。

### 🌊 4. 智慧動態流體物理與擴散 (Dynamic Fluid Simulation)
- **雙軌道貪婪網格化 (Dual-Pass Meshing)**：流體採用逐體素 2x2 全域角落動態內插，呈現連續光滑的流動水面斜面。
- **5-Block Lookahead 尋路大腦**：流體具備 5 格遠視懸崖感測能力，支援重力截斷、瀑布落水柱全滿特權與退潮動態重新平衡。
- **雙模態手持交互**：使用 **F 鍵** 進行收水（喚醒退潮連鎖）與放水（注入水源擴散）。

---

## 📂 系統架構與目錄結構 (System Architecture)

```text
CaveGame/
├── Cargo.toml          # Rust 專案與 Bevy / wgpu 依賴配置
├── Current.md          # 系統架構最高指導原則與技術真理藍圖
├── physic.md           # 物理引擎技術專題文檔
├── assets/             # 方塊貼圖 (D2Array) 與資產檔
└── src/
    ├── config.rs       # 全域遊戲常數與渲染視距配置
    ├── main.rs         # Bevy App 進入點、狀態機與 Plugin 註冊
    ├── phys/           # 物理碰撞模組
    │   └── swept.rs    # Swept AABB 連續碰撞與離散軸向位移消解
    ├── player/         # 玩家控制器
    │   └── mod.rs      # 第一人稱視角、第一幀安全傳送、AABB 姿態與快捷列互動
    ├── render/         # 核心圖形渲染管線
    │   ├── greedy.rs   # AO 輔助型梯度貪婪網格化演算法 (Async)
    │   ├── material.rs # VoxelMaterial 自訂 WGSL 材質管線
    │   ├── texture_array.rs # Texture2DArray 材質打包管理
    │   └── textures.rs # 資產載入器
    ├── ui/             # 使用者介面與 HUD
    │   ├── debug.rs    # F3 定時除錯疊加層、FPS、 Holding 與 F3+C 區塊邊界
    │   ├── hud.rs      # 螢幕底部 3D 離屏渲染 Hotbar HUD 與準星
    │   ├── main_menu.rs# 主選單介面預留
    │   └── settings.rs # 設定選單介面預留
    ├── utils/          # 工具模組 (座標轉換、數學常數)
    └── world/          # 體素世界核心數據結構與管理
        ├── chunk.rs    # 32x32x32 1D 扁平 ChunkBuffer
        ├── fluid.rs    # BFS 心跳流體擴散與動態平衡
        ├── generator.rs# 二階段無狀態地形生成器 (Fbm + Ridged Noise)
        ├── lighting.rs # 天空光與方塊光雙層 BFS 阻斷與蔓延
        ├── storage.rs  # RLE 存檔壓縮與異步硬碟持久化
        └── voxel.rs    # 方塊類型 BlockType 定義
```

---

## 🎮 遊戲控制與操作說明 (Controls & Usage)

| 按鍵 / 操作 | 功能說明 |
| :--- | :--- |
| **W / A / S / D** | 玩家前後左右移動 |
| **滑鼠移動** | 第一人稱視角旋轉 (FPS Camera) |
| **Space (空白鍵)** | 起跳 / 在水中向上游泳與攀瀑 / 旁觀者升空 |
| **Left Ctrl (左 Ctrl)** | 疾跑 / 蹲下 (AABB 高度縮至 1.2m) / 旁觀者降落 |
| **滑鼠左鍵** | 破壞目標方塊 (距離 5.0m Raycast) |
| **滑鼠右鍵** | 放置當前手持快捷列之方塊 |
| **F 鍵** | 流體雙模態交互 (瞄準水體收水退潮 / 瞄準空處注入水源) |
| **數字鍵 1 ~ 9** | 快速切換手持快捷列欄位 (1-9 Slot) |
| **滑鼠滾輪** | 帶有滑順能量池與防越界環形輪詢切換快捷列 |
| **F3** | 開啟 / 關閉 除錯 HUD (座標、FPS、當前手持方塊) |
| **F3 + C** | 開啟 / 關閉 32x32x32 3D 區塊邊界黑色 Wireframe |
| **F4** | 切換 **旁觀者飛行模式 (Spectator Flight Mode)** (無視碰撞穿牆) |
| **P 鍵** | 即時開關平滑光照 (Smooth Lighting & AO) 並重新烘焙網格 |

---

## 🛠️ 快速開始與建置指南 (Getting Started)

### 前置需求 (Prerequisites)
- [Rust Toolchain](https://www.rust-lang.org/) (建議 Stable 1.75 或更新版本，支援 2021 Edition)
- 支援 Vulkan、DirectX 12 或 Metal 的顯示卡與最新驅動程式

### 編譯與執行 (Build & Run)

1. **複製專案庫**：
   ```bash
   git clone https://github.com/Uyen666/cavegame_in_rust.git
   cd cavegame_in_rust
   ```

2. **開發模式開發與執行 (Debug Mode)**：
   ```bash
   cargo run
   ```
   *(註：專案已在 `Cargo.toml` 針對開發模式下的第三方依賴庫開檔 `opt-level = 3` 優化，保證 Debug 模式依然順暢)*

3. **發布模式極限編譯 (Release Mode)**：
   ```bash
   cargo run --release
   ```

4. **VS Code 開發者快速鍵**：
   - 本專案內建 `.vscode/settings.json` 與 `.vscode/tasks.json`，在 VS Code 中直接按下 **`Ctrl + Shift + B`** 即可一鍵編譯並啟動遊戲。

---

## 🔮 🚀 未來發展規劃 (Future Roadmap)

CaveGame 目前已完成核心引擎底層（渲染、物理、存檔、流體、UI），未來將持續擴充豐富的遊戲性與系統模組：

### 🧱 1. 多方塊與生態豐富化 (World & Biome Expansion)
- [ ] **豐富方塊種類**：新增原木、樹葉、沙子、礫石、礦石（煤、鐵、金、鑽石）與玻璃（半透明渲染支援）。
- [ ] **植物與植被生成器**：在地形生成器中整合樹木 (Trees)、花草、藤蔓的程序化結構生成。
- [ ] **多元生態系 (Biomes)**：擴充沙漠、雪原、深海、叢林與地下巨型溶洞特化生態系。

### 💡 2. 動態光源與晝夜交替系統 (Dynamic Lighting & Day/Night Cycle)
- [ ] **動態方塊光源**：實裝火把 (Torch)、岩漿 (Lava) 與螢光石，支援動態 0~15 級方塊光 BFS 蔓延與即時網格重烘焙。
- [ ] **晝夜交替與太陽月亮**：天空盒與太陽/月亮軌道旋轉，天空光強 (0~15) 隨時間動態漸變與平滑環境色過渡。
- [ ] **動態陰影 (Cascaded Shadow Maps)**：研究整合 `wgpu` 陰影貼圖與體素光照交織。

### 🛠️ 3. 道具背包、合成與裝備系統 (Inventory & Crafting)
- [ ] **E 鍵玩家背包介面**：實作 27 格儲物背包 UI，支援物品拖曳與數量堆疊。
- [ ] **工作台與合成九宮格 (Crafting System)**：經典 2x2 與 3x3 物品配方解算與工具製作。
- [ ] **方塊採掘等級與工具耐久度**：不同方塊需使用對應工具（鎬、斧、鏟）與採掘破壞動畫。

### 👾 4. 實體與 AI 生物系統 (Entities & Mob AI)
- [ ] **通用 Entity 物理框架**：將 Swept AABB 碰撞擴展支援至所有非玩家實體 (Dropped Items, Mobs)。
- [ ] **掉落物 (Dropped Items)**：破壞方塊後產生微型 3D 浮動方塊，具備旋轉與玩家磁吸拾取體驗。
- [ ] **生物與敵對 AI (Mobs & Pathfinder)**：實裝 3D 體素 A* 尋路算法，引進被動生物（豬、牛）與夜間敵對怪物。

### 🎵 5. 音效與環境音效引擎 (Audio & Ambience)
- [ ] **3D 空間音效**：整合 `bevy_audio` 或 `kira`，實裝基於距離與方向的 3D 腳步聲（區分草地、石頭、水聲）。
- [ ] **環境背景音樂 (BGM)**：洞穴深處與地表動態切換沉浸式背景音樂。

### 🌐 6. 多人在線連線機制 (Multiplayer Networking)
- [ ] **客戶端/伺服器架構 (C/S Networking)**：採用 `renet` 或 `quinn` (UDP/QUIC)，實現區塊同步與多玩家姿態插值。

---

## 📄 技術文件導覽 (Documentation)

詳細的架構說明與物理推導可參考專案內包含的技術文檔：
- 📘 [Current.md](file:///c:/Users/%E6%9E%97%E5%B0%9A%E6%A5%B7/Desktop/RUST/CaveGame/Current.md) — 體素引擎最新完全體架構藍圖與技術規範
- 📙 [physic.md](file:///c:/Users/%E6%9E%97%E5%B0%9A%E6%A5%B7/Desktop/RUST/CaveGame/physic.md) — Swept AABB 碰撞與 Kinematics 物理引擎技術專題

---

## 📜 許可證 (License)

本專案採用 **MIT License** 開源許可。詳情請參閱 `LICENSE` 檔案。
