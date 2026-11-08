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
├── item/           (一級物品與背包模組，專職處理物品、工具註冊表與 ItemStack/Inventory 背包模型)
│   └── mod.rs      (ItemType, ItemKind, ItemDefinition, ItemRegistry, ItemStack, Inventory Component 與掉落函數)
├── phys/           (一級物理模組，專職處理碰撞與幾何)
│   ├── mod.rs      (模組導出)
│   └── swept.rs    (處理 AABB 空間幾何與 Swept AABB 連續碰撞消解)
├── player/         (玩家控制器模組)
│   ├── mod.rs      (玩家實體、第一人稱視角、動態互動、背包快捷列切換與輸入捕獲)
│   └── vitals.rs   (玩家生理狀態機：Health、Stamina、Thirst、體溫、潮濕度、背包負重結算)
├── render/         (核心渲染管線模組)
│   ├── mod.rs      (渲染器入口與自訂材質掛載、環境光配置)
│   ├── greedy.rs   (工業級 AO 輔助型雙線性梯度貪婪網格生成演算法與 GPU 封裝)
│   ├── lod.rs      (Voxy 風格微體素階梯 LOD 地平線系統、半透明遠洋水體與大氣迷霧全域同步)
│   ├── material.rs (流體雙面渲染與網格材質流水線管線配置)
│   ├── particles.rs(輕量化 3D 體素碎屑與火花粒子系統)
│   ├── texture_array.rs (Texture2DArray 生成與管理，打包所有方塊貼圖)
│   └── textures.rs (材質載入器與資產管理)
├── ui/             (使用者介面模組)
│   ├── mod.rs      (UI 系統入口)
│   ├── debug.rs    (F3 定時除錯疊加層、區塊狀態、生理數值與平滑光照動態開關)
│   ├── hud.rs      (遊戲 HUD、準星、3D 離屏渲染 Hotbar 與 Vitals 體徵數值狀態條)
│   ├── inventory.rs(36 格全功能沉浸式背包管理介面)
│   ├── main_menu.rs(預留主選單介面)
│   └── settings.rs (預留設定選單介面)
├── utils/          (通用工具模組)
│   ├── mod.rs      (模組導出)
│   └── math.rs     (全域坐標轉換、區塊索引映射、3D DDA 體素射線步進與基礎數學常數)
└── world/          (核心世界資料結構與管理系統)
    ├── mod.rs      (WorldManager、區塊 3D 動態加載/卸載生命週期調度與真理之源)
    ├── systems.rs  (Bevy ECS 系統分流：處理實體生成、網格髒污追蹤與非同步加載輪詢)
    ├── chunk.rs    (Chunk 實體結構與 1D 扁平化 ChunkBuffer，儲存方塊資料)
    ├── collapse.rs (地質結構穩定度、礦坑木支架與覆岩應力坍塌物理)
    ├── fluid.rs    (流體動態系統：BFS 蔓延、水流等級下降與更新隊列)
    ├── generator.rs(工業級無狀態二階段地形管線：地形雕刻與 Fbm 噪音)
    ├── lighting.rs (光照子系統：天空光泛洪、方塊光與阻斷 BFS 重算)
    ├── registry.rs (數據驅動註冊表：BlockDefinition, BlockRegistry, BLOCK_DEFINITIONS 靜態無鎖表與 SixFaces 貼圖映射)
    ├── soil.rs     (鬆散土壤垂直重力與安息角滑坡物理)
    ├── storage.rs  (手寫 RLE 壓縮與非同步硬碟讀寫，持久化存檔)
    ├── torch.rs    (火把 180s 燃盡熄滅生命週期、Phase 5a BFS 光源消除與黑煙碎屑)
    ├── tree.rs     (樹木 Timber 連鎖砍伐、葉落掉枝與空手樹幹重力滑落物理)
    ├── voxel.rs    (體素 BlockType 列舉，固有方法委派全域 static 表 0 鎖定開銷 O(1) 常數直尋)
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
* **頂點位元極限壓縮 (16-bit XYZ & Dual Light Packing)**：全面淘汰傳統浮點數頂點屬性，將 `xyz` 降維轉換為一維索引 (`x + y*33 + z*1089`) 塞入 16-bit (0~35936)，剩餘位元完美分配給 `face_id(3)`、`tex_layer(4)`、`block_light(4)` 與 `sky_light(4)`。單一 32-bit 的 `u32` 屬性 `ATTRIBUTE_PACKED_DATA` 完美容納幾何、材質與雙軌光照，徹底清除了 Position 與 Color 的內存佔用。
* **物理與圖形嚴格解耦 (Solid vs Opaque Separation)**：
  * **`is_solid()`** 專司物理碰撞，決定 AABB 幾何障礙物。
  * **`is_opaque()`** 專司貪婪網格 Face Culling。相鄰方塊必須為實心且「完全不透明」（如石頭/泥土），才能剔除當前面。此防線確保了玻璃 (`Glass`) 與樹葉 (`OakLeaves`) 交界處不會產生錯誤的透明破洞。
* **2D 紋理陣列與程序化 WGSL 著色器**：採用 `D2Array` 管理材質層級，配合手寫 `voxel.wgsl` 在 Vertex Shader 即時解包出局部座標與法線朝向，並透過 `face_id` 與 `(z, -y)` 等動態投射算法程序化生成透視插值的平鋪 UV。手動在 `commands.spawn` 掛載 Aabb 保證 Frustum Culling 在移除 Position 屬性後依然精準運作。
* **幾何繞向完美對齊**：6 個面的頂點嚴格遵循 CCW 繞向，所有軸向遵循 `rev = normal < 0` 統一規則。

## 3. 🧱 物理碰撞與玩家移動系統
* **座標慣例**：嚴格維持 Bevy 預設的 Y-up 座標系（X, Z 為水平，Y 為高度）。
* **離散軸向分離碰撞**：玩家速度向量在每影格拆解為嚴格獨立的三階段結算（X 軸位移 → Z 軸位移 → Y 軸位移）。撞擊方塊被水平彈回時，強迫維持 0.001 格安全距離外推，根除 False Grounding 浮點數微觀滲透 Bug。
* **FixedUpdate 雙軌視角同步 (Jitter-Free Camera)**：將相機的旋轉視角 (`player_look`) 與物理底座的位移 (`player_move`) 剛性綁定於相同的 `FixedUpdate` 階段，徹底消除顯示卡變動幀率與固定物理時步之間的相位差抖動 (Jitter)，達成極致跟手的 3D 視角移動。
* **玩家 AABB 尺寸與速度**：水平半徑 0.3。站立高度 1.8 格，蹲下（按住 左 Ctrl）動態縮小為 1.2 格。
* **陸地連跳與輸入緩衝 (Bunny Hopping & Input Buffering)**：實裝了玩家動作鎖存器 (`wants_to_jump`)。在 `Update` 高頻捕獲瞬間按鍵，並在 `FixedUpdate` 與長按輸入做雙軌交集。完美還原了 Minecraft 經典的按住連續起跳 (Bunny Hopping) 以及落地前提前按鍵的無縫起跳 (Input Buffering) 體驗。
* **流體物理交互與立體游泳系統 (Fluid Interaction & Swimming Physics)**：
  * **雙層感知浮力大腦**：系統分別針對玩家全身 AABB (`in_fluid`) 與頭頂 (`head_in_fluid`) 進行雙軌流體感知。身處水中時套用黏滯阻力與平滑慣性（水平加速度基底 10.0*dt，阻尼係數 6.0），保持流體真實滑行體驗；按住 Shift 可切換為疾跑游泳（0.65x 速度）。
  * **水面平穩踩水 (Surface Treading)**：開闊水面露出頭部時按住 Space，系統賦予溫和向上的穩定浮力並剛性限速在 `vel.y <= 1.0 m/s`，玩家眼部能自然穩定在水線上方觀察四周，徹底消滅了舊版每影格重複施加 8.5 m/s 導致的「水面彈簧床連續拋射」破綻。
  * **岸邊登岸翻越 (Ledge Vault / Shore Hop)**：當玩家頭部露於水面、正前方發生水平方塊碰撞 (`is_colliding_horizontally`) 且輸入向前推進時，按下 Space 將獲得適度的登岸翻越衝量 (`6.38 m/s`)；並透過 `vel.y < 2.0` 單次觸發閘門，確保單次按鍵僅產生單一平滑跳躍弧線，精準翻越 1 格高的河岸或湖邊台階。
  * **穩態瀑布攀爬 (Controlled Fluid Climbing)**：完全浸於水中時貼緊固體牆面按 Space，系統提供平穩上升的恆定攀爬力（限速 `3.0 m/s`），杜絕了深海貼壁時瞬發 8.5 m/s 垂直火箭電梯的失真物理。
  * **下蹲下潛統一控制 (Submerged Diving)**：水中下潛統一綁定為下蹲鍵（`ControlLeft`，亦支援 `KeyC`），與陸地下蹲邏輯完全對齊，徹底釋放 `ShiftLeft` 給游泳疾跑，解決鍵位語義衝突。
  * **水底起跳水阻保留 (Waterbed Jump Drag)**：在淺水或河床上起跳時，跳躍衝量適度受水體阻力衰減（0.85x），且水平移動維持流體慣性，杜絕起跳瞬間水阻瞬時蒸發。
