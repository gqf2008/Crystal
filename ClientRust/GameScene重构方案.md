# GameScene 架构重构方案

## 一、当前问题分析

### 1.1 当前架构缺陷
```rust
// ❌ 当前 GameScene (game_scene.rs line 24-96)
pub struct GameScene {
    pub user: Option<UserObject>,
    pub objects: HashMap<u32, Box<dyn DrawableMapObject>>,
    pub map_control: Option<map_control::MapControl>,
    pub tile_texture_manager: RefCell<TileTextureManager>,
    // ... 碎片化的数据管理
}
```

**问题:**
1. **GameScene 不是中枢**: 只是场景切换器,不管理游戏状态
2. **MapControl 分离**: 地图渲染逻辑散落在独立模块
3. **无 UI 控件树**: 缺少统一的 UI 管理框架
4. **数据碎片化**: 物品/技能/任务/社交分散在各处
5. **渲染混乱**: 没有清晰的 DrawControl → base.DrawControl 层次

### 1.2 C# GameScene.cs 架构优势

```csharp
// ✅ C# GameScene 架构 (GameScene.cs line 27-1061)
public sealed class GameScene : MirScene {
    // ========== 中枢数据管理 (line 27-240) ==========
    public static UserObject User;
    public static MapObject SelectedCell;
    public static Dictionary<uint, MapObject> Objects;
    
    // 物品与经济
    public static MirItemCell[] Inventory;      // [46]
    public static MirItemCell[] Storage;        // [80]
    public static MirItemCell[] BeltIdx;        // [6]
    public static uint Gold, Credit;
    
    // 技能与 Buff
    public static List<ClientMagic> Magics;
    public static List<Buff> Buffs;
    
    // 任务系统
    public static List<QuestInfo> Quests;
    public static List<QuestTracker> TrackedQuests;
    
    // 社交系统
    public static List<Friend> Friends;
    public static List<Relationship> Relationships;
    public static GuildObject Guild;
    
    // ========== MapControl 嵌套类 (line 10209-11241) ==========
    public sealed class MapControl : MirControl {
        // 地图数据
        private M2CellInfo[,] M2CellInfo;
        private Texture FloorTexture;
        
        // 渲染方法
        private void DrawFloor();      // 静态地表烘焙
        private void DrawBackground(); // 远景背景
        private void DrawObjects();    // 动态对象+叠加
        private void DrawLights();     // 光照遮罩
        
        public override void DrawControl() {
            CreateTexture();  // 调用上述方法
        }
    }
    
    // ========== 主场景方法 (line 1062-1382) ==========
    protected internal override void DrawControl() {
        if (MapControl != null)
            MapControl.DrawControl();  // Phase 1: 地图与对象
        base.DrawControl();            // Phase 2: UI 控件树
        // Phase 3: 顶层元素 (拖拽物品/提示文本)
    }
    
    protected override void ProcessPacket(Packet p) {
        // 巨大 switch 处理所有服务器消息
        switch ((ServerPacketIds)p.Index) {
            case S.MapInformation: ...
            case S.ObjectPlayer: ...
            case S.UserLocation: ...
            // ... 200+ case
        }
    }
}
```

**优势:**
1. **中枢化**: GameScene 是游戏运行期唯一数据源
2. **模块清晰**: MapControl 负责地图,UI 控件树负责界面
3. **渲染分层**: MapControl.DrawControl → base.DrawControl → 顶层叠加
4. **网络集中**: ProcessPacket 统一处理所有服务器消息

---

## 二、重构目标架构

### 2.1 核心原则
1. **Rust GameScene 必须镜像 C# GameScene.cs**
2. **controls 模块 = MirControls (UI框架)**
3. **objects 模块 = MapObjects (游戏对象)**
4. **渲染和网络已完成,只需集成**

### 2.2 目标结构

