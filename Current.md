## 1. Cavegame Current Architecture Ground Truth本檔案為 Cavegame 體素引擎的當前最新完全體架構藍圖。此文件做為 AI Agent (Google Antigravity) 的最高指導原則。在進行任何代碼編寫、重構或除錯前，必須嚴格遵循本文件定義之資料結構、渲染管線與物理規則，嚴禁退回舊版不相容之架構。
## 1. ⚙️ 開發核心與模組結構
**開發語言**：Rust (Stable 分支，天生具備無垃圾回收 GC-free 特性，精確控制內存生命週期)。
**遊戲引擎**：Bevy 0.14 (基於高效能多執行緒排程的 ECS 實體組件系統)。
**圖形 API**：wgpu (低階圖形驅動，Windows 平台自動掛載 Vulkan 或 DirectX 12 核心)。

### 📂 目錄結構與分工
```text
src/
├── config.rs       (全域參數配置與設定檔，包含遊戲與渲染各項常數)
├── main.rs         (狀態機初始化與全域 Plugin 註冊，程式進入點)
├── phys/           (一級物理模組，專職處理碰撞與幾何)
│   ├── mod.rs      (模組導出)
│   └── swept.rs    (處理 AABB 空間幾何與 Swept AABB 連續碰撞消解)
├── player/         (玩家控制器模組)
│   └── mod.rs      (玩家實體、滑鼠第一人稱視角、動態破壞/放置方塊互動與背包切換邏輯)
├── render/         (核心渲染管線模組)
│   ├── mod.rs      (渲染器入口與自訂材質掛載、環境光配置)
│   ├── greedy.rs   (工業級 AO 輔助型雙線性梯度貪婪網格生成演算法與 GPU 封裝)
│   ├── material.rs (流體雙面渲染與網格材質流水線管線配置)
│   ├── texture_array.rs (Texture2DArray 生成與管理，打包所有方塊貼圖)
│   └── textures.rs (材質載入器與資產管理)
├── ui/             (使用者介面模組)
│   ├── mod.rs      (UI 系統入口)
│   ├── debug.rs    (F3 定時除錯疊加層、包含區塊狀態與平滑光照動態開關)
│   ├── hud.rs      (遊戲 HUD 與準星渲染)
│   ├── main_menu.rs(預留主選單介面)
│   └── settings.rs (預留設定選單介面)
├── utils/          (通用工具模組)
│   ├── mod.rs      (模組導出)
│   └── math.rs     (全域坐標轉換、區塊索引映射與基礎數學常數)
└── world/          (核心世界資料結構與管理系統)
    ├── mod.rs      (WorldManager、區塊 3D 動態加載/卸載生命週期調度與真理之源)
    ├── systems.rs  (Bevy ECS 系統分流：處理實體生成、網格髒污追蹤與非同步加載輪詢)
    ├── chunk.rs    (Chunk 實體結構與 1D 扁平化 ChunkBuffer，儲存方塊資料)
    ├── fluid.rs    (流體動態系統：BFS 蔓延、水流等級下降與更新隊列)
    ├── generator.rs(工業級無狀態二階段地形管線：地形雕刻與 Fbm 噪音)
    ├── lighting.rs (光照子系統：天空光泛洪、方塊光與阻斷 BFS 重算)
    ├── storage.rs  (手寫 RLE 壓縮與非同步硬碟讀寫，持久化存檔)
    ├── voxel.rs    (體素基本定義與方塊類型列舉 BlockType)
    └── gen/        (地形生成演算法特定實作)
        ├── mod.rs
        └── flat.rs (超平坦地形生成器實作)
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
* **3x3 鄰居光照完工鎖 (AO Neighbor Barrier) 與 固體光照強制清零**：在網格生成前，除了檢查自身光照外，強制盤查水平相鄰的 8 個區塊是否 `is_lighting_ready`，若未完工則暫緩當前區塊的烘焙（AO Barrier）。同時，光照採樣器在遇見固體方塊時會強制歸零，杜絕固體內部殘留陽光引發的接縫透光。
* **跨區塊全域探測與聯動更新 (Global Check & Remesh Propagation)**：
  * **動態破壞/放置**：系統依賴絕對座標全域查詢 `world.get_block_global`，當玩家在區塊交界處放置或破壞方塊時，透過 6 向邊界偵測自動將相鄰區塊標記為 Dirty，確保跨區塊接縫的 Face Culling 即時無縫重算。
  * **加載期連動 (Race Condition 修正)**：當全新的區塊完成非同步生成並插入 `WorldManager` 的瞬間，主執行緒會立即主動探測 6 個相鄰軸向的舊區塊。若存在則強制將其標記為 `is_dirty = true`，強迫舊區塊重新網格化並剔除過期的邊界殘留牆，達成無瑕疵的地形接縫。
  * **放塊時序跨幀補償 (Spawn Latency Compensation)**：透過 `respawn_later` 剛性回寫機制，完美閃避 Bevy `commands.spawn` 的一幀延遲，達成玩家放置空區塊方塊瞬間 100% 同步烘焙的零延遲體驗。
* **頂點位元壓縮 (Vertex Bit Packing)**：全面淘汰傳統浮點數頂點屬性，將 `x(6)`、`y(6)`、`z(6)`、`face_id(3)` 與 `tex_layer(11)` 完美壓縮進單一 32-bit 的 `u32` 屬性 `ATTRIBUTE_PACKED_DATA` 中。徹底清除了 Position 與 Color 的內存佔用。
* **2D 紋理陣列與程序化 WGSL 著色器**：採用 `D2Array` 管理材質層級，配合手寫 `voxel.wgsl` 在 Vertex Shader 即時解包出局部座標與法線朝向，並透過 `face_id` 與 `(z, -y)` 等動態投射算法程序化生成透視插值的平鋪 UV。手動在 `commands.spawn` 掛載 Aabb 保證 Frustum Culling 在移除 Position 屬性後依然精準運作。
* **幾何繞向完美對齊**：6 個面的頂點嚴格遵循 CCW 繞向，所有軸向遵循 `rev = normal < 0` 統一規則。

## 3. 🧱 物理碰撞與玩家移動系統
* **座標慣例**：嚴格維持 Bevy 預設的 Y-up 座標系（X, Z 為水平，Y 為高度）。
* **離散軸向分離碰撞**：玩家速度向量在每影格拆解為嚴格獨立的三階段結算（X 軸位移 → Z 軸位移 → Y 軸位移）。撞擊方塊被水平彈回時，強迫維持 0.001 格安全距離外推，根除 False Grounding 浮點數微觀滲透 Bug。
* **FixedUpdate 雙軌視角同步 (Jitter-Free Camera)**：將相機的旋轉視角 (`player_look`) 與物理底座的位移 (`player_move`) 剛性綁定於相同的 `FixedUpdate` 階段，徹底消除顯示卡變動幀率與固定物理時步之間的相位差抖動 (Jitter)，達成極致跟手的 3D 視角移動。
* **玩家 AABB 尺寸與速度**：水平半徑 0.3。站立高度 1.8 格，蹲下（按住 左 Ctrl）動態縮小為 1.2 格。
* **陸地連跳與輸入緩衝 (Bunny Hopping & Input Buffering)**：實裝了玩家動作鎖存器 (`wants_to_jump`)。在 `Update` 高頻捕獲瞬間按鍵，並在 `FixedUpdate` 與長按輸入做雙軌交集。完美還原了 Minecraft 經典的按住連續起跳 (Bunny Hopping) 以及落地前提前按鍵的無縫起跳 (Input Buffering) 體驗。
* **流體物理交互與浮力系統 (Fluid Interaction & Buoyancy)**：
  * **雙層感知浮力大腦**：系統會分別針對玩家腳底 (`foot_in_fluid`) 與頭部 (`head_in_fluid`) 進行雙軌流體感知。當身處水中時，會自動套用黏滯阻力 (`vel.x * 0.8`) 減緩水平移動，並強制切換為流體浮力墜落曲線。
  * **海豚跳特權 (Dolphin Jump / Surface Escape)**：當玩家腳在水中但頭部已露出水面，按住空白鍵時將觸發「水面逃脫特權」，賦予強大的向上爆發力，完美還原如海豚般躍出水面登岸的極致流暢操作。
  * **瀑布攀爬 (Fluid Climbing)**：當玩家在水中且身體緊貼固體牆面 (`is_colliding_horizontally`)，按住空白鍵即可無視水流阻力，將水體當作梯子般向上攀爬。
* **安全防禦機制與旁觀者模式**：
  * **初次出生點地面傳送與安全鎖 (Instant Surface Spawning)**：為避免非同步地形生成導致玩家卡入地底，玩家初始座標設定於極限高空 (Y=250)，且物理重力在第一幀將被剛性凍結 (`!has_spawned`)。待系統偵測到底下 3D 區塊加載完成後，會發射垂直射線掃描出最高固體方塊，並將玩家「零延遲」直接傳送降落於該方塊表面，確保玩家登入的第一眼必定是看得到天空的地表。
  * **反卡死救援**：玩家無法在自己身體 AABB 內放置新方塊；若因地形意外卡入實心方塊，系統會在單影格內自動將玩家向上溫和頂開救援。
  * **F4 旁觀者穿牆模式 (Spectator Mode)**：透過 F4 鍵可即時切換旁觀者模式。此模式在物理引擎最頂端執行短路接管，無視地形安全鎖、AABB 碰撞與重力牽引，允許玩家透過 W/A/S/D 與 Space/Shift 進行絕對 3D 自由飛行，速度可透過 Ctrl 疾跑翻倍。同時，在滑鼠交互系統中注入了短路閘門，於此模式下完全閹割放置與破壞方塊的權限，確保上帝視角僅供純粹巡檢，無法意外物理影響世界。

## 4. 🌍 動態 3D 無限世界與持久化存檔
* **空間結構與全域高度換算**：以 32x32x32 的方塊組成 Chunk。全域使用 HashMap 進行 3D 區塊 `IVec3::new(cx, cy, cz)` 管理。地形生成算法（如 Perlin Noise）嚴格使用 `global_y = chunk_pos.y * 32 + local_y` 的全域絕對高度對齊地層，避免地貌在不同垂直區塊被重複複製。
* **垂直 3D 動態加載 (3D Render Distance)**：每影格以玩家為中心，向外加載 5x5 的水平範圍，並且垂直覆蓋 8 層區塊（CY = 0 到 7，對應高度 0 到 256）。當超出卸載距離或垂直邊界時觸發 GPU 網格實體銷毀與內存回收。系統於 `get_block_global` 實作了跨區界全域極限防護，任何小於 0 或大於等於 `WORLD_MAX_Y` (256) 的空間探測皆強制安全降級回傳 `Air`，確保 `greedy.rs` 的邊界 Culling 極度穩定。
* **非同步環形螺旋加載管線 (Async Ring-Sorted Loading Pipeline)**：放棄傳統同步巢狀迴圈，改採 3D 距離平方進行排序（`diff.x^2 + diff.y^2 + diff.z^2`），確保玩家腳下與視角前方的區塊享有最高加載權重。主執行緒透過 `.take(4)` 每影格限流派發最多 4 個任務，由背景的 `AsyncComputeTaskPool` 執行 Perlin Noise 或硬碟讀取，徹底杜絕 CPU 瞬間負載超載所引發的 Stuttering（掉幀）。
* **資料與實體徹底解耦 (Data & Entity Decoupling)**：純空氣區塊僅作為包含資料 `ChunkBuffer` 的 `ChunkEntry` 留在 HashMap 中，**不佔用任何 Bevy ECS 實體**。當玩家在純空區塊放置第一顆實心方塊時，才會觸發**延遲生成 (Lazy Spawning)** 動態建立網格實體。並配合 **ECS Component Flash Sync** 於每次網格生成前深度同步，確保 `Chunk` 組件狀態與資料層真理之源 100% 一致。
* **一維資料結構與 RLE 完美存檔防禦 (Save Bloating Defense)**：使用原生的一維扁平化陣列 `ChunkBuffer` `[BlockType; 32768]` 作為核心資料儲存，全面淘汰巢狀陣列。存檔時交由 IoTaskPool 異步寫入硬碟，並採用手寫的 **RLE (Run-Length Encoding) 遊程編碼** 將巨型連續空方塊極限壓縮。
* **雙軌真空剔除防線 (Dual-Vacuum Culling)**：在存檔判定上實施了極為嚴格的「固體與流體」雙重真空判定。只有當區塊內 100% 全是空氣，並且沒有一滴殘留流體時，才會無情刪除舊存檔以釋放空間。此防線確保了高空的懸浮水源與瀑布絕對不會在重啟後無端蒸發。卸載輪詢具備**完美閉環**：只要超出渲染距離，必定無條件執行 `despawn_recursive` 銷毀視覺實體並從 HashMap 移除，從根源杜絕存檔無限膨脹與記憶體釘子戶。

## 5. 🎛️ 遊戲狀態機與 UI/Debug 系統
* **GameState 狀態控制**：切分 `MainMenu`、`InGame` 等狀態，退回選單時背景世界與物理會凍結。
* **快捷列與手持切換系統 (Hotbar & Selection)**：玩家擁有 9 格 `hotbar`，前 3 格預設為 Stone, Dirt, Grass，其餘為 Air (空手)。可透過數字鍵 `1-9` 瞬間指定，或利用 `滑鼠滾輪` 動態輪詢切換（內建能量累加器與剛性邊界防護），徹底消除連續相同方塊導致的「卡頓錯覺」。右鍵建築時動態讀取選中方塊，大幅提升建造流暢度。
* **快捷列 UI 介面 (Hotbar HUD)**：在 `hud.rs` 中實作了完整的 UI 疊加層。於底部中央繪製 9 個方格，內部左上角配有 1~9 的微型數字索引（徹底移除了底部佔位的方塊名稱，讓出全額視覺空間）。我們為 1~9 格全數建立了 3D 離屏渲染攝影棚 (透過 9 組 `Camera3dBundle` 與 `RenderTarget` 空間隔離至 `RenderLayers::layer(1)`)，將 3D 相機大幅拉近 (`scale: 0.65`) 並擴充 UI 節點至 `40x40`，常態性 100% 滿版顯示正宗 Minecraft 風格的 3D 立體方塊。網格捨棄了單張圖的標準材質，改為手動建構 32-bit `ATTRIBUTE_PACKED_DATA` 並掛載 `VoxelMaterial`，確保草方塊等複雜方塊能精準還原「頂面、側面、底面」的真實多重材質貼圖。選中框維持半透明灰底，僅邊框高亮黃色。空氣方塊 (`Air`) 則直接套用 `Visibility::Hidden` 確保極度乾淨。
* **目標方塊鋼絲邊框 (Target Wireframe)**：實作了 3D 體素射線步進 (Raycast) 系統，每影格精準捕捉距離玩家視線 5.0 公尺內的第一個固體方塊。並利用 `Gizmos` 在方塊外部繪製 1.002 倍微擴張的黑色立方體邊框，徹底解決深度貼面造成的 Z-fighting 閃爍，提供極致的視覺對齊反饋（旁觀者模式自動隱藏）。
* **F3 Debug HUD 異步更新分流**：使用 `Timer::from_seconds(0.5)` 控制 F3 疊加層上的 FPS 與 Frame Time 刷新頻率，解決文字因幀率震盪而閃爍看不清的問題；而玩家座標等空間數據則無延遲即時更新。同時追加顯示目前快捷列手持之方塊 (`Holding`)。
* **F3 + C 區塊邊界檢視 (Chunk Borders)**：實作了致敬 Minecraft 的除錯快捷鍵。在 F3 開啟狀態下按下 C 鍵，可透過獨立且零干擾的 `Bevy Gizmos` 系統，精準繪製出對齊世界座標的 32x32x32 黑色區塊邊界，大幅協助空間除錯。

## 6. ⛰️ 全域無狀態地形生成器 (1D Stateless Terrain Generator)
* **無狀態二階段生成 (Two-Pass Generation)**：屏除所有局部計數器與光線透射邏輯。第一階段生成純粹的密度場緩存 `density_cache` 與地表高度緩存 `base_h_cache`；第二階段嚴格根據給定的 XY 座標查找緩存陣列，決定方塊材質，避免地形邊界出現割裂與狀態不一致。
* **多級區段平滑混合 (Multi-Stage Parameter Blending)**：廢除死板的單一公式，透過低頻 `r` 指標（0.0~1.0）劃分生態。動態平滑解算出 `base_h`、`amplitude` 與 `mountain_weight`，完美過渡平原、丘陵與高度直插 Y=200 的峭壁巨山。
* **脊狀分形噪聲混合 (Ridged Noise Blending)**：引入幾何對折公式將 Fbm 的波峰翻轉為刀鋒峭壁。根據 `mountain_weight` 動態融合圓潤的 Fbm 與尖銳的脊狀噪聲，確保平原柔和、高山崢嶸。
* **天坑與溶洞生態系統**：透過獨立的 3D 溶洞噪音結合距離地平線 `Y=64` 的二次函數衰減塑造龐大的地下通道；並引入特權的 `entrance_gate` 2D 破口閘門，一旦觸發，溶洞可無視高空衰減限制，暴力切開地表，形成壯觀的天然天坑。
* **草地與生態高空雙軌制**：實施「高空無條件解鎖 ＋ 地表誤差防禦」。只要高度超過溶洞帶 (Y >= 115) 或是處於地平線 ±12 格以內的區域，即可合法披上草皮與泥土。完美解決巨山山頂光禿無草的 Bug，並保留深淵的岩石裸露感。

## 7. 💡 全域光照引擎與極限效能優化 (Global Lighting Engine & Extreme Performance)
* **O(1) 高度圖快取 (Heightmap Cache)**：為了在未生成的區塊與天空邊界判定陽光遮蔽，系統將 `get_max_surface_y` 的高昂地形噪聲計算全面移交給背景 `AsyncComputeTaskPool` 處理。生成的 2D 高度圖 `max_surface_y_map` 會被 `WorldManager` 進行快取，主執行緒的 `get_light_global` 僅需執行極速的 O(1) 陣列查表。
* **純空氣區塊光照駐留 (Pure-Air Chunk Light Retention)**：`ChunkLightBuffer` 嚴格綁定於資料層的 `ChunkEntry`，而非 ECS 實體。這意味著就算是一個不包含任何固體的 100% 空氣區塊，依然能夠承載真實的光照漸層衰減（例如陽光從 15 衰減至 12）。此架構完美消滅了邊界交接處的死黑斷層。
* **渲染管線資料解耦 (Data-Layer Decoupling)**：貪婪網格化 `greedy.rs` 在抓取相鄰區塊的光照資訊時，直接與 `WorldManager` 溝通取得資料層的數據，徹底跳脫對 `Query<&mut Chunk>` 的依賴，大幅提升多執行緒安全度與程式碼的執行效能。
* **太陽直射不減光鐵律 (Direct Sunlight Vertical Propagation)**：優化了 BFS 光照傳播演算法與動態方塊破壞的初始賦值邏輯。現在光線向正下方（-Y）傳播時若為最高亮度（15），將無條件直接繼承 15；玩家破壞地表方塊時若上方為 15，新空氣格也將立即獲得 15。完美還原了陽光筆直穿透深洞的物理現象。

## 8. 🌫️ 動態環境霧化與視覺包覆 (Dynamic Environment & Fog Alignment)
* **視線亮度感知與背景同步 (Eye-Light Background Sync)**：遊戲實作了 `update_dynamic_environment` 系統，每幀根據玩家眼部的天空光數據（`eye_light`），動態插值（Lerp）出合適的環境色，並套用於視窗的 `ClearColor`。確保玩家從地表潛入洞穴時，背景顏色能從明亮的天藍色滑順地過渡至帶有微弱環境光 (`min_ambient_light`) 的深灰色，徹底消除畫面突變的生硬感，同時維持洞穴底部的基礎幾何辨識度。
* **相機遠剪裁面動態對齊與原生迷霧阻斷 (Far Clip Alignment & Fog Falloff)**：將相機的 `far` 剪裁面與渲染視距 (`render_distance`) 動態剛性鎖死為 `max_distance + 64.0`，賦予幾何體充裕的深層演算空間。同時結合 Bevy 原生的 `FogSettings` 實施黃金比例漸變：在 `max_distance * 0.75` 處柔和起霧，並在 `max_distance - 8.0` 處完全阻斷。這將地圖加載邊界完美遮蔽，達成了無瑕疵的超遠景深包覆。
* **著色器端貪婪矩形幾何微外推 (Quad Edge Padding in Shader)**：廢棄了傳統消耗龐大算力的 CPU 端「頂點焊接」與錯誤的「法線外推」機制。在 GPU 的 `voxel.wgsl` 頂點著色器中，利用 `@builtin(vertex_index)` 計算出該頂點於 Greedy Quad 中的 2D 角落象限（如 BL, BR, TR, TL），並根據其所屬面 (`face_id`) 的切線 (Tangent) 與副切線 (Bitangent) 軸向，將網格邊緣向平面外側精準擴張 `0.0005` 格。此舉在物理底層促使相鄰的面片產生微觀交織重疊，以零效能折損的代價徹底絕殺了 T-Junction 所引發的靜態藍點縫隙與漏光破綻。
* **自定義屬性佈局與不透明防線 (Strict Attribute Layout & Alpha Lock)**：採用客製化 VoxelMaterial 管線，將頂點屬性極限壓縮至 `@location(0) packed_data` 與 `@location(1) flow_vector` 雙插槽，徹底剔除了非法 Built-in 對屬性佈局的記憶體錯位污染。同時於 Fragment Shader 中將固體方塊的 Alpha 通道剛性鎖死為 `1.0`，斷絕圖集透明像素引發的地形透明化災難，確保世界實心無破綻。

## 9. 🌊 動態流體物理與渲染系統 (Dynamic Fluid Physics & Rendering)
* **雙軌道貪婪網格化 (Dual-Pass Meshing)**：徹底重構了 `generate_greedy_mesh`，現在它會同步輸出固體網格與流體網格（`solid_vertices`, `fluid_vertices`）。流體網格完全繞過傳統的貪婪合併，改為**逐體素生成 (Per-Voxel Generation)**，賦予每一個水面獨立的頂點控制權。
* **逐頂點流體內插與動態分母平滑斜面 (Dynamic Per-Vertex Fluid Interpolation)**：實作了針對流體頂面（+Y）角落的 2x2 全域管柱高度採樣。採用「動態流體分母演算法」，空氣方塊不參與平均以避免邊緣塌陷；同時牆壁頂點會對稱地採樣周圍 4 象限的「最高真實水位」作為鏡像基準，確保水流斜面左右絕對對稱，並呈現與《Minecraft》一致的連續動態幾何斜面。
* **無限水源剛性幾何封頂 (Rigid Source Block Meshing)**：原生水源 (`0x80`) 與瀑布落水柱擁有最高渲染特權，直接繞過周圍高度採樣，無條件強制鎖死為 100% 滿格，確保高空水塊維持完美 3D 立方體不變形。
* **純淨位元傳播與階梯重力自適應 (Pure Bitwise Flow & Gravity-Aware Stairs)**：BFS 擴散嚴格執行 `0x0F` 純淨水位解碼與 `0x80` 水源標籤隔離，消滅惡性水源遺傳。同時具備階梯重力感知，瀑布落在實心方塊上會自動轉為水平自然遞減，而非暴力灌滿。
* **零縫隙側臉密封與法線校準 (Zero-Gap Sealing & Winding Order)**：在生成側面時強制頂端兩個頂點繼承與頂面邊角一模一樣的高度位移量，確保物理密封。並針對水平四向側面實施了極度嚴謹的方向特異性索引繞行 (Face-Specific Winding Order)，徹底擊殺 Backface Culling 漏光現象。
* **半透明雙材質著色器 (Translucent Dual Materials)**：為流體引入 `AlphaMode::Blend` 獨立材質球，修改 `voxel.wgsl` 自動施加 0.9 的藍色透明度 Tint。利用 32-bit `ATTRIBUTE_PACKED_DATA` 冗餘位元（25~27 bits）即時解碼幾何下沉量，兼顧美學與頂級效能。
* **全域聯鎖物理與流動擴散 (Fluid Interlocking Physics)**：引入基於 0.1 秒心跳的 BFS 擴散引擎，支援水平擴散遞減與「重力截斷鐵律」。並在玩家採掘互動中掛載「方塊破壞喚醒機制」，敲碎支撐方塊瞬間能立刻喚醒周圍 6 向休眠水體，實現真實的流瀑崩塌物理。
* **全域無縫世界 UV 投影 (World-Space Planar Mapping)**：徹底廢除流體的 Local UV 採樣，改採 Fragment Shader 即時讀取 `in.world_position`，並搭配 `in.world_normal` 自動映射出世界坐標平面 UV。配合由 `px - nx` 高度差解算出的真實向外擴散梯度向量，讓水流紋理跨越方塊邊界完美平鋪，並呈現真正的「放射狀擴散」。
* **下落全滿特權與去內壁化 (Falling Column & Internal Culling)**：實作了極簡去內壁化鐵律，水平面嚴格限定 `nf == 0` 才允許生成外圍水牆，徹底消滅相鄰水方塊之間的內部隔板穿幫。同時導入三維掃描算子：當方塊為滿級(8)且上方有水注入、四周存在空氣缺口時，強制觸發「下落全滿特權」，將該頂面的 4 個角落位移清零，瞬間化身 100% 飽和的垂直立體瀑布。
* **動態局部期望平衡演算法 (Dynamic Fluid Re-evaluation)**：徹底廢除舊有「單向擴散」盲推邏輯。系統會動態解算每個水方塊的「期望水位」，當期望值與實際值失衡（例如水源被移除導致退潮）時，方塊會主動更新並將周圍 6 向鄰居重新壓入佇列。配合 `Level 9` 絕對水源保留機制，達成完美無死迴圈的擴散與收斂（退潮）連鎖反應。
* **放水/收水雙模態交互 (Dual-Mode Fluid Interaction)**：升級 F 鍵交互邏輯。若射線瞄準既有水體，則執行「收水」並觸發退潮連鎖；若瞄準無水處，則注入「絕對水源」並啟動擴散。完全還原《Minecraft》水桶的雙向動態操作體驗。
* **Minecraft 規格動態尋路大腦 (5-Block Lookahead Pathfinding)**：導入具備 5 格遠視的懸崖探測系統。具備純淨拓撲掃描防線，嚴格區分真實深淵與瀑布水柱；同時實作「局部平地開路特權」，允許水流在鎖定遠處懸崖的同時溢入一格高的平地隧道，解決逆流綁架死鎖。
* **立體喚醒防線與高效全域去重 (3x3x3 Wake-up & Queue Deduplication)**：方塊更新時強制喚醒周圍 3x3x3 立體空間的空氣與流體，並利用 `HashSet` 達成 `O(1)` 極速去重與嚴格的「固體過濾」，徹底防堵隊列洪水 (Queue Flooding) 癱瘓每幀運算預算，保證水流不卡頓不斷流。
* **精準頂面剔除特權 (Precise Top Face Culling)**：廢除了舊版粗暴的固體天花板遮擋剔除。一格高隧道內的流動水 (Level 1~7) 頂面將 100% 正常渲染，呈現完美平滑的水流遞減斜面。同時保留滿格水源 (Level 8) 的 Z-Fighting 防閃爍隱藏機制，達成物理與渲染的極致一體化。

## 10. 🚀 極限效能優化與非同步網格管線 (Extreme Performance & Async Meshing Pipeline)
* **全域配置模組與編譯器優化 (Global Config & Const Optimization)**：建立了 `config.rs` 集中管理 `render_distance` 與流體參數。但為了防止每影格讀取導致的效能衰退，將 `MAX_FLUID_LEVEL` 等底層規則強制回歸 `const`，成功釋放 Rust 編譯器的「常數摺疊與循環展開 (Loop Unrolling)」極致優化，FPS 暴增回穩。
* **非同步網格線程池 (Async Meshing Task Pool)**：將 `Greedy Meshing` 與流體網格生成徹底剝離主執行緒。利用 Bevy 的 `AsyncComputeTaskPool` 在背景並行計算，並實作 `ComputeMeshTask` 進行異步追蹤，徹底消滅了玩家移動或載入新地圖時引發的卡頓 (Stuttering)。
* **輕量化資料抽離 (Lightweight Data Extraction)**：為了防止傳入背景線程時發生恐怖的全域 HashMap Deep Clone，設計了專用結構 `ChunkMeshInputData`。僅提取當前區塊與 6 個鄰居的純陣列 Buffer `Box<[u8; 32768]>` 快照交給線程，達成了極低開銷的零鎖並行。
* **唯讀邊界防禦與安全降級 (Read-Only Boundary Defense)**：為了防止網格化查詢鄰居時誤觸「自動生成」機制引發記憶體幾何級數暴漲，實施了鐵血唯讀查詢。對於超出邊界或尚未加載的區塊，實作了精準的「安全降級」：低於地表視為堅硬的 `Stone`，高於地表視為 `Air`，完美封裝邊界破口。
* **純空氣區塊稀疏配置 (Sparse Chunk Allocation & Vacuum Culling)**：實作了 O(N³) 記憶體炸彈的終極解法。當區塊生成完畢發現內部 100% 為空氣且無流體時，將剛性攔截其進入 HashMap 與 ECS 實體池的資格。透過專屬的 `vacuum_chunks` 黑名單機制阻擋重複加載，令天空區域達到絕對的**零記憶體佔用**。
* **扁平化 3D 螺旋視野 (Flattened 3D Render Distance)**：將區塊掃描系統從原本的「無視高度無限柱子 (0~7)」升級為以玩家為中心，上下各 2 層的「動態 3D 扁平包圍盒」。加載與卸載邏輯雙管齊下同步限制 Y 軸，大幅砍掉了高達 70% 的無效地下/高空區塊運算。
* **動態視野迷霧連動 (Dynamic Fog Distance Alignment)**：將 Shader `voxel.wgsl` 中死板的 64 格距離常數徹底拔除。現在迷霧的 `fog_start` 與 `fog_end` 完全綁定 `render_distance` 動態變化，並在片元著色器中採用精準的 `clamp` 線性插值，實現了與視距完美連動的遼闊地平線。
* **空網格實體剛性剔除 (Zero-Vertex Entity Culling)**：針對地底深層 100% 被岩石包覆、完全無法被看見的區塊，在非同步任務完成後加入了 `has_geometry` 頂點長度過濾。一旦返回零頂點網格，系統將直接清空任務標籤，**拒絕呼叫** `commands.spawn` 分配 Bevy 子實體，維持 ECS 場景圖的絕對輕盈乾淨。

## 11. 🛸 高度自適應邊界與動態復活系統 (Height-Adaptive Boundary & Revival System)
* **高度自適應隱含背景 (Height-Adaptive Implicit Background)**：重構了全域查詢接口，當遇到尚未加載或被剔除的純空氣區塊 (`None`) 時，依據 Y 軸高度動態回傳環境值：高空 (Y>=2) 回傳 `Air`，地底 (Y<2) 回傳 `Stone`。徹底解決了邊界隱形牆與掉入虛空的物理 Bug。
* **物理碰撞與重力解鎖 (Unified Physics Collision Interface)**：移除了舊有粗暴的「第一幀地形安全鎖」，物理引擎現已全盤信賴高度自適應查詢。允許玩家在高空未加載區塊自由落體，並在地底未加載區塊安全行走。
* **嚴格限制的動態復活機制 (Strict Dynamic Chunk Revival)**：徹底阻擋光照與流體蔓延至未載入區塊時誤觸發記憶體寫入。將「動態復活」權限嚴格限定於「玩家主動放置方塊」的那一刻。且復活時依據高度自動將背景填滿 `Stone` 或 `Air`，完美消滅地底邊界的 32x32 巨型灰色平面破綻。
* **高空復活區塊光照自適應 (Height-Adaptive Sky Light for Revived Chunks)**：修正高空動態復活區塊預設為黑洞的 Bug。當高空復活時，強制將光照層填滿 15 級大自然陽光 (`0xF0`)，並即時向周圍 6 向鄰居擴散 `is_dirty` 信號，保證網格重新生成完美無瑕的陽光過渡面。
* **防溢位精準計數器校準 (Underflow Prevention for Block Counters)**：修復了 `Chunk::new()` 預設 `non_air_count` 為 0 導致複製上萬個地底石頭後，玩家一挖方塊即觸發 `attempt to subtract with overflow` 的崩潰 Bug。在 Lazy Spawn 時強制進行 `chunk.buffer.blocks.iter().count()` 精準校準，遊戲穩定性達到新高。

## 12. 🌈 光照泛洪與渲染防線同步系統 (Light Propagation & Render Synchronization)
* **正統光照阻斷泛洪更新 (Light Removal BFS)**：徹底移除了原先硬編碼暴力灌 0 的補丁。在玩家放置固體方塊時，改為啟用正統的雙層 BFS 佇列演算法：先消除被阻斷的舊有光照 (Unpropagate)，再從周邊尚未受影響的光源重新向內蔓延 (Re-propagate)，達成極致自然的陰影衰減，完美模擬光線的遮蔽連鎖反應。
* **區塊初始化光照完工剛性同步鎖 (Chunk Initialization Light Sync Lock)**：為了解決 ECS 生成延遲 (Spawn Latency) 導致網格烘焙過早讀取未結算光照、引發「地底幽靈牆面發光」的惡性競爭條件 (Race Condition)：將 `is_lighting_ready` 標籤從 Component 抽離，剛性寫入 `WorldManager` 的底層字典 `ChunkEntry` 中。`greedy.rs` 渲染前會直接向資料層詢問，只有當光照 BFS 徹底清空佇列後才會秒刻解鎖，徹底封殺渲染偷跑。
* **流體頂點環境光遮蔽平滑化 (Fluid Vertex Ambient Occlusion Smoothing)**：解決了隧道口水流因吃到周邊 15 級光照而導致邊界呈現刺眼亮藍的穿幫破綻。全面廢除單一面光照採樣，升級為 4-Corner 頂點平滑採樣：在生成水方塊的 Quad 時，每一個頂點會獨立向其相鄰的 4 個方塊進行採樣並求取平均 `(light_sum / 4)`，實現了水流在明暗交界處那極其柔順的環境光過渡漸層。

## 13. 🖼️ AO 輔助型梯度貪婪網格 (AO-Aware Gradient Greedy Meshing)
* **3 方塊對角線 AO 採樣 (Diagonal AO Sampling)**：為固體方塊實裝了工業級 Voxel AO。每個表面在計算頂點光照時，會精準向 U 軸、V 軸與 U+V 對角線的 4 個鄰居格採集環境光並取平均。讓凹陷牆角與洞穴深處展現極度深邃的環境光遮蔽黑暈。
* **光照梯度感知合併 (Gradient-Aware Merging)**：打破了傳統貪婪網格「光照不同即碎裂」的效能魔咒。只要相鄰方塊的 U 軸與 V 軸光照斜率（Slope）保持恆定（即符合雙線性插值），系統就會大膽放行橫向與縱向的無限合併。這保證了平原與均勻陰影邊界能維持 O(1) 的超大網格效能，同時完美呈現畫素級的平滑漸層。
* **斜率快取剛性重置與連鎖髒污令 (Slope Reset & Mesh Invalidation)**：確保合併截斷時立刻清空初始斜率快取，防止漸層污染。並在 F3 除錯選單實作了 `P` 鍵全域畫質切換開關，切換時會剛性觸發全域實體 `is_dirty = true`，逼迫渲染引擎在一幀內連同水流與固體一併完成背景重新烘焙，達成零延遲的畫質切換體驗。

## 14. 🛠️ 開發環境與工作流 (Development Workflow)
* **VS Code 終端環境自適應 (.vscode)**：為避免系統環境變數遺失引發的終端機報錯，專案於根目錄掛載了專屬的 `.vscode/settings.json`，強制將 Cargo 路徑注入整合終端機。同時配備 `.vscode/tasks.json`，讓開發者只需按下 `Ctrl+Shift+B` 便能一鍵無縫 `cargo run`，維持最高的開發效率。