* **安全防禦機制與旁觀者模式**：
  * **初次出生點地面傳送與安全鎖 (Instant Surface Spawning)**：為避免非同步地形生成導致玩家卡入地底，玩家初始座標設定於極限高空 (Y=250)，且物理重力在第一幀將被剛性凍結 (`!has_spawned`)。待系統偵測到底下 3D 區塊加載完成後，會發射垂直射線掃描出最高固體方塊，並將玩家「零延遲」直接傳送降落於該方塊表面，確保玩家登入的第一眼必定是看得到天空的地表。
  * **反卡死救援**：玩家無法在自己身體 AABB 內放置新方塊；若因地形意外卡入實心方塊，系統會在單影格內自動將玩家向上溫和頂開救援。
  * **F4 旁觀者穿牆模式 (Spectator Mode)**：透過 F4 鍵可即時切換旁觀者模式。此模式在物理引擎最頂端執行短路接管，無視地形安全鎖、AABB 碰撞與重力牽引，允許玩家透過 W/A/S/D 與 Space/Shift 進行絕對 3D 自由飛行，速度可透過 Ctrl 疾跑翻倍。同時，在滑鼠交互系統中注入了短路閘門，於此模式下完全閹割放置與破壞方塊的權限，確保上帝視角僅供純粹巡檢，無法意外物理影響世界。

## 4. 🌍 動態 3D 無限世界與持久化存檔
* **空間結構與全域高度換算**：以 32x32x32 的方塊組成 Chunk。全域使用 HashMap 進行 3D 區塊 `IVec3::new(cx, cy, cz)` 管理。地形生成算法嚴格使用 `global_y = chunk_pos.y * 32 + local_y` 的全域絕對高度對齊地層。
* **垂直 3D 動態加載與 1024 宏觀世界 (1024-Block Macro World)**：垂直覆蓋 **32 層區塊**（CY = 0 到 31，對應高度 0 到 1024，`WORLD_CHUNKS_Y = 32`, `WORLD_MAX_Y = 1024`）。全域海平面錨定在 `SEA_LEVEL = 256`。系統於 `get_block_global` 實作了跨區界全域極限防護，任何小於 0 或大於等於 `WORLD_MAX_Y` (1024) 的空間探測皆強制安全降級回傳 `Air`，確保 `greedy.rs` 的邊界 Culling 極度穩定。高空 (Y >= SEA_LEVEL) 未加載區塊隱含背景為 `Air`，地底 (Y < SEA_LEVEL) 未加載區塊隱含背景為 `Stone`。
* **非同步視錐前向加權與動態並發加載管線 (Async Frustum Forward & Dynamic Concurrent Loading)**：
  * **視角錐體前向加權 (Frustum Dot Product Prioritization)**：玩家移動與飛行時，將水平位移向量與玩家前向朝向進行點積（$\vec{v} \cdot \vec{d}$），正前方視錐錐體（FOV）內的區塊享有最高加載優先級，杜絕身後無效區塊搶佔前方視野。
  * **高空純空氣 0 微秒極速短路剪枝 (Surface AABB Fast-Culling)**：依據大陸度樣條與多碎形噪聲，以納秒級解析度推算區塊柱的理論地表極限 $base\_h + amplitude + 32$。凡底部高於該極限且高於海平面之高空區塊，0 微秒直接登記為 `vacuum_chunks`，徹底消除全地圖 65%~70% 的高空無效 3D 密度計算與記憶體開銷。
  * **動態並發任務調度 (Dynamic Concurrent Dispatching)**：徹底廢除舊版每影格固定 `take(4)` 嚴重限流瓶頸，根據 `q_generating` 任務隊列即時負載動態批次派發 8~32 個任務，完全飽和利用 CPU 多核運算，使向前飛行時地貌秒速展開。
  * **遠近分級垂直視距 (Distance-Scaled Vertical Frustum Culling) 與大洋水牆根除**：在玩家周圍 128 米核心區維持垂直全深度加載；遠景宏觀區則僅加載外露地表與水體層，過濾掉深埋於數百米實心岩層下的不可見無效區塊。大洋區塊頂部強制保證加載至海平面 (`max_visible = (surf_cy + 1).max(sea_cy)`)，徹底根除遠處海水切面斷崖水牆！
  * **Distant Horizons (DH) 2.0 公里超遠景 LOD 地平線系統 (Distant Horizons LOD Horizon System)**：
    * **人眼無感自然過渡**：核心體素渲染距離設定為 15 區塊 (480 米)，玩家視野所及範圍 100% 全為真實 3D 體素方塊與樹木，絕無任何平滑低多邊形地表在眼前穿模；480 米至 2,048 米 (64 區塊) 則在遠景迷霧中由 Distant Horizons LOD 地平線系統無縫接管，在人眼無法分辨方塊微觀結構的超遠距離呈現壯麗群山與無際汪洋！
    * **無狀態解析幾何生成**：以 8x8 區塊 (256x256 米) 為單元，利用 `get_max_surface_y` 在背景線程進行 8 米步長低多邊形網格採樣 (33x33 頂點，2,048 三角面)，並以有限差分法烘焙平滑法線與太陽漫反射光照。
    * **自然大氣透視生物群系色彩**：深海錨定平整水面深海藍 ($Y=256$)，濱海沙灘金色 ($Y \le 260$)，溫帶自然大地綠 ($260 < Y \le 520$)，高海拔花崗岩灰色 ($Y > 520$) 與 1024 宏觀雪峰白色 ($Y > 580$)。
    * **GPU 剛性防護罩與無縫過渡**：LOD Shader 動態執行 `if dist < near_voxel_radius (464m) { discard; }`，近處 100% 嚴格純方塊；體素在 375m~472m 平滑融入大氣天光，LOD 在 450m~1800m 展現壯闊遠景，達成 0 破面、0 縫隙、0 Z-Fighting 的極致過渡。
    * **極限流暢效能**：關閉無效 DirectionalLight 級聯陰影貼圖 (Shadow Maps)，消除數千 Mesh 的 Shadow Pass；`update_chunks` 實施位移觸發與 0.15s 降頻節流，主線程 CPU 負載驟降 90%；網格生成每影格限流 24 個，兼顧 480 米寬闊方塊視距與 160+ FPS 極限流暢度！
* **立體水文與地底空洞解救系統 (Realistic Hydrology & Subterranean Cave Rescue)**：
  * **海盆 vs 陸地地下洞穴嚴格分離**：海平面水體（$Y=256$）僅在大洋海盆、海溝、海灘與峽谷等「地表高程 $\le SEA\_LEVEL$」的開放水域垂直注入。陸地山脈與丘陵（$surface\_y > SEA\_LEVEL$）底下的龐大 3D 溶洞（$Y=32 \sim 240$）100% 保持乾燥通風空氣，徹底解救地底世界，恢復可自由行走、插火把與採礦的完整地下生態。
  * **高山懸崖瀑布泉眼 (Cliff Waterfall Springs)**：在高海拔峭壁邊緣（$Y > 280$ 且相鄰高度落差 $\ge 8$ 格）依據噪聲安放瀑布水源方塊，並在空氣柱預置流動水瀑，藉由流體物理引擎（`fluid_buffer` + 動態蔓延）自然形成自百米絕壁直墜深谷的動態飛瀑景觀。
* **資料與實體徹底解耦 (Data & Entity Decoupling)**：純空氣區塊僅作為包含資料 `ChunkBuffer` 的 `ChunkEntry` 留在 HashMap 中，**不佔用任何 Bevy ECS 實體**。當玩家在純空區塊放置第一顆實心方塊時，才會觸發**延遲生成 (Lazy Spawning)** 動態建立網格實體。並配合 **ECS Component Flash Sync** 於每次網格生成前深度同步，確保 `Chunk` 組件狀態與資料層真理之源 100% 一致。
* **一維資料結構與 RLE 完美存檔防禦 (Save Bloating Defense)**：使用原生的一維扁平化陣列 `ChunkBuffer` `[BlockType; 32768]` 作為核心資料儲存，全面淘汰巢狀陣列。存檔時交由 IoTaskPool 異步寫入硬碟，並採用手寫的 **RLE (Run-Length Encoding) 遊程編碼** 將巨型連續空方塊極限壓縮。
* **雙軌真空剔除防線 (Dual-Vacuum Culling)**：在存檔判定上實施了極為嚴格的「固體與流體」雙重真空判定。只有當區塊內 100% 全是空氣，並且沒有一滴殘留流體時，才會無情刪除舊存檔以釋放空間。此防線確保了高空的懸浮水源與瀑布絕對不會在重啟後無端蒸發。卸載輪詢具備**完美閉環**：只要超出渲染距離，必定無條件執行 `despawn_recursive` 銷毀視覺實體並從 HashMap 移除，從根源杜絕存檔無限膨脹與記憶體釘子戶。