```rust
// ✅ 目标架构: ClientRust/src/scenes/game_scene.rs
pub struct GameScene {
    // ==================== 中枢数据管理 ====================
    // 对应 C# line 27-240
    
    // 玩家与英雄
    user: Option<UserObject>,           // C#: public static UserObject User
    hero: Option<HeroObject>,           // C#: public static HeroObject Hero
    selected_cell: Option<MapObject>,   // C#: public static MapObject SelectedCell
    
    // 对象管理 (游戏逻辑用)
    objects: HashMap<u32, Box<dyn MapObject>>,  // C#: Dictionary<uint, MapObject> Objects
    
    // 物品与经济
    inventory: [Option<ItemCell>; 46],   // C#: MirItemCell[] Inventory
    storage: [Option<ItemCell>; 80],     // C#: MirItemCell[] Storage
    belt: [Option<ItemCell>; 6],         // C#: MirItemCell[] BeltIdx
    gold: u32,                           // C#: uint Gold
    credit: u32,                         // C#: uint Credit
    
    // 装备槽
    equipment: [Option<ItemCell>; 14],   // C#: MirItemCell[] Equipment
    
    // 技能与 Buff
    magics: Vec<ClientMagic>,            // C#: List<ClientMagic> Magics
    buffs: Vec<Buff>,                    // C#: List<Buff> Buffs
    
    // 任务系统
    quests: Vec<QuestInfo>,              // C#: List<QuestInfo> Quests
    tracked_quests: Vec<QuestTracker>,   // C#: List<QuestTracker> TrackedQuests
    
    // 社交系统
    friends: Vec<Friend>,                // C#: List<Friend> Friends
    relationships: Vec<Relationship>,    // C#: List<Relationship> Relationships
    guild: Option<GuildObject>,          // C#: GuildObject Guild
    
    // 邮件系统
    mail_list: Vec<Mail>,                // C#: List<ClientMail> Mail
    
    // 排行榜
    rankings: Vec<Rank>,                 // C#: List<Rank> Rankings
    
    // ==================== MapControl (嵌套功能) ====================
    // 对应 C# MapControl 嵌套类 (line 10209-11241)
    map_control: MapControl,             // 地图渲染与交互
    
    // ==================== UI 控件树 ====================
    // 对应 C# line 242-459 (所有 Dialog 初始化)
    controls: Vec<Box<dyn Control>>,     // 子控件列表 (通过 MirControl 框架管理)
    
    // 主要 UI 对话框
    main_dialog: Option<MainDialog>,
    chat_dialog: Option<ChatDialog>,
    inventory_dialog: Option<InventoryDialog>,
    character_dialog: Option<CharacterDialog>,
    storage_dialog: Option<StorageDialog>,
    trade_dialog: Option<TradeDialog>,
    skill_bar_dialog: Option<SkillBarDialog>,
    // ... 40+ dialogs
    
    // ==================== 输入与渲染 ====================
    // 对应 C# line 461-1061
    mouse_location: Point,
    selected_item: Option<ItemCell>,     // 拖拽中的物品
    output_messages: VecDeque<OutputMessage>,  // 左上角滚动提示
    
    // 模式与状态
    attack_mode: AttackMode,             // C#: AMode
    pet_mode: PetMode,                   // C#: PMode
    lights: LightSetting,                // C#: Lights
    
    // 时间戳
    move_time: i64,
    attack_time: i64,
    spell_time: i64,
    pickup_time: i64,
}

// ==================== MapControl 结构 ====================
// 对应 C# MapControl 嵌套类
pub struct MapControl {
    // 地图数据
    cell_info: Vec<Vec<M2CellInfo>>,     // C#: M2CellInfo[,] M2CellInfo
    width: i32,
    height: i32,
    
    // 离屏缓存
    floor_texture: Option<Texture>,      // C#: Texture FloorTexture
    floor_valid: bool,                   // C#: bool FloorValid
    
    // 瓦片纹理管理
    tile_manager: TileTextureManager,    // 已存在
    
    // 视口与相机
    offset_x: i32,                       // C#: OffSetX
    offset_y: i32,                       // C#: OffSetY
    
    // 天气与粒子
    weather: Weather,                    // C#: Weather flags
    particle_engines: Vec<ParticleEngine>,
    
    // 灯光
    lights: LightSetting,                // C#: LightSetting
    map_dark_light: i32,                 // C#: MapDarkLight
}

impl MapControl {
    // ========== 渲染方法 (对应 C# line 10328-10803) ==========
    
    /// 主渲染入口 (C# CreateTexture line 10333)
    pub fn draw_control(&mut self) {
        self.create_texture();
    }
    
    /// 创建离屏纹理并渲染所有层 (C# line 10333-10418)
    fn create_texture(&mut self) {
        if !self.floor_valid {
            self.draw_floor();  // 烘焙静态地表
        }
        
        // 绘制到离屏纹理
        self.draw_background();      // 远景
        // 叠加 floor_texture
        self.draw_objects();         // 动态层+对象
        self.draw_weather();         // 天气粒子
        self.draw_lights();          // 光照遮罩
    }
    
    /// 绘制静态地表到 FloorTexture (C# DrawFloor line 10442-10544)
    fn draw_floor(&mut self) {
        // 1) Back 层 (BackImage/BackIndex)
        // 2) Middle 层静态部分 (规则尺寸,无动画)
        // 3) Front 层静态部分 (规则尺寸,无动画,简易门偏移)
    }
    
    /// 绘制远景背景 (C# DrawBackground line 10546-10566)
    fn draw_background(&mut self) {
        // 根据地图名称选择背景图
    }
    
    /// 绘制动态层与对象 (C# DrawObjects line 10568-10803)
    fn draw_objects(&mut self) {
        // 1) 背景特效 (Effects where DrawBehind)
        // 2) 尸体 (M2CellInfo[x,y].DrawDeadObjects)
        // 3) Shanda 瓦片动画层 (TileAnimationImage)
        // 4) Middle 层动态/混合/不规则
        // 5) Front 层动态/混合/门动画
        // 6) 对象本体 (M2CellInfo[x,y].DrawObjects)  ← 核心!
        // 7) User 额外渲染 (半透明/高亮)
        // 8) 前景特效 (Effects where !DrawBehind)
        // 9) 对象叠加 (名字/血条/聊天/伤害)
    }
    
    /// 绘制光照遮罩 (C# DrawLights line 10805-10859)
    fn draw_lights(&mut self, setting: LightSetting) {
        // 夜晚/黎明/黄昏/毒盲减光
    }
}

impl GameScene {
    // ========== 场景生命周期 (对应 C# line 242-459) ==========
    
    pub fn new() -> Self {
        // 创建所有 UI 对话框
        // 设置 Parent = self 建立控件树
    }
    
    pub fn initialize(&mut self) {
        // 加载地图资源
        // 初始化 MapControl
    }
    
    // ========== 主渲染方法 (对应 C# DrawControl line 1062-1146) ==========
    
    pub fn draw_control(&mut self, canvas: &mut Canvas) {
        // Phase 1: 地图与对象
        self.map_control.draw_control();
        
        // Phase 2: UI 控件树 (base.DrawControl)
        for control in &mut self.controls {
            if control.is_really_visible() {
                control.draw(canvas);
            }
        }
        
        // Phase 3: 顶层元素
        self.draw_dragging_item(canvas);   // 拖拽物品图标
        self.draw_output_messages(canvas); // 左上角提示
    }
    
    // ========== 输入处理 (对应 C# line 1148-1382) ==========
    
    pub fn on_key_down(&mut self, key: KeyCode) {
        // 映射 KeybindOptions 到功能
        // 技能释放/面板切换/拾取/交易等
    }
    
    pub fn on_mouse_move(&mut self, location: Point) {
        // 更新鼠标位置
        // 更新物品悬浮提示
    }
    
    pub fn on_mouse_down(&mut self, button: MouseButton, location: Point) {
        // 委托给 MapControl 或 UI 控件
    }
    
    // ========== 网络协议处理 (对应 C# ProcessPacket line 1384-5976) ==========
    
    pub fn process_packet(&mut self, packet: ServerPacket) {
        match packet {
            ServerPacket::MapInformation(p) => self.handle_map_information(p),
            ServerPacket::ObjectPlayer(p) => self.handle_object_player(p),
            ServerPacket::UserLocation(p) => self.handle_user_location(p),
            ServerPacket::NewItemInfo(p) => self.handle_new_item_info(p),
            ServerPacket::BaseStatsInfo(p) => self.handle_base_stats_info(p),
            // ... 200+ handlers
        }
    }
    
    // ========== 游戏逻辑 (对应 C# line 5978-10207) ==========
    
    pub fn use_spell(&mut self, spell: Spell) {
        // 技能释放逻辑
    }
    
    pub fn pickup_item(&mut self) {
        // 拾取物品
    }
    
    pub fn drop_item(&mut self, item_cell: ItemCell) {
        // 丢弃物品
    }
    
    pub fn add_output_message(&mut self, message: String, message_type: OutputMessageType) {
        // 添加屏幕提示
    }
}
```