## 5. 🎛️ 遊戲狀態機與 UI/Debug 系統
* **GameState 狀態控制**：切分 `MainMenu`、`InGame` 等狀態，退回選單時背景世界與物理會凍結。
* **通用背包與物品堆疊模型 (Inventory & ItemStack Model)**：全面淘汰傳統固定的 `[BlockType; 9]` 陣列，導入抽象的 `ItemStack` (封裝 `item_type`, `count`, `durability`) 與通用 ECS Component `Inventory`（可擴充槽位，支援玩家 36 格背包、寶箱或工作台）。玩家快捷列可透過數字鍵 `1-9` 瞬間切換，或利用 `滑鼠滾輪` 動態輪詢切換（內建能量累加器與剛性邊界防護）。
* **剛性清空與自動入包 (Zero-Count Cleanup & Direct Auto-Pickup)**：實裝剛性防線，當物品數量 `count` 或工具耐久度 `durability` 扣減至 0 時，槽位剛性置為 `None`。左鍵破壞方塊時自動根據 `get_block_drop` 計算掉落物並直接加入 `Inventory`，同時扣減工具耐久度 1，杜絕 3D 浮空掉落物 Entity 開銷。
* **快捷列 UI 雙軌 3D/2D 渲染介面 (Dual 3D/2D Hotbar HUD)**：在 `hud.rs` 中實作了完整的無狀態 UI 疊加層。於底部中央繪製 9 個方格，內部左上角配有 1~9 的微型數字索引。為 1~9 格全數建立了 3D 離屏渲染攝影棚 (9 組 `Camera3dBundle` 與 `RenderTarget` 隔離至 `RenderLayers::layer(1)`) 與 2D Icon 節點。
  * **`ItemKind::Block`**：顯示 3D 立體方塊網格預覽，隱藏 2D Icon。`count > 1` 時於右下角繪製數量數字。
  * **`ItemKind::Tool` / `Material`**：隱藏 3D 網格，顯示 2D 物品 Icon (由 `setup_item_icons` 於 Startup 程序化生成 16x16 像素 Icon)。工具受損時於槽位底部繪製動態長度與綠/黃/紅比例之耐久度條。
  * **`None` 空槽位**：3D 網格、2D Icon、數量數字與耐久度條一律設定為 `Visibility::Hidden`。選中框高亮黃色。
* **目標方塊鋼絲邊框 (Target Wireframe)**：實作了 3D 體素射線步進 (Raycast) 系統，每影格精準捕捉距離玩家視線 5.0 公尺內的第一個固體方塊。並利用 `Gizmos` 在方塊外部繪製 1.002 倍微擴張的黑色立方體邊框，徹底解決深度貼面造成的 Z-fighting 閃爍，提供極致的視覺對齊反饋（旁觀者模式自動隱藏）。
* **F3 Debug HUD 異步更新分流**：使用 `Timer::from_seconds(0.5)` 控制 F3 疊加層上的 FPS 與 Frame Time 刷新頻率，解決文字因幀率震盪而閃爍看不清的問題；而玩家座標與手持物品數據 (`inventory.selected_item()`) 則無延遲即時更新，完整顯示物品名稱、數量與耐久度。
* **F3 + C 區塊邊界檢視 (Chunk Borders)**：實作了致敬 Minecraft 的除錯快捷鍵。在 F3 開啟狀態下按下 C 鍵，可透過獨立且零干擾的 `Bevy Gizmos` 系統，精準繪製出對齊世界座標的 32x32x32 黑色區塊邊界，大幅協助空間除錯。

## 6. ⛰️ 全域 1024 宏觀地球級地形與水文系統 (1024-Block Earth-Scale Terrain & Hydrology)
* **無狀態二階段生成 (Two-Pass Generation)**：第一階段生成純粹的密度場緩存 `density_cache` 與地表高度緩存 `base_h_cache`；第二階段嚴格根據給定的 XY 座標查找緩存陣列決定方塊材質，避免地形邊界出現割裂。
* **大尺度大陸性與 1024 宏觀地貌分層 (Continentality & C1 Smooth Spline)**：
  - **海平面基準面**：全域統一於 `SEA_LEVEL = 256`。
  - **全域 C1 平滑樣條過渡 (Smoothstep Spline)**：徹底廢除階梯斷崖，深海（Y=80~150）、淺海大陸架（Y=150~252）、海灘（Y=252~275）、丘陵高原（Y=275~480）與萬仞雪峰（Y=480~850+）在所有分界點上高度與一階導數嚴格光滑對齊，消滅任何人工垂直斷層！
  - **淺海大陸架緩坡 (Continental Shelf)**：在水深 0 ~ 20 米的近海區域形成平緩延伸的大陸棚緩坡，玩家踩著細沙自然入水，波浪拍岸。
  - **地層礦脈高度概率衰減**：鐵礦與煤礦分佈隨高度向上平滑衰減，徹底杜絕懸崖表面出現橫向貫穿的硬水平紅線。
* **河流與大峽谷深切侵蝕 (River & Canyon Carving)**：在陸地採樣脊狀侵蝕噪聲，當 `river_sample < 0.055` 時向下暴力深切至海平面 (Y=252~256)，形成科羅拉多大峽谷般數百格落差的斷崖峭壁與瀑布！
* **全域自然水體注入系統 (Natural Water Infiltration)**：低於海平面 (`SEA_LEVEL = 256`) 且為空氣的空間，生成時自動寫入 `fluid_buffer` 滿級水源 (`8 | 0x80`)，自然湧現深邃汪洋與河道水流。
* **水下沉浸感知與深洞深淵黑暗迷霧 (Underwater & Cave Abyss Fog)**：
  - **水下沉浸**：玩家眼部浸入水中時，視野距離急劇收窄至 36 格，背景色切換為深邃海藍黑，還原真實深潛能見度；
  - **深淵與大洞感知**：Shader 根據目標體素的 `sky_light` 進行迷霧色彩插值。俯瞰無天光地底大洞、溶洞或深海海溝時，遠景迷霧平滑過渡為深邃純黑 (`vec3(0.001, 0.002, 0.005)`)，徹底杜絕「深淵底下泛藍天」的違和感！
* **深層溶洞系統**：在 Y=32 ~ 240 地底雕刻深邃地下空腔，並具備地表天坑破口閘門。
* **地底礦脈嵌入**：Y<80 地層利用 deterministic hash 點狀分佈鐵礦 (`IronOre`) 與煤礦 (`CoalOre`)。
* **跨區塊無縫樹木生成 (Cross-Chunk Trees)**：
  * 在判定樹木生成時，將掃描範圍跨界外擴至 `[-2..34]`，精準抓取周圍鄰接區塊邊界的樹木種子。
  * 若鄰接邊界長樹，其 `OakLog` 與 `OakLeaves` 枝葉若溢出至當前 `[0..32]` 區塊，將被直接寫入當前 `ChunkBuffer`。徹底絕殺了過去地形生成中常見的「邊界半棵樹被切斷」致命破綻。

## 7. 💡 全域光照引擎與極限效能優化 (Global Lighting Engine & Extreme Performance)
* **O(1) 高度圖快取 (Heightmap Cache)**：為了在未生成的區塊與天空邊界判定陽光遮蔽，系統將 `get_max_surface_y` 的高昂地形噪聲計算全面移交給背景 `AsyncComputeTaskPool` 處理。生成的 2D 高度圖 `max_surface_y_map` 會被 `WorldManager` 進行快取，主執行緒的 `get_light_global` 僅需執行極速的 O(1) 陣列查表。
* **純空氣區塊光照駐留 (Pure-Air Chunk Light Retention)**：`ChunkLightBuffer` 嚴格綁定於資料層的 `ChunkEntry`，而非 ECS 實體。這意味著就算是一個不包含任何固體的 100% 空氣區塊，依然能夠承載真實的光照漸層衰減（例如陽光從 15 衰減至 12）。此架構完美消滅了邊界交接處的死黑斷層。
* **渲染管線資料解耦 (Data-Layer Decoupling)**：貪婪網格化 `greedy.rs` 在抓取相鄰區塊的光照資訊時，直接與 `WorldManager` 溝通取得資料層的數據，徹底跳脫對 `Query<&mut Chunk>` 的依賴，大幅提升多執行緒安全度與程式碼的執行效能。
* **太陽直射阻斷與植被陰影 (Direct Sunlight Blocking & Shadow Casting)**：優化了區塊初始化光照的垂直掃描邏輯。天空光線向正下方（-Y）傳播時，若遭遇樹葉 (`OakLeaves`) 等動態生成的實體方塊，即使高度大於地形基準線 (`max_surface_y`)，也會剛性觸發 `is_blocked` 阻斷。這使得樹冠正下方的空氣格失去 15 級特權，並無縫交由 BFS 演算法從周遭側面吸取光線（如 14、13 級），完美實現 Minecraft 的自然樹影衰減。
* **動態晝夜交替與雙軌光照連動 (Day-Night Cycle & Dual Lighting)**：實作了全域 `DayNightCycle` 資源，系統會根據 24 小時時鐘動態計算出 0.05~1.0 的 `sky_factor` 並推播至 GPU (`EnvironmentUniform`)。在片元著色器 (Fragment Shader) 中，GPU 即時計算 `max(sky_light * sky_factor, block_light)`，達成晝夜交替時【網格完全不需重新烘焙】，且夜間火把方塊光 100% 恆定高亮！
* **動態 UI 高亮與非滿版特殊幾何 (UI Rendering & Special Geometries)**：貪婪網格生成解耦了 `is_solid` 與面剔除的依賴。為非實心發光體追加了獨立幾何管線，如 `BlockType::Torch` 採用了 GPU 頂點程序化變形 (GPU Morphing)。透過推送方塊基礎整數座標，並由 Shader 中的 `@builtin(vertex_index)` 與 `face_id` 即時解算出 6x6x10 的非滿版長條比例。同時針對原版 `16x16` 火把貼圖精準映射 UV (U: 0.4375~0.5625, V: 0.375~1.0)，並利用 `flow_vector` 將牆面火把即時傾斜 15 度貼壁，並將底座向內嵌入 0.08 單位達成完美嵌合。Raycast 判定擴展至 `is_torch()` 以支援準星瞄準與挖除互動。
* **火把特化物件系統 (Torch Specialized Object System)**：`BlockType` 提供 `get_aabb_offsets()` 方法回傳自訂包圍盒，且具備**朝向感知 (Direction-Aware)** 能力，讓鋼絲選中邊框動態貼合牆面傾斜火把。玩家的瞄準與破壞實作了 **微型物件射線檢測 (Sub-Block Raycast Precision)**，射線會精確比對 AABB 交點，若未擊中火把本體則會穿透至後方牆壁。火把頂點的 `block_lights` 強制鎖定為 15 級滿亮，並在地形貪婪網格生成時透過嚴格判斷 `is_opaque()` 來**禁用火把與玻璃的 AO 陰影投射**，避免在其後方牆面產生突兀的暗角。每個面生成 12 Quad 雙面幾何 (Double-Sided)，確保任何角度皆無缺面穿幫。
* **平滑光照連續性校驗與 UI 預覽網格對齊 (Smooth Lighting Gradient Merging & Unified UI Meshing)**：在貪婪網格的 `can_merge` 邏輯中，針對發光體注入了針對 `block_lights` 的四角平滑連貫性與雙維度梯度校驗，杜絕了在夜間因天空光無梯度而導致不同火把光強的網格遭粗暴合併的現象，讓火把光能在周圍地形上呈現極致順滑的光照漸層 (Smooth Lighting)。此外，在 UI 物品欄的 `build_single_voxel_mesh` 生成時，徹底重構並與區塊貪婪網格的切線空間 (u, v) 及 CCW 繞行方向剛性統一，杜絕了舊手寫頂點表中 +X 面背面剔除漏面（導致原木等方塊右側面破洞並透視到底部）的缺陷；火把預覽更注入 12 Quad 雙面幾何與滿載 `block_light(15)`，使得 UI 在任何環境與視角下均保持完美無死角立體顯色。
* **光照即時廣播 (Runtime Lighting Remesh Broadcast)**：`set_block_global` 在執行完 BFS 光照泛洪後，會對所有被波及而標記為 dirty 的區塊統一設置 `is_lighting_ready = true`，確保 `mesh_dirty_chunks` 系統的 3x3 鄰居光照完工鎖不會阻塞這些區塊的即時重烘焙，實現放置/破壞火把時光照零延遲亮起。
* **四階段正交光照更新與光源動態消除 (Four-Phase Orthogonal Lighting Updates & Light Source Removal)**：徹底摒棄了舊有按「是否為空氣方塊」粗暴二分的分支邏輯，將 `set_block_global` 升級為四大正交階段：(1) 舊發光體消除：當破壞或覆蓋發光方塊 (如火把) 時，觸發雙向 BFS 佇列精準消除舊光場，杜絕光源殘留；(2) 固體遮蔽更新：當放置不透明方塊時阻斷天空光與方塊光；(3) 固體破壞湧入：當挖開不透明方塊時，允許周遭天空光與方塊光泛洪湧入；(4) 新光源擴散：當放置新發光體時擴散新光場。同時光照 BFS 傳播全面依據 `!is_opaque()` 判定，允許玻璃與火把等透明方塊正確穿透光線。
* **UI 網格緩存與查詢解耦 (Hotbar Mesh Caching & Change Detection)**：將快捷列 `update_hotbar_ui` 的觸發條件由 `Changed<Player>` 解耦為 `Changed<Inventory>`，消除每幀滑鼠轉動相機時重複執行 UI 系統的開銷；並在 `UiPreviewMesh` 組件引入 `current_block` 狀態緩存，僅在槽位方塊類型變更時重新建立 Mesh，徹底消滅每幀重複調用 `meshes.add` 所造成的嚴重內存溢出與 GPU 網格資產洩漏。
## 8. 🌫️ 動態環境霧化與視覺包覆 (Dynamic Environment & Fog Alignment)
* **視線亮度感知與晝夜雙層背景同步 (Eye-Light & Day-Night Sky Sync)**：遊戲實作了 `update_dynamic_environment` 系統。白天天空呈現蔚藍色，天黑時則平滑過渡至**深邃夜空藍 (Midnight Blue)**。同時每幀根據玩家眼部的光照數據（`eye_light`），動態插值（Lerp）出合適的環境色，確保玩家從地表潛入洞穴時，背景顏色能從星空或藍天滑順地過渡至帶有微弱環境光的深灰色，徹底消除畫面突變的生硬感。
* **相機遠剪裁面動態對齊與原生迷霧阻斷 (Far Clip Alignment & Fog Falloff)**：將相機的 `far` 剪裁面與渲染視距 (`render_distance`) 動態剛性鎖死為 `max_distance + 64.0`，賦予幾何體充裕的深層演算空間。同時結合 Bevy 原生的 `FogSettings` 實施黃金比例漸變：在 `max_distance * 0.75` 處柔和起霧，並在 `max_distance - 8.0` 處完全阻斷。這將地圖加載邊界完美遮蔽，達成了無瑕疵的超遠景深包覆。
* **著色器端貪婪矩形幾何微外推 (Quad Edge Padding in Shader)**：廢棄了傳統消耗龐大算力的 CPU 端「頂點焊接」與錯誤的「法線外推」機制。在 GPU 的 `voxel.wgsl` 頂點著色器中，利用 `@builtin(vertex_index)` 計算出該頂點於 Greedy Quad 中的 2D 角落象限（如 BL, BR, TR, TL），並根據其所屬面 (`face_id`) 的切線 (Tangent) 與副切線 (Bitangent) 軸向，將網格邊緣向平面外側精準擴張 `0.0005` 格。此舉在物理底層促使相鄰的面片產生微觀交織重疊，以零效能折損的代價徹底絕殺了 T-Junction 所引發的靜態藍點縫隙與漏光破綻。
* **自定義屬性佈局與智能透明裁切 (Strict Attribute Layout & Alpha Masking)**：採用客製化 VoxelMaterial 管線，將頂點屬性極限壓縮至 `@location(0) packed_data` 與 `@location(1) flow_vector` 雙插槽。固體方塊的渲染模式啟用了 `AlphaMode::Mask(0.5)`，這不僅維持了貪婪網格的高效深度剔除，還允許玻璃 (`Glass`) 等自帶透明孔洞的方塊展現出完美的穿透感。並且在 Shader 中對特定紋理（如 Grass、OakLeaves）即時施加物理的自然綠色渲染 (Foliage Tint)，並對玻璃施加微量淡藍高光，極大豐富了生態與建材視覺。

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
* **流體雙軌光照平滑採樣與日夜動態同步 (Fluid Dual-Light Averaging & Day-Night Sync)**：修復了 `push_fluid_quad` 頂點光照打包時將天空光粗暴複製並寫入方塊光槽位的致命缺陷。全面重構為雙軌平滑採樣：頂點周遭 4 格分別獨立採集天空光與方塊光並平均，精確寫入 `packed_data` 的 `[28..32]` (天空光) 與 `[24..28]` (方塊光) 槽位。使得水體在夜間隨 `env.sky_factor` 自動深邃變暗，杜絕夜晚水面發光如同夜光塑料的穿幫，同時正確響應周遭火把的暖色照明。

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
## 最近更新紀錄
- **10/2 物理與視覺沉浸深度收尾 (Physical & Visual Immersion System)**:
  - **劈砍缺口 (Chop Notches)**：`EnvironmentUniform` 擴充 `mining_data: Vec4`，WGSL 頂點著色器實裝依據 `mining_progress: 0.0~1.0` 即時動態內縮之 V 型斧砍缺口幾何形變；
  - **倒木實體化與體素撞地碎裂 (Dynamic Falling Tree & Ground Contact Detection)**：大樹以底座為支點受重力力矩角加速度傾倒，每影格進行體素接觸判定；碰觸固體斜坡、地形或達到 88 度傾角時即刻碎裂，沿倒木分佈在地面生成 3D 物理掉落物實體 (`ItemDrop`)；
  - **倒木寫實重擊傷害與擊退 (Falling Tree Crush Damage & Knockback)**：倒木砸中站於傾倒方向下方的玩家施加 35 點鈍擊傷害、向上與後推強烈擊退衝量，並引發 trauma 0.9 劇烈相機震顫；安全距離 (12m 內) 周遭玩家同步感受地面轟鳴震顫；
  - **相機震顫系統 (Screen Shake Trauma System)**：新增 `ScreenShake` 資源，於視角結算中套用非線性平方衰減之高頻微幅震顫；在地質坍塌 1.2 秒預警期間週期性注入天花板微震顫，坍塌瞬間爆發強震；
  - **3D 物理掉落物與 Q 鍵丟棄 (3D ItemDrop & Drop Keybind)**：實裝獨立 `ItemDrop` 實體，具備真實重力、固體地面碰撞、懸浮緩慢自轉、1.8m 磁吸入包與 300s 超時銷毀；玩家按 Q 鍵可手動丟棄選中槽位物品。
  - **全挖掘方塊全面實體化與開局鐵斧配發**：初始快捷列第 8 格配置鐵斧 (`IronAxe`)；全方塊挖掘破壞（草地、石頭、火把等）全面升級為 3D 物理掉落物 (`ItemDrop`)，採集彈射立體方塊並磁吸入包；未達工具等級採伐時提供直觀反饋提示。