---

## 三、分阶段重构计划

### Phase 0: 准备工作 (已完成 ✅)
- ✅ controls 模块已有 Control trait
- ✅ objects 模块已有 MapObject/UserObject/MonsterObject 等
- ✅ 网络模块已完成 (mir2_shared packets)
- ✅ 渲染模块已完成 (ggez)

### Phase 1: 重构 GameScene 数据结构 (1-2天)

#### 1.1 创建新的 GameScene 结构
```rust
// ClientRust/src/scenes/game_scene_v2.rs
pub struct GameScene {
    // 中枢数据 (参考上面目标结构)
    user: Option<UserObject>,
    inventory: [Option<ItemCell>; 46],
    magics: Vec<ClientMagic>,
    // ...
    
    map_control: MapControl,
    controls: Vec<Box<dyn Control>>,
}
```

#### 1.2 定义 MapControl 结构
```rust
// ClientRust/src/scenes/game_scene/map_control.rs
pub struct MapControl {
    cell_info: Vec<Vec<M2CellInfo>>,
    floor_texture: Option<Texture>,
    tile_manager: TileTextureManager,
    // ...
}
```

#### 1.3 迁移现有数据
- 从 `game_scene.rs` 迁移数据字段到新结构
- 保留旧文件作为 `game_scene_old.rs` 备份