- **11/7 木棍雙面 Quad 幾何、切削系統視覺引導與背包方塊圖示全面普及**:
  - **物品掉落物純水平雙面 Quad (`greedy.rs`)**：物品掉落物廢除 4 個垂直側面，僅生成頂面 (+Y) 與底面 (-Y) 雙面 Quad，徹底剷除側面拉伸造成的「鋸齒階梯/肋骨狀」畸變；
  - **切削系統 (Knapping) 視覺指引與進度追蹤 (`src/ui/knapping.rs`)**：
    - 消滅介面所有中文豆腐塊亂碼；
    - 5x5 網格實裝「目標斧刃輪廓指示」：14 格斧刃核心格標註高亮鋼青色邊框，11 格待敲除石片標註微暗紅邊框與紅色 `x` 標記；
    - 即時進度追蹤 (`Progress: 0/11 Flakes Chipped`)，點擊敲碎廢料時 `x` 標記自動消除，若誤敲核心本體即時警示 `Ruined!`；
  - **背包 36 格方塊 2D 貼圖全數載入 (`src/ui/hud.rs`)**：
    - `load_from_zip` 擴充支援 `textures/block/`；
    - 原木 (`OakLog`)、火把 (`Torch`)、泥土 (`Dirt`)、石頭 (`Stone`)、沙子 (`Sand`)、樹葉 (`OakLeaves`)、玻璃 (`Glass`) 等全數註冊 2D 貼圖圖示；
    - 背包所有方塊告別英文字縮寫，全面呈現高解析原版方塊圖標！

- **11/7 掉落物 Alpha Discard 鏤空、背包 UI 升級與初始燧石獲取閉環**:
  - **著色器 Alpha Discard 剔除黑邊 (`voxel.wgsl`)**：在片元著色器加入 `if env.is_fluid == 0u && tex_color.a < 0.1 { discard; }`，徹底根除 2D 物品貼圖（木棒、燧石、斧頭）透明背景被繪製為黑色的重大破綻；
  - **Dropz 級擬真平躺 1:1 比例 (`src/item/mod.rs`)**：木棒、燧石、各階斧頭與工具調整為自然平躺比例 (`Vec3::new(0.36, 0.08, 0.36)`)，伏地半高精確維持 0.04，長寬 1:1 正方無拉伸，掉落在草地上呈現真實自然的立體樹枝與工具外觀；
  - **背包 UI 全面升級與消滅亂碼 (`src/ui/inventory.rs`)**：
    - 全面採用現代遊戲英文標題與提示（`INVENTORY`, `MAIN STORAGE`, `HOTBAR`, `[E] / [ESC] Close`），徹底消滅 Bevy 缺字導致的豆腐塊 `` 亂碼；
    - 36 格全面接入 `ItemIconRegistry` 物品 2D 圖示（Icon），無圖示方塊回退簡短名稱，右下角顯示堆疊數量，底部動態顯示綠/黃/紅耐久度進度條；
    - 即時懸停（Hover Tooltip）與手持（Carried）資訊列，提示物品全名、數量與精確耐久度；
  - **開局配備燧石與野外淘石機制 (`src/player/mod.rs`)**：
    - 初始快捷列第 0 格配置燧石 4 塊 (`Flint x4`)，第 1 格配置木棒 4 根 (`Stick x4`)，玩家進遊戲第一時間即可實測右鍵 5x5 Knapping 切削工藝；
    - 挖掘泥土（Dirt）與草地（Grass）具備 15% 機率翻找淘出燧石，水岸礫石（Gravel）100% 掉落燧石，徹底打通空手野外探索與燧石獲取鏈路。

- **11/7 原始擊石切削系統 (Knapping) 與燧石斧組裝 (Flint Knapping & Tool Assembly)**:
  - **5x5 擊石切削工藝介面 (`src/ui/knapping.rs`)**：
    - 手持 2 塊燧石 (`ItemType::Flint >= 2`) 右鍵點擊開啟 5x5 Knapping UI；
    - 游標無縫解鎖釋放，背景相機視角凍結；
    - 點擊四周石塊逐格敲除，噴發深灰石屑與微弱火花粒子反饋；
    - 吻合經典斧刃配方 (`FLINT_AXE_HEAD_PATTERN`) 瞬間爆發金黃火花，扣除手持 2 塊燧石，產出「燧石斧刃 (`ItemType::FlintAxeHead`)」；
    - 按 Esc 或切削完成時自動關閉介面並重新置中鎖定游標；
  - **手持斧刃組裝燧石斧 (`src/player/mod.rs`)**：
    - 手持 `FlintAxeHead` 右鍵點擊，消耗背包 1 根木棒 (`ItemType::Stick`)，成功組裝出具備木級採伐等級 (`ToolTier::Wood`)、耐久度 45 的「燧石斧 (`ItemType::FlintAxe`)」；
    - 若缺少木棒則貼心打印引導提示；
    - 徹底打通「空手打樹葉拿木棒 + 挖礫石拿燧石 -> 雙燧石敲出斧刃 -> 組裝燧石斧 -> 砍倒大樹收穫原木」的生存玩法第一閉環！
  - **掉落物生成閃白與材質系統強化**：
    - 修正掉落物生成瞬間全亮（15, 15）閃白問題：`spawn_item_drop` 接收世界指標並於生成瞬間直接採樣環境光；
    - 擴充頂點打包位元（`0x1F`，支援 0..31 層紋理陣列）；
    - 註冊木棒、燧石、煤炭、鐵錠、各階斧頭掉落物 3D 材質與 HUD / 背包 2D Icon（優先從 `resource_pack.zip` 動態讀取）；
  - **單元測試全數通過**：
    - 新增 `test_knapping_pattern_matching`、`test_flint_axe_and_head_registry`、`test_flint_axe_can_harvest_oak_log`，全專案 56 項測試 100% 通過。