### Phase 2: 实现 MapControl 渲染 (2-3天)

#### 2.1 实现六层渲染
```rust
impl MapControl {
    fn draw_floor(&mut self) {
        // 从现有 draw() 方法提取 BackImage/MiddleImage/FrontImage 逻辑
    }
    
    fn draw_objects(&mut self) {
        // 提取动态层渲染逻辑
        // 集成 M2CellInfo[x,y].DrawObjects() 模式
    }
}
```

#### 2.2 对象渲染集成
```rust
// 每个格子维护对象列表
pub struct M2CellInfo {
    pub back_image: u32,
    pub middle_image: u32,
    pub front_image: u32,
    pub cell_objects: Vec<u32>,  // 对象 ID 列表
}

impl M2CellInfo {
    pub fn draw_objects(&self, objects: &HashMap<u32, Box<dyn MapObject>>) {
        // 按深度排序并绘制对象
    }
}
```

### Phase 3: UI 控件树集成 (2-3天)

#### 3.1 扩展 Control trait
```rust
// src/controls/control.rs
pub trait Control {
    fn draw(&mut self, canvas: &mut Canvas);
    fn update(&mut self, dt: f32);
    fn on_mouse_down(&mut self, button: MouseButton, location: Point) -> bool;
    // ... 其他事件
    
    // 子控件管理
    fn add_child(&mut self, child: Box<dyn Control>);
    fn children(&self) -> &[Box<dyn Control>];
}
```

#### 3.2 实现基础控件
```rust
// src/controls/mir_control.rs - Base control
pub struct MirControl {
    location: Point,
    size: Size,
    visible: bool,
    children: Vec<Box<dyn Control>>,
}

// src/controls/mir_label.rs - Label
pub struct MirLabel {
    base: MirControl,
    text: String,
    font: Font,
}

// src/controls/mir_image_control.rs - Image
pub struct MirImageControl {
    base: MirControl,
    texture: Texture,
}

// src/controls/mir_button.rs - Button
pub struct MirButton {
    base: MirControl,
    on_click: Option<Box<dyn Fn()>>,
}
```

#### 3.3 实现主要对话框
```rust
// src/scenes/game_scene/dialogs/main_dialog.rs
pub struct MainDialog {
    base: MirControl,
    hp_bar: MirImageControl,
    mp_bar: MirImageControl,
    exp_bar: MirImageControl,
    level_label: MirLabel,
    // ...
}

// src/scenes/game_scene/dialogs/inventory_dialog.rs
pub struct InventoryDialog {
    base: MirControl,
    grid: Vec<MirItemCell>,  // 46 个物品格子
    gold_label: MirLabel,
    // ...
}
```