- **11/7 Dropz 級寫實物理掉落物系統 (Dropz Realistic Physics & Lighting Item Drops)**:
  - **物品物理屬性完全註冊表數據驅動 (Data-Driven Item Physics Registry)**：
    - 在 `ItemDefinition` 中引入 `physics: ItemPhysicsProperties`，徹底廢除硬編碼 match 判定；
    - 支援每種物品/方塊自由配置 3D 渲染比例 (`scale`)、伏地高度 (`half_height`)、平躺姿態 (`flat_resting`)、水體浮力加速度 (`buoyancy`) 與夜間自體發光 (`is_emissive`)；
    - `spawn_item_drop` 與 `update_item_drops` 全面切換為依賴註冊表，新增任何方塊或物品皆可自由設定是否在水上漂浮、落地橫躺或發光；
  - **環境光動態採樣與網格頂點更新**：
    - 徹底根除掉落物在黑夜自發光像 100W 燈泡的嚴重瑕疵（修復原先 `build_single_voxel_mesh` 將 `block_lights` 硬編碼為 15 的問題）；
    - 掉落物跨越體素時自動向 `WorldManager` 採樣真實世界的 `sky_light` 與 `block_light`（自發光物品除外）；
    - 在 `src/render/greedy.rs` 新增 `update_single_voxel_mesh_light`，直接透過位元遮罩覆寫頂點 packed data 高 8 位元，零內存分配極致高效；
  - **寫實伏地平躺 (Dropz Flat Resting) 與質心旋轉軸修正**：
    - 透過父子實體層級注入 `-0.5 * scale` 幾何偏移，將體素 `[0, 1]` 頂點網格原點精確對齊實體中心，徹底根治物品以角頂點為軸心偏心旋轉的缺陷，所有空中翻滾與地面偏轉均完美繞質心旋轉；
    - 木棒 (`ItemType::Stick`) 與工具類物品落地後立即呈 90 度橫躺於地面（Y 軸貼合 1.04），方塊物品穩固坐落於地面（Y 軸貼合 1.14），停止自轉與浮空；
    - 空中拋射與下落時呈現自然 3D 翻滾，著地後平穩停止；
  - **零閃爍穩固地面檢測 (Zero-Flicker Ground Resting Defense)**：
    - 根除落地後因體素邊界浮點數向下取整跳變導致的 60Hz 著地/浮空交替振盪（消滅雙重殘影/虛影破綻）；
    - 著地後主動向下探測 0.05 格支撐體素，唯有支撐方塊被破壞或滑出邊緣時方才轉為下墜，確保持續穩定休止；
  - **全向 3D 體素 AABB 碰撞盒與彈壁消解**：
    - 掉落物具備半徑 0.15m、高 0.08m~0.28m 的 3D AABB 碰撞盒；
    - 嚴密檢測 X / Z 水平碰撞，遭遇土牆、岩壁時以 0.3 彈性係數反彈並減速，徹底杜絕旋轉穿模或半截卡進土牆內的視覺破綻；
    - Y 軸地面碰撞偵測，高速撞地產生真實小彈跳，低速則穩固貼地；
  - **水體浮力與阻力 (Fluid Physics / Buoyancy)**：
    - 掉落物進入水體 (`world.get_fluid_global > 0`) 時套用水阻減速；
    - 依註冊表 `buoyancy > 0` 判定，原木 (`OakLog`)、木棒 (`Stick`)、樹葉 (`OakLeaves`) 穩妥浮於水面自然微幅漂動；
    - 石頭、礦石等重物 (`buoyancy < 0`) 則以緩慢速度下沉到底部；
  - **倒木傾倒動畫與視錐剔除徹底修復 (Falling Tree Topple Animation & Frustum Fix)**：
    - 為 `FallingTree` 子實體複合網格注入 `NoFrustumCulling`，根治因 Packed Format 虛擬頂點 `[0, 0, 0]` 導致 Bevy Frustum Culling 在仰視時直接將整棵樹剔除不可見的隱形 Bug；
    - 修正地面碰撞判定：取消樹葉 (`leaf_offsets`) 於 11 度時刮蹭地形提早爆炸的錯誤邏輯，改為倒塌超過 45 度 (`angle > 0.80 rad`) 時僅針對堅硬原木軀幹 (`log_offsets`) 判定地面碰撞，完整展現流暢震撼的伐木傾倒弧線；
  - **底座原木連帶傾倒與剛體轉動慣量動態角加速度 (Tree Base Inclusion & Rigid Body Angular Dynamics)**：
    - **底座原木連帶傾倒**：手持斧頭伐木時，被砍伐的那一格原木 (`broken_pos`) 不再提前在腳邊掉落，而是作為倒木最底層納入 `FallingTree` 實體；傾倒旋轉支點 (`pivot`) 精準對齊該方塊底面中心 (`broken_pos.as_vec3() + Vec3::new(0.5, 0.0, 0.5)`)，整棵大樹拔地傾倒，倒地撞擊碎裂時連同底座原木一併爆散噴出掉落物；若是砍伐孤立單立方塊原木，則直接原處生成掉落物；
    - **動態剛體角加速度**：依據均勻剛體桿繞底旋轉公式 $\alpha = \frac{\tau}{I} = \frac{3g}{2L} \sin\theta$，樹木傾倒角加速度與樹高 $L$ 成反比，並結合樹冠樹葉空氣阻抗阻尼 (`leaf_count`)。矮樹倒伏俐落迅速（~1.0s），參天巨木倒塌磅礡沉重（~1.8~2.2s），徹底告別死板固定數值！
  - **平滑磁吸拾取**：
    - 玩家靠近 1.8m 時喚醒平滑磁吸，滑順飛向玩家並納入背包；背包滿或拾取延遲冷卻中則保持伏地平躺。

- **10/1 硬核真實生存四大里程碑系統 (Hardcore Realistic Survival 4-Step Milestone)**:
  - **第一步：樹木重力與採伐工具門檻 (Tree Felling & Gravity System)**:
    - `ToolTier::Wood` 斧頭採伐門檻：原木 (`OakLog`) 只有在手持斧頭時方可採收，空手敲擊不掉落原木；敲擊樹葉掉落木棒 (`ItemType::Stick`)；
    - Timber 斧頭連鎖倒塌：手持斧頭砍伐樹幹底座時觸發 Timber 連鎖倒塌，整棵樹幹被砍伐收穫，樹冠凋落掉落木棒並爆發噴散樹葉碎屑粒子；
    - 空手重力滑落：空手破壞樹幹底座時，上方懸空樹幹在重力作用下整齊下落 1 格，徹底消滅原版不科學的浮空樹木！
  - **第二步：真黑極限暗夜與火把燃盡/燧石擊打引燃 (Pitch-Black Darkness & Torch Burnout / Flint Sparks)**:
    - `voxel.wgsl` 著色器移除人工環境光偏移，`min_ambient_light` 設為 0.0，無光照環境達成 100% 真黑深邃體驗；
    - 火把 180s 燃盡生命週期 (`TorchBurnManager`)：放置的火把在 180 秒後燃盡熄滅並轉為空氣，透過 BFS 光照消除管線精準抹除方塊光並噴散黑煙碎屑；
    - 燧石擊打火花與引燃合成：破壞礫石掉落燧石 (`ItemType::Flint`)，右鍵擊打燧石可迸發火花並消耗木棒與煤炭手工引燃合成 4 支火把。
  - **第三步：玩家生理狀態機與 HUD 體徵數值條 (Player Physiology State Machine & Vitals HUD)**:
    - 實裝 `PlayerVitals` 元件，管理 Health (100)、Stamina (100)、Thirst (100)、Body Temperature (37.0°C) 與 Wetness (0~100%)；
    - 疾跑與跳躍消耗體力，體力耗盡時自動降速為常規走速；
    - 背包動態負重計算 (`calculate_inventory_encumbrance`)，攜帶沉重礦石與原木減緩移動速度並增加耐力消耗；
    - 涉水增加潮濕度，潮濕與暗夜引發失溫（體溫低於 35°C 扣除生命值），靠近火把 (光照 >= 9) 可迅速烘烤回暖與蒸發水分；
    - 口渴度持續消耗，右鍵水源（或浸於水中）飲水補充水份；
    - Hotbar 上方掛載專屬生命 (紅)、體力 (黃)、口渴 (藍) 三色動態狀態條與體溫/潮濕度數值反饋，F3 除錯界面同步顯示生理指標。
  - **第四步：地質結構穩定度、礦坑木支架與覆岩應力坍塌物理 (Geological Structural Integrity & Cave-in Physics)**:
    - 地底覆岩應力 (Y <= 64)：在缺乏支護的情況下採礦會觸發頂板岩層失穩；
    - 礦坑木支架 (`OakLog`) 支護半徑：任何 `OakLog` 木支架可在水平 4 格、垂直 4 格範圍內提供安全結構牽引；
    - 自穩跨度檢測：窄於 5 格 (半徑 <= 2) 的天然岩柱/岩壁具備自穩拱效應，開掘大於 4 格開闊空腔時頂板失穩坍塌；
    - 局部落石坍塌：失穩頂板崩解為空氣，碎石沿重力落於地面形成礫石堆 (Gravel)，對正下方玩家施加毀滅性落石重擊傷害。

- **9/30 焦點驅動游標鎖定架構 (Focus-Driven Cursor Locking Architecture)**:
  - 視窗未獲焦點時絕對不鎖鼠標：移除 `WindowPlugin` 與 `setup_player` 中的過早鎖定，視窗啟動預設標準游標；在 `toggle_grab_cursor` 實施第一鐵律：當 `!window.focused` 或剛失去焦點時，游標無條件維持自由釋放 (`CursorGrabMode::None`, `visible = true`)，絕不在非聚焦狀態定住或約束鼠標；
  - 聚焦時才置中鎖定：當視窗獲得焦點（開局視窗首次聚焦就緒或切回視窗觸發 `WindowFocused { focused: true }`）且背包未開啟時，游標精確定位至視窗中央 `(width/2, height/2)` 並鎖定 (`CursorGrabMode::Locked`, `visible = false`)，同時觸發 `CursorJustLocked` 防誤觸閘門；
  - 按 ESC 鍵釋放游標 (`CursorGrabMode::None`, `visible = true`)，點擊視窗重新置中鎖定；關閉背包時同步檢測 `window.focused`，保證未獲焦點時絕不誤鎖。

- **9/30 第三梯隊 (Tier 3) 3D DDA 快速體素遍歷演算法、區塊網格記憶體零拷貝提取優化與冗餘代碼清理**:
  - 實裝 Amanatides & Woo 3D 快速體素遍歷演算法 (Fast Voxel Traversal / 3D DDA)，徹底淘汰 0.05m 固定步長線性搜尋；結合 Slab Method 達成火把等局部子幾何體 (Sub-block AABB) 精準求交，並產出精確表面法線與相鄰放置座標；全面套用於 `player_interaction`、`draw_target_block_highlight` 與 `update_crosshair`；
  - 將資料層 `ChunkEntry` 緩衝區 (`buffer`, `light_buffer`, `fluid_buffer`) 全面升級託管為 `std::sync::Arc`，網格生成提取鄰居時使用 `Arc::clone`，將每區塊高達 2.16MB 的堆疊記憶體複製與分配開銷降為 **0 位元組 (0 Bytes)**，徹底杜絕內存震盪；修改時採用 Copy-on-Write (`Arc::make_mut`) 確保無鎖極致線程安全；
  - 徹底剔除已無未來發展空間之 `get_block_global_mut`、`Chunk::get_block`、`Chunk::is_pure_air`，清理全專案編譯器警告，維持全專案 `cargo check` 0 警告、0 錯誤與全單元測試 100% 通過。

- **9/30 第二梯隊 (Tier 2) 方塊採掘進度體系、36格全背包管理介面與採掘碎屑粒子**:
  - 實裝長按左鍵累積採掘進度與方塊硬度/工具效率乘數公式，空手無法採掘高階岩石礦物且速度受懲罰，匹配工具享受 efficiency 倍率加速；
  - 實裝瞄準方塊進度漸層轉紅與破裂內部框回饋，火把等 0 硬度方塊維持即時敲除體驗；
  - 建立輕量化 3D 體素碎屑粒子系統 (`src/render/particles.rs`)，挖掘過程飛散細微碎屑，破裂瞬間爆發噴散 14 顆對應材質代表色之物理彈性粒子；
  - 建立 36 格全功能背包介面 (`src/ui/inventory.rs`)，透過 `E` 鍵 / `Escape` 呼出/關閉並解鎖游標，支援拿起/放下/堆疊/置換物品與無損歸還防線。

- **9/30 第一梯隊 (Tier 1) 著色器距離迷霧、水流沖垮火把物理與天光穿透透明方塊**:
  - 在 `voxel.wgsl` 與 `EnvironmentUniform` 中實作了 16-byte std140 對齊之片元距離迷霧，與相機背景色及視距平滑線性混合，消滅遠處地形幾何生硬切斷；
  - 實裝水流沖刷火把物理，水流擴散至火把位置時自動拆除火把、聯動消除光源並將火把掉落物自動返還玩家背包，流體探測不再將火把誤判為實心擋水壩；
  - 重構天空光初始化與傳播邏輯，改採 `!block.is_opaque()`，使陽光可穿透玻璃與火把直射與漫射；
  - 補強 `.gitignore`，嚴密阻絕敏感憑證與私密資料。

- **9/30 水中物理與跳躍系統工業級重構**: 徹底解決了水面彈簧床拋射與貼牆火箭電梯等非物理問題；實裝水面平穩踩水（限速 1.0 m/s）、單次登岸翻越閘門（6.38 m/s）、穩態瀑布攀爬（3.0 m/s）與下蹲鍵 (ControlLeft/C) 垂直下潛，操作手感與物理真實性達到商業級水準。

- **9/30 夜晚水面發光缺陷修復**: 修正了 `greedy.rs` 中 `push_fluid_quad` 誤將天空光寫入方塊光（Block Light）導致水體在黑夜恆定 15 滿亮發光的 Bug；實裝頂點雙軌光照平滑採樣，確保夜晚水體隨 `sky_factor` 柔和變暗且正常接收火把光。

- **9/30 火把光源殘留修復與 UI 網格記憶體洩漏根治**: 修復了打掉火把後光源永久殘留的重大缺陷，將方塊變更重構為四階段正交光照更新；修復了天空光在玻璃與火把等透明方塊上的傳播阻斷；根治了 Hotbar 每幀滑鼠移動時重複調用 `meshes.add` 造成的記憶體洩漏；並修正了玻璃方塊破壞時掉落物掉落表匹配問題。

- **8/8 通用 Entity 物理組件化**: 將原先綁定於 Player 的運動學、流體感測與 Swept AABB 碰撞消解解耦，封裝為獨立的 PhysicsPlugin。透過 RigidBody、AabbCollider、Velocity 等元件，實現支援多軸同步消解、Safewalk 防跌落與旁觀者模式切換之通用 ECS 物理架構。

- **8/8 重大 Bug 熱修復**: 修正了 greedy.rs 流體網格打包資料（Packed Data）的 Bit Shifts 偏差與溢位，徹底消滅了水體渲染產生的 GPU 頂點爆炸尖刺；並重構了 WorldManager::set_fluid_global 的 Chunk 邊界連動髒污標記機制，確保流體動態即時烘焙；同時擴增了 FluidSensor 的全範圍 AABB 掃描，還原了精確的流體浮力互動。

- **10/2 樹冠正統 Fancy Leaves 濃密渲染、背包全方塊 3D 等軸預覽統一與游標跟手拖拽系統 (Fancy Leaves, Unified 3D Isometric Inventory & Cursor-Following Dragging)**:
  - **樹冠正統 Fancy Leaves 雙向切面掃描 (Two-Way Directional Slice Pass)**:
    - 徹底根治樹葉在開啟 Alpha Discard 後內部被貪婪網格粗暴剔除導致的「空心薄殼」問題；
    - 在 `src/render/greedy.rs` 將切片掃描重構為 `for normal_dir in [1i32, -1i32]` 雙向獨立掃描：當相鄰兩個方塊均為樹葉 (`OakLeaves`) 時，+1 與 -1 側均生成獨立面片，透過鏤空孔隙層層疊疊展示內部深邃繁茂枝葉，還原 Minecraft 原版 Fancy Graphics 視覺效果；
    - 同時維持玻璃內部相鄰面的剔除 (`b0 == b1 && b0 == Glass`) 與非樹葉固體方塊的面剔除，效能與視覺兼具。
  - **大背包與 Hotbar 全面統一 3D 等角等軸立體方塊展示 (Unified 3D Isometric Block Previews)**:
    - 在 `src/ui/hud.rs` 啟動階段為遊戲中全部 11 種方塊（Stone, Dirt, Grass, OakLog, OakLeaves, Sand, Gravel, CoalOre, IronOre, Glass, Torch）配置等角 3D 離屏渲染攝影棚 (`Block3dPreviewRegistry`)；
    - 透過 `.chain()` 依序傳入 `setup_item_icons`，將 11 種方塊的 3D 渲染圖注入 `ItemIconRegistry.icons`；
    - 大背包（Inventory）36 格自動享有與 Hotbar 完全一致的精美 3D 等角方塊，格子圖標升級為 32x32 滿幅顯示，同時自動隱藏粗糙英文簡寫文字；材料與工具則維持官方 2D 圖示。
  - **滑鼠拖拽即時跟手系統 (Real-time Cursor-Following Dragging)**:
    - 實裝 `FloatingCarriedItemNode`，包含 `FloatingCarriedIconNode` 與 `FloatingCarriedCountNode`，層級為 `ZIndex::Global(100)`；
    - 在 `update_inventory_ui` 中即時取得 `window.cursor_position()`，手持物品時無延遲將 3D/2D 圖示與數量文字精確定位至游標尖端 (`pos.x - 18.0, pos.y - 18.0`)；
    - 全節點配置 `FocusPolicy::Pass`，滑鼠點擊判定無阻礙穿透至底層背包格子按鈕，達成流暢跟手拖拽與精準放置交互；手持為空或關閉背包時自動隱藏並歸還。

- **10/2 倒木動態雙軌環境光照取樣與著色器 WGSL 整數平坦插值規範修復 (Falling Tree Dynamic Lighting & WGSL Flat Interpolation Fix)**:
  - **倒木在黑夜倒下變黑剪影根本原因剖析**:
    1. 在複合網格生成器 `build_compound_voxel_mesh` 中頂點方塊光硬編碼為 0 (`block_lights = [0, 0, 0, 0]`)，倒木在黑夜中即使身處火把旁也無法接收任何火把光源，受夜間天空光衰減因子 `sky_factor = 0.05` 壓制，表面亮度暴跌至 ~1.4% (RGB: 9, 5, 2) 呈焦炭剪影；
    2. 生成倒木實體時未採樣世界環境光照；
    3. 著色器 `assets/shaders/voxel.wgsl` 中頂點與片元傳遞的整數 `texture_index: u32` 漏標 `@interpolate(flat)`，違反 WebGPU/WGSL 規範造成插值未定義行為。
  - **核心修復實裝**:
    1. 在 `src/render/greedy.rs` 新增 `build_compound_voxel_mesh_with_light(blocks, sky_light, block_light)`，支援傳入自訂天空光與方塊光；
    2. 在 `src/world/tree.rs` 的 `handle_tree_break` 中，即時向世界資料層 `WorldManager` 採樣砍伐點 `broken_pos` 與 6 個鄰居方塊的雙軌世界光照（天空光與火把方塊光，天空光保底 12 級），注入倒木初始網格；
    3. 在 `update_falling_trees` 物理系統中，每影格動態追蹤倒木中心體素的世界光照，當環境光階層變更時透過 `update_single_voxel_mesh_light` 動態更新倒木網格頂點光照，倒木隨旋轉傾倒滾入或遠離火把光源時自然滑順變更亮度；
    4. 在 `assets/shaders/voxel.wgsl` 為 `VertexOutput` 與 `FragmentInput` 中的 `texture_index: u32` 明確標註 `@interpolate(flat)`，全單元測試 61 項 100% 通過。
  - **單元測試全數驗證通過**:
    - 新增 `test_fancy_leaves_preserves_faces_between_adjacent_leaves`（驗證相鄰樹葉保留雙向內部面片與外圍貪婪合併）；
    - 新增 `test_all_11_preview_blocks_have_item_types` 與 `test_carried_item_return_and_slot_swap`；
    - 全專案 59 項單元測試 100% 通過，`cargo check` 0 警告、0 錯誤。