### Phase 4: 网络集成 (1-2天)

#### 4.1 实现 ProcessPacket
```rust
impl GameScene {
    pub fn process_packet(&mut self, packet: ServerPacket) {
        match packet {
            ServerPacket::MapInformation(p) => {
                self.map_control.load_map(&p.file_name);
                self.map_control.set_title(&p.title);
                // ...
            },
            ServerPacket::ObjectPlayer(p) => {
                let player = UserObject::from_packet(p);
                self.objects.insert(player.object_id, Box::new(player));
            },
            // ... 200+ handlers
        }
    }
}
```

#### 4.2 客户端消息发送
```rust
impl GameScene {
    pub fn send_move(&mut self, direction: MirDirection) {
        let packet = ClientPacket::Walk { direction };
        self.network.send(packet);
    }
    
    pub fn send_attack(&mut self, direction: MirDirection) {
        let packet = ClientPacket::Attack { direction, spell: Spell::None };
        self.network.send(packet);
    }
}
```

### Phase 5: 输入处理 (1天)

#### 5.1 键盘映射
```rust
impl GameScene {
    pub fn on_key_down(&mut self, key: KeyCode) {
        match key {
            KeyCode::F1 => self.toggle_inventory(),
            KeyCode::F2 => self.toggle_character(),
            KeyCode::Z => self.pickup_item(),
            // ... 按 KeybindOptions 映射
        }
    }
}
```

#### 5.2 鼠标处理
```rust
impl GameScene {
    pub fn on_mouse_down(&mut self, button: MouseButton, location: Point) {
        // 1) 检查 UI 控件命中
        for control in &mut self.controls {
            if control.contains(location) {
                if control.on_mouse_down(button, location) {
                    return; // 控件消费了事件
                }
            }
        }
        
        // 2) 委托给 MapControl
        self.map_control.on_mouse_down(button, location);
    }
}
```

### Phase 6: 测试与优化 (1-2天)

#### 6.1 单元测试
- MapControl 渲染正确性
- 物品拖拽逻辑
- 技能释放冷却

#### 6.2 性能优化
- FloorTexture 缓存生效
- 对象剔除 (视口外不渲染)
- 控件树裁剪

---

## 四、关键技术点

### 4.1 控件树遍历
```rust
// C# 的 base.DrawControl() 等价实现
impl GameScene {
    fn draw_controls(&mut self, canvas: &mut Canvas) {
        for control in &mut self.controls {
            if control.is_really_visible() {
                control.draw(canvas);
                // 递归绘制子控件 (在 Control::draw 内部处理)
            }
        }
    }
}
```

### 4.2 对象深度排序
```rust
impl M2CellInfo {
    pub fn draw_objects(&self, objects: &HashMap<u32, Box<dyn MapObject>>, canvas: &mut Canvas) {
        let mut draw_list: Vec<&Box<dyn MapObject>> = self.cell_objects.iter()
            .filter_map(|id| objects.get(id))
            .collect();
        
        // 按 Y 坐标排序 (后面的在前面)
        draw_list.sort_by_key(|obj| obj.map_location().y);
        
        for obj in draw_list {
            obj.draw(canvas);
        }
    }
}
```

### 4.3 物品拖拽
```rust
impl GameScene {
    pub fn start_drag_item(&mut self, item: ItemCell, source: ItemSource) {
        self.selected_item = Some(item);
        self.drag_source = Some(source);
    }
    
    pub fn drop_item(&mut self, target: Point) {
        if let Some(item) = self.selected_item.take() {
            // 检查目标位置 (背包/装备/地面)
            if let Some(target_slot) = self.get_slot_at(target) {
                self.move_item(item, target_slot);
            } else {
                self.drop_item_to_ground(item);
            }
        }
    }
}
```

---

## 五、文件组织结构