- **10/2 原始石器切削 5x5 多配方系統、自適應雙層材質圖標與全英文介面語意優化 (Primitive Knapping Multi-Recipe System, Adaptive Dual-Layer Icons & English-First Polish)**:
  - **解決開局生存閉環斷層 (Survival Loop Closure)**:
    - 原版遊戲若無預設初始工具，玩家無法開採石頭方塊（鵝卵石）。新增「燧石鎬 (`FlintPickaxe`)」與「燧石鏟 (`FlintShovel`)」、「燧石刀 (`FlintKnife`)」及其對應工具頭部（`FlintPickaxeHead`, `FlintShovelHead`, `FlintKnifeBlade`）；
    - `FlintPickaxe` 設定為 `ToolType::Pickaxe` + `ToolTier::Wood`，效率 2.0，耐久度 40，可合法採收 `BlockType::Stone`（掉落鵝卵石）與 `BlockType::CoalOre`（掉落煤炭），形成「徒手淘燧石 -> 5x5 切削燧石鎬刃 -> 摘樹葉取木棒組裝 -> 開採石頭與煤礦 -> 邁向石器時代」的完整原始生存閉環。
  - **自適應雙層材質架構與專屬像素美術 (Dual-Layer Adaptive Icons & Visual Separation)**:
    - 徹底終結「天然燧石與加工燧石長一樣」、「燧石工具與石製工具長一樣」的視覺混淆；
    - 在 `src/ui/hud.rs` 實裝 16x16 專屬像素圖標生成器：
      - 4 種 Tool Heads 擁有獨立打製刃面與凹槽反光；
      - 4 種 Flint Tools 擁有深黑曜色燧石刃面＋粗藤條/植物纖維捆紮（Twine Wrap）＋粗糙木柄，與淺灰石器徹底分流；
    - 優先讀取資源包同名 2D png，若資源包未提供則無縫落入此程序化像素美術，未來更換任何原版材質包絕不破圖、不留後患；
    - 3D 掉落物在 `src/render/greedy.rs` 中將燧石工具與頭部精確綁定至燧石材質層（Layer 15），與石製工具（Layer 19）在三維空間亦徹底分流。
  - **5x5 擊石切削多配方體系與動態幽靈導引 (5x5 Multi-Recipe Knapping & Ghost Guides)**:
    - 擴充 `KnappingRecipe` 枚舉（`AxeHead`, `PickaxeHead`, `ShovelHead`, `KnifeBlade`），建立各自符合石器力學的 5x5 布林形狀矩陣；
    - 切削介面提供 `[Axe Head]`、`[Pickaxe Head]`、`[Shovel Head]`、`[Knife Blade]` 即時標籤切換按鈕；
    - 實裝動態幽靈導引：刃身格點以鋼青色邊框高亮，廢料石屑以軟紅邊框標記 `x`；點擊噴發石屑粒子，形狀吻合時噴發金黃火花，扣除 2 塊燧石並發放對應工具頭部。
  - **手持通用組裝機制 (In-Hand Universal Assembly)**:
    - 在 `src/player/mod.rs` 實裝通用工具組裝：手持任一 Flint Tool Head 右鍵時，檢測背包是否有 `ItemType::Stick`；
    - 若有木棒：消耗 1 根木棒與 1 個 Tool Head，生成滿耐久工具，噴散金黃與木質粒子並輸出操作指引；
    - 若無木棒：提示玩家「缺少木棒！請破壞樹葉收集 1 根木棒來組裝工具！」。
  - **全方位英文介面與提示字串純化 (English-First Polish)**:
    - 將 `player/mod.rs`、`ui/debug.rs`、`world/collapse.rs`、`ui/knapping.rs`、`item/mod.rs` 中所有終端機與在屏提示全面英文化，徹底根除 Bevy 預設字體缺少 CJK 字形導致的 `□□□` 缺失字形問題，奠定國際化受眾基礎。
  - **驗證成果**:
    - 全專案 61 項單元測試 100% 通過（包含多配方矩針測試、石器工具登錄與採伐能力驗證），`cargo check` 0 警告、0 錯誤。

- **10/3 大陸架平滑 C1 樣條、深淵黑霧解耦與自然水文重構 (Continental Shelf C1 Spline, Abyss Fog & Natural Hydrology)**:
  - **海岸階梯人工斷層根治**: 大陸度指標在海深 0~20 米處採用全域 C1 連續樣條過渡，形成平緩金色沙灘與淺海大陸棚，消滅人工垂直斷崖。
  - **深淵與洞穴深黑迷霧解耦**: 僅在海平面以下封閉地底洞穴將迷霧色彩平滑過渡為深淵黑霧 (`vec3(0.001, 0.002, 0.005)`)，消除俯瞰洞穴冒藍天的違和感；高空與地表 100% 沐浴於純淨大氣天光散射下。
  - **乾燥地下洞穴解救**: 陸地高山與丘陵下方的 3D 溶洞 100% 保持乾燥通風空氣，可自由行走與採礦。

- **10/5 Voxy 風格微體素階梯 LOD (Voxelized Stepped LOD)、半透明遠洋與海底連續延展重構 (Voxy-Style Voxel LOD, Underwater Seabed & Transparent Ocean)**:
  - **傳統 2.5D 高度圖三角面與「黏土饅頭山」的根本治本**:
    - **問題根因**：舊版 LOD 採用頂點斜向三角面連接（Smooth Triangles），導致遠山呈現圓弧平滑的「綠色黏土假山」與「饅頭山」，與近處 90 度直角體素階梯嚴重風格撕裂；
    - **重構方案**：全面進階為 **Voxy 級純體素階梯 (Voxelized Stepped LOD)**！每個 8x8 米柱體由 1 個水平頂面 (Top Quad, 法線嚴格為 `[0, 1, 0]`) 與四周 4 向垂直側壁 (Side Quads, 法線嚴格為 `[±1, 0, 0]` 與 `[0, 0, ±1]`) 構成；
    - **視覺成果**：遠山徹底保留經典 Minecraft 90 度體素階梯輪廓，遠遠望去 100% 符合體素世界觀，徹底根除任何圓滑塑膠黏土感！
  - **水下海床連續延伸與半透明遠海水體 (`LODWaterMaterial`)**:
    - **水面斷層根因**：舊版只要高度低於海平面直接在 Y=256 鋪不透明純藍板並抹平海床，導致近處透明海水邊界與遠處不透明藍板形成刀切色塊斷崖；
    - **重構方案**：
      1. 地形網格忠實生成水下海床、沙灘與海底斷崖，不強行截斷；
      2. 水面單獨生成一層半透明網格（`AlphaMode::Blend`），掛載專屬 `LODWaterMaterial`，著色器套用 `[0.15, 0.35, 0.75, 0.60]` 水藍半透明色彩；
      3. 玩家在高處俯瞰大洋時，遠海水面半透明透出水下連續延伸的海床沙礫，近海與遠洋水色深度一致，刀切斷層徹底消滅！
  - **遠景樹冠微結構 (Canopy Micro-Voxels)**:
    - 採樣樹木分佈程序化 Hash，在森林與丘陵區域自動將微體素柱體抬高 5 格並著色為深綠色樹冠頂面與側壁 (`RGB(0.18, 0.36, 0.14)`)，遠山不再是光禿禿的光頭山，完美延續近景森林。
  - **體素方塊與遠景 LOD 大氣迷霧全域同步 (Unified Atmospheric Fog)**:
    - 迷霧起始距離由 450m 提前至 380m，在近景體素方塊結束 (480m) 前已溫和柔化 7%，近景方塊不再於 472m 驟然泛白；
    - 近景方塊與遠景 LOD 共享完全相同的迷霧曲線 (380m~1800m) 與背景色，銜接交界無縫融合。
  - **驗證成果**:
    - 全專案 72/72 項單元測試 100% 通過（耗時 0.04 秒），`cargo check` 0 警告、0 錯誤；
    - Release 二進位檔案編譯完成，維持 160+~200+ FPS 高效能。