```
ClientRust/src/scenes/
├── game_scene.rs                # 新 GameScene 主文件
├── game_scene/
│   ├── mod.rs
│   ├── map_control.rs           # MapControl 实现
│   ├── tile_texture_manager.rs  # 已存在
│   ├── cell_info.rs             # M2CellInfo 定义
│   ├── dialogs/                 # 所有对话框
│   │   ├── main_dialog.rs
│   │   ├── chat_dialog.rs
│   │   ├── inventory_dialog.rs
│   │   ├── character_dialog.rs
│   │   ├── storage_dialog.rs
│   │   ├── trade_dialog.rs
│   │   └── ... (40+ dialogs)
│   └── handlers/                # 网络包处理
│       ├── object_handlers.rs   # ObjectPlayer/Monster/NPC
│       ├── item_handlers.rs     # NewItemInfo/UserStorage
│       ├── skill_handlers.rs    # MagicLevels/ObjectMagic
│       └── ... (按功能分类)

ClientRust/src/controls/
├── mod.rs
├── control.rs                   # Control trait (已存在)
├── mir_control.rs               # 基础控件实现
├── mir_label.rs
├── mir_image_control.rs
├── mir_button.rs
├── mir_item_cell.rs             # 物品格子控件
└── ... (更多控件)

ClientRust/src/objects/
├── mod.rs
├── map_object.rs                # 已存在
├── user_object.rs               # 已存在
├── monster_object.rs            # 已存在
└── ... (已完成)
```

---

## 六、迁移检查清单

### 数据迁移
- [ ] User/Hero 对象
- [ ] Objects 字典
- [ ] Inventory[46]
- [ ] Storage[80]
- [ ] Equipment[14]
- [ ] Magics/Buffs
- [ ] Quests/Friends/Guild
- [ ] Mail/Rankings

### MapControl 功能
- [ ] DrawFloor() - 静态地表烘焙
- [ ] DrawBackground() - 远景背景
- [ ] DrawObjects() - 六层渲染
- [ ] DrawLights() - 光照遮罩
- [ ] M2CellInfo.DrawObjects() - 对象本体
- [ ] 天气粒子系统
- [ ] 门动画系统

### UI 控件
- [ ] Control trait 完整实现
- [ ] MirControl 基类
- [ ] MainDialog (主界面)
- [ ] ChatDialog (聊天)
- [ ] InventoryDialog (背包)
- [ ] CharacterDialog (角色)
- [ ] 40+ 其他对话框

### 网络处理
- [ ] ProcessPacket switch
- [ ] MapInformation handler
- [ ] ObjectPlayer/Monster/NPC handlers
- [ ] Item/Storage/Trade handlers
- [ ] Skill/Buff handlers
- [ ] Quest/Social handlers
- [ ] 200+ 协议处理

### 输入处理
- [ ] KeybindOptions 映射
- [ ] 鼠标点击/拖拽
- [ ] 物品拾取/丢弃
- [ ] 技能释放
- [ ] UI 交互

---

## 七、风险与注意事项

### 7.1 性能风险
- **控件树遍历**: 40+ 对话框每帧遍历可能有性能问题
- **解决方案**: 使用脏标记,只重绘变化的控件

### 7.2 兼容性风险
- **网络协议**: 必须与服务器完全兼容
- **解决方案**: 复用 mir2_shared::packets,已验证

### 7.3 渲染风险
- **FloorTexture 缓存**: 地图滚动时需正确失效
- **解决方案**: 参考 C# FloorValid 逻辑

---

## 八、总结

### 当前状态
- ✅ objects 模块完成
- ✅ controls 模块基础完成
- ✅ 网络/渲染已完成
- ❌ GameScene 架构不符合 C# 设计

### 重构后
- ✅ GameScene 成为游戏中枢
- ✅ MapControl 负责地图渲染
- ✅ UI 控件树统一管理
- ✅ 网络协议集中处理
- ✅ 易于维护和调试

### 预计工时
- Phase 1 (数据结构): 1-2天
- Phase 2 (MapControl): 2-3天
- Phase 3 (UI 控件): 2-3天
- Phase 4 (网络集成): 1-2天
- Phase 5 (输入处理): 1天
- Phase 6 (测试优化): 1-2天
- **总计: 8-13天**

### 下一步行动
1. 创建 `game_scene_v2.rs` 新文件
2. 定义完整的 GameScene 结构体
3. 实现 MapControl 基础框架
4. 逐步迁移现有功能

---

**备注**: 此方案严格遵循 C# GameScene.cs 架构,确保 Rust 实现与原版行为一致。
