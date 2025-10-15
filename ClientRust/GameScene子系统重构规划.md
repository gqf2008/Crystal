# GameScene 子系统重构规划

> **目的**: 将当前臃肿的 GameScene (2400行) 拆分为清晰的子系统架构,避免重蹈 C# 版本的覆辙 (13605行)

移动平滑 - FSM 插值 ✅
摄像机平滑 - lerp 插值 ✅
方向平滑 - 逐步旋转 ✅
动画流畅 - 帧更新正常 ✅
---

## 📊 C# 版本问题分析

### 当前状态
- **总行数**: 13605 行
- **UI 对话框**: 50+ 个直接作为字段
- **静态数据列表**: 10+ 个静态字段 (ItemInfoList, QuestInfoList, Storage 等)
- **时间戳字段**: 20+ 个 (MoveTime, AttackTime, PickUpTime 等)
- **核心方法**:
  - `ProcessPacket()`: 300+ 行的巨大 switch-case
  - `Process()`: 处理所有对象更新、输入、鼠标检测
  - `CreateTexture()`: 8 步渲染管线 (背景→地板→对象→粒子→光照)
  - `GameScene_KeyDown()`: 处理所有快捷键

### 主要问题
1. **职责不清**: UI、逻辑、渲染、网络全部混在一起
2. **全局状态**: 大量静态字段导致测试困难
3. **循环依赖**: 各模块互相引用
4. **难以维护**: 添加新功能需要修改多处代码
5. **性能隐患**: 没有明确的更新顺序和优先级

---

## 🏗️ Rust 版本子系统设计

### 设计原则
✅ **单一职责**: 每个子系统只负责一件事  
✅ **明确边界**: 通过 trait 定义接口,避免循环依赖  
✅ **实例状态**: 避免全局静态变量  
✅ **组合优于继承**: 使用 Rust 的 trait 和泛型  
✅ **数据驱动**: 配置和逻辑分离  

---

## 📦 子系统划分

### 1. **InputSystem (输入系统)**
**职责**: 统一处理鼠标、键盘输入

```rust
// src/systems/input_system.rs
pub struct InputSystem {
    mouse_state: MouseState,
    keyboard_state: KeyboardState,
    keybind_config: KeybindConfig,
}

pub struct MouseState {
    position: Point,              // 当前鼠标位置
    left_button: ButtonState,     // 左键状态
    right_button: ButtonState,    // 右键状态
    scroll_delta: f32,            // 滚轮增量
}

pub enum ButtonState {
    Released,
    Pressed,
    Held,
}

impl InputSystem {
    /// 每帧主动读取输入状态
    pub fn update(&mut self, ctx: &Context) {
        self.mouse_state.position = ctx.mouse.position().into();
        self.mouse_state.left_button = self.get_button_state(MouseButton::Left, ctx);
        self.mouse_state.right_button = self.get_button_state(MouseButton::Right, ctx);
    }
    
    /// 检查快捷键是否触发
    pub fn check_keybind(&self, action: GameAction) -> bool {
        if let Some(key) = self.keybind_config.get_key(action) {
            self.keyboard_state.is_key_down(key)
        } else {
            false
        }
    }
    
    /// 获取鼠标状态 (只读)
    pub fn mouse(&self) -> &MouseState { &self.mouse_state }
}
```

**优势**:
- ✅ 统一的输入处理入口
- ✅ 支持按键映射和组合键
- ✅ 可以记录输入历史 (用于回放、作弊检测)

---

### 2. **ObjectManager (对象管理系统)**
**职责**: 管理所有地图对象 (玩家、怪物、NPC、掉落物)

```rust
// src/systems/object_manager.rs
pub struct ObjectManager {
    objects: HashMap<u32, MapObject>,     // 所有对象
    user: Option<UserObject>,             // 玩家对象
    hero: Option<HeroObject>,             // 英雄对象
    
    // 空间索引 (加速鼠标碰撞检测)
    spatial_index: Grid<Vec<u32>>,        // 格子 -> 对象ID列表
    
    // 视野裁剪
    visible_objects: Vec<u32>,            // 当前可见的对象ID
}

impl ObjectManager {
    /// 添加新对象到场景
    pub fn add_object(&mut self, obj: MapObject) {
        let id = obj.id();
        let pos = obj.position();
        
        // 添加到对象字典
        self.objects.insert(id, obj);
        
        // 更新空间索引
        self.spatial_index.insert(pos, id);
    }
    
    /// 移除对象
    pub fn remove_object(&mut self, id: u32) {
        if let Some(obj) = self.objects.remove(&id) {
            let pos = obj.position();
            self.spatial_index.remove(pos, id);
        }
    }
    
    /// 更新所有对象 (移动、动画、AI)
    pub fn update(&mut self, delta_time: f32) {
        // 1. 更新玩家 (优先级最高)
        if let Some(user) = &mut self.user {
            user.update(delta_time);
        }
        
        // 2. 更新英雄
        if let Some(hero) = &mut self.hero {
            hero.update(delta_time);
        }
        
        // 3. 更新其他对象 (怪物、NPC、掉落物)
        for obj in self.objects.values_mut() {
            obj.update(delta_time);
        }
        
        // 4. 更新视野裁剪 (只处理可见对象)
        self.update_visible_objects();
    }
    
    /// 鼠标拾取检测 (从鼠标位置找对象)
    pub fn pick_object_at(&self, mouse_pos: Point, camera: &Camera) -> Option<&MapObject> {
        // 1. 屏幕坐标 -> 世界坐标
        let world_pos = camera.screen_to_world(mouse_pos);
        
        // 2. 查询空间索引
        let grid_pos = world_to_grid(world_pos);
        let nearby_ids = self.spatial_index.get(grid_pos)?;
        
        // 3. 精确碰撞检测 (从上到下)
        for &id in nearby_ids.iter().rev() {
            if let Some(obj) = self.objects.get(&id) {
                if obj.contains_point(world_pos) {
                    return Some(obj);
                }
            }
        }
        
        None
    }
    
    /// 获取玩家对象
    pub fn user(&self) -> Option<&UserObject> {
        self.user.as_ref()
    }
}
```

**优势**:
- ✅ 空间索引加速鼠标检测 (O(1) 查询)
- ✅ 视野裁剪减少不必要的更新
- ✅ 统一的对象生命周期管理

---

### 3. **UIManager (UI管理系统)**
**职责**: 管理所有对话框和控件

```rust
// src/systems/ui_manager.rs
pub struct UIManager {
    dialogs: HashMap<DialogType, Box<dyn Dialog>>,
    active_dialogs: Vec<DialogType>,     // 可见对话框栈 (后进先出)
    
    hover_item: Option<UserItem>,        // 鼠标悬停的物品
    selected_item: Option<UserItem>,     // 选中/拖动的物品
}

pub enum DialogType {
    Inventory,
    Character,
    Skills,
    Chat,
    Minimap,
    NPC,
    Storage,
    // ... 50+ 对话框类型
}

pub trait Dialog {
    fn update(&mut self, input: &InputSystem, delta_time: f32);
    fn draw(&self, ctx: &mut Context);
    fn on_show(&mut self);
    fn on_hide(&mut self);
    fn handle_input(&mut self, input: &InputSystem) -> bool; // 返回是否消耗输入
}

impl UIManager {
    /// 显示对话框
    pub fn show_dialog(&mut self, dialog_type: DialogType) {
        if !self.active_dialogs.contains(&dialog_type) {
            self.active_dialogs.push(dialog_type);
            if let Some(dialog) = self.dialogs.get_mut(&dialog_type) {
                dialog.on_show();
            }
        }
    }
    
    /// 隐藏对话框
    pub fn hide_dialog(&mut self, dialog_type: DialogType) {
        if let Some(pos) = self.active_dialogs.iter().position(|&d| d == dialog_type) {
            self.active_dialogs.remove(pos);
            if let Some(dialog) = self.dialogs.get_mut(&dialog_type) {
                dialog.on_hide();
            }
        }
    }
    
    /// 更新所有对话框 (从顶层到底层)
    pub fn update(&mut self, input: &InputSystem, delta_time: f32) {
        for &dialog_type in self.active_dialogs.iter().rev() {
            if let Some(dialog) = self.dialogs.get_mut(&dialog_type) {
                dialog.update(input, delta_time);
                
                // 如果对话框消耗了输入,停止传递
                if dialog.handle_input(input) {
                    break;
                }
            }
        }
    }
    
    /// 绘制所有对话框 (从底层到顶层)
    pub fn draw(&self, ctx: &mut Context) {
        for &dialog_type in &self.active_dialogs {
            if let Some(dialog) = self.dialogs.get(&dialog_type) {
                dialog.draw(ctx);
            }
        }
    }
    
    /// ESC键关闭所有对话框
    pub fn close_all(&mut self) {
        for dialog_type in self.active_dialogs.drain(..) {
            if let Some(dialog) = self.dialogs.get_mut(&dialog_type) {
                dialog.on_hide();
            }
        }
    }
}
```

**优势**:
- ✅ 统一的对话框生命周期管理
- ✅ 输入事件按 Z-Order 传递 (顶层优先)
- ✅ 支持模态对话框和拖拽

---

### 4. **RenderingPipeline (渲染管线)**
**职责**: 分层渲染地图和对象

```rust
// src/systems/rendering_pipeline.rs
pub struct RenderingPipeline {
    map_renderer: MapRenderer,           // 地图渲染器
    effect_renderer: EffectRenderer,     // 特效渲染器
    weather_renderer: WeatherRenderer,   // 天气粒子渲染器
    light_renderer: LightRenderer,       // 光照渲染器
}

impl RenderingPipeline {
    /// 8步渲染管线 (对应 C# 的 CreateTexture)
    pub fn render(&mut self, 
                  ctx: &mut Context, 
                  objects: &ObjectManager,
                  camera: &Camera,
                  light_setting: LightSetting) -> GameResult {
        // 步骤 1: 绘制远景背景 (山脉、沙漠)
        self.draw_background(ctx, camera)?;
        
        // 步骤 2: 绘制地面瓦片 (Back/Middle/Front 三层)
        self.map_renderer.draw_floor(ctx, camera)?;
        
        // 步骤 3: 绘制动态对象 (按 Y 坐标排序)
        self.draw_objects(ctx, objects, camera)?;
        
        // 步骤 4: 绘制特效和动画
        self.effect_renderer.draw(ctx, camera)?;
        
        // 步骤 5: 绘制粒子天气 (雨雪风沙)
        self.weather_renderer.draw(ctx)?;
        
        // 步骤 6: 绘制光照遮罩 (夜晚/黄昏)
        if light_setting != LightSetting::Day {
            self.light_renderer.draw(ctx, camera, light_setting)?;
        }
        
        // 步骤 7: 绘制名字和血条
        self.draw_names(ctx, objects, camera)?;
        
        // 步骤 8: 绘制调试信息 (格子碰撞、寻路路径)
        if cfg!(debug_assertions) {
            self.draw_debug_info(ctx, camera)?;
        }
        
        Ok(())
    }
    
    /// 绘制对象 (按 Y 坐标排序,实现遮挡)
    fn draw_objects(&mut self, 
                    ctx: &mut Context, 
                    objects: &ObjectManager,
                    camera: &Camera) -> GameResult {
        // 1. 收集可见对象
        let mut visible: Vec<_> = objects.visible_objects()
            .iter()
            .filter_map(|&id| objects.get(id))
            .collect();
        
        // 2. 按 Y 坐标排序 (Y 小的先画,Y 大的后画)
        visible.sort_by_key(|obj| obj.draw_y());
        
        // 3. 绘制每个对象
        for obj in visible {
            obj.draw(ctx, camera)?;
        }
        
        Ok(())
    }
}
```

**优势**:
- ✅ 清晰的渲染步骤
- ✅ 易于添加后处理效果 (模糊、HDR、SSAO)
- ✅ 支持多线程渲染准备 (Rust 的 Send/Sync)

---

### 5. **NetworkHandler (网络处理系统)**
**职责**: 处理服务器消息并分发到子系统

```rust
// src/systems/network_handler.rs
pub struct NetworkHandler {
    packet_rx: tokio::sync::mpsc::UnboundedReceiver<ServerPacket>,
    command_tx: tokio::sync::mpsc::UnboundedSender<ClientCommand>,
    
    // 消息处理器注册表
    handlers: HashMap<PacketType, Box<dyn PacketHandler>>,
}

pub trait PacketHandler: Send + Sync {
    fn handle(&mut self, packet: ServerPacket, context: &mut GameContext);
}

impl NetworkHandler {
    /// 处理所有待处理的网络消息
    pub fn process_packets(&mut self, context: &mut GameContext) {
        // 每帧最多处理 100 个包 (防止卡顿)
        for _ in 0..100 {
            match self.packet_rx.try_recv() {
                Ok(packet) => {
                    self.dispatch_packet(packet, context);
                }
                Err(_) => break,
            }
        }
    }
    
    /// 分发消息到对应的处理器
    fn dispatch_packet(&mut self, packet: ServerPacket, context: &mut GameContext) {
        let packet_type = packet.packet_type();
        
        if let Some(handler) = self.handlers.get_mut(&packet_type) {
            handler.handle(packet, context);
        } else {
            tracing::warn!("未处理的消息类型: {:?}", packet_type);
        }
    }
    
    /// 注册消息处理器
    pub fn register_handler<H: PacketHandler + 'static>(&mut self, 
                                                         packet_type: PacketType, 
                                                         handler: H) {
        self.handlers.insert(packet_type, Box::new(handler));
    }
}

// 示例: 对象移动消息处理器
pub struct ObjectWalkHandler;
impl PacketHandler for ObjectWalkHandler {
    fn handle(&mut self, packet: ServerPacket, context: &mut GameContext) {
        if let ServerPacket::ObjectWalk(data) = packet {
            if let Some(obj) = context.objects.get_mut(data.object_id) {
                obj.walk_to(data.location, data.direction);
            }
        }
    }
}
```

**优势**:
- ✅ 解耦网络协议和游戏逻辑
- ✅ 支持热插拔处理器 (MOD 支持)
- ✅ 消息处理限流 (防止服务器攻击)

---

### 6. **EffectSystem (特效系统)**
**职责**: 管理动画、粒子、天气、声音

```rust
// src/systems/effect_system.rs
pub struct EffectSystem {
    effects: Vec<Effect>,                // 动画特效
    particles: Vec<ParticleEmitter>,     // 粒子发射器
    weather: WeatherSystem,              // 天气系统
    sound_manager: SoundManager,         // 音效管理器
}

pub enum Effect {
    Animation {
        id: u32,
        position: Point,
        animation: Animation,
        lifetime: f32,
    },
    Projectile {
        id: u32,
        start: Point,
        end: Point,
        speed: f32,
        sprite: Sprite,
    },
}

impl EffectSystem {
    /// 创建动画特效
    pub fn spawn_effect(&mut self, effect_type: EffectType, position: Point) {
        let animation = Animation::from_effect_type(effect_type);
        self.effects.push(Effect::Animation {
            id: self.next_id(),
            position,
            animation,
            lifetime: 2.0, // 2秒后自动销毁
        });
    }
    
    /// 播放音效
    pub fn play_sound(&mut self, sound_type: SoundType, position: Point) {
        // 3D音效: 根据距离和方向调整音量和声像
        self.sound_manager.play_3d(sound_type, position);
    }
    
    /// 更新所有特效
    pub fn update(&mut self, delta_time: f32) {
        // 1. 更新动画
        self.effects.retain_mut(|effect| {
            effect.update(delta_time);
            !effect.is_finished()
        });
        
        // 2. 更新粒子
        for emitter in &mut self.particles {
            emitter.update(delta_time);
        }
        
        // 3. 更新天气
        self.weather.update(delta_time);
    }
}
```

**优势**:
- ✅ 统一的特效管理
- ✅ 自动销毁过期特效 (防止内存泄漏)
- ✅ 支持 3D 音效定位

---

### 7. **DataManager (数据管理系统)**
**职责**: 缓存游戏静态数据 (物品、技能、任务、配方)

```rust
// src/systems/data_manager.rs
pub struct DataManager {
    items: HashMap<u32, ItemInfo>,           // 物品模板
    magics: HashMap<u32, MagicInfo>,         // 技能模板
    quests: HashMap<u32, ClientQuestInfo>,   // 任务模板
    recipes: HashMap<u32, RecipeInfo>,       // 配方模板
    npcs: HashMap<u32, NpcInfo>,             // NPC 模板
}

impl DataManager {
    /// 从服务器加载数据
    pub fn load_from_server(&mut self, data: InitialData) {
        self.items = data.items.into_iter().map(|i| (i.id, i)).collect();
        self.magics = data.magics.into_iter().map(|m| (m.id, m)).collect();
        // ...
    }
    
    /// 获取物品信息 (只读)
    pub fn get_item(&self, id: u32) -> Option<&ItemInfo> {
        self.items.get(&id)
    }
    
    /// 获取技能信息 (只读)
    pub fn get_magic(&self, id: u32) -> Option<&MagicInfo> {
        self.magics.get(&id)
    }
}
```

**优势**:
- ✅ 集中管理静态数据
- ✅ 避免多次从服务器请求
- ✅ 支持热重载 (开发调试)

---

## 🎯 GameScene 重构后的结构

```rust
// src/scenes/game_scene.rs (重构后: ~500 行)
pub struct GameScene {
    // ==================== 子系统 ====================
    input_system: InputSystem,           // 输入系统
    object_manager: ObjectManager,       // 对象管理
    ui_manager: UIManager,               // UI管理
    rendering_pipeline: RenderingPipeline, // 渲染管线
    network_handler: NetworkHandler,     // 网络处理
    effect_system: EffectSystem,         // 特效系统
    data_manager: DataManager,           // 数据管理
    
    // ==================== 摄像机 ====================
    camera: Camera,                      // 跟随玩家的摄像机
    
    // ==================== 场景状态 ====================
    state: GameSceneState,               // 加载状态 (Loading/Ready)
}

impl GameScene {
    /// 创建场景
    pub fn new(ctx: &mut Context) -> Self {
        Self {
            input_system: InputSystem::new(),
            object_manager: ObjectManager::new(),
            ui_manager: UIManager::new(ctx),
            rendering_pipeline: RenderingPipeline::new(ctx),
            network_handler: NetworkHandler::new(),
            effect_system: EffectSystem::new(),
            data_manager: DataManager::new(),
            camera: Camera::new(),
            state: GameSceneState::Loading,
        }
    }
    
    /// 每帧更新 (ECS风格: Systems处理Components)
    pub fn update(&mut self, ctx: &mut Context) -> GameResult {
        // 1. 更新输入系统 (读取鼠标键盘状态)
        self.input_system.update(ctx);
        
        // 2. 处理网络消息
        self.network_handler.process_packets(&mut self.create_context());
        
        // 3. 更新对象 (玩家、怪物、NPC)
        self.object_manager.update(ctx.time.delta().as_secs_f32());
        
        // 4. 更新特效
        self.effect_system.update(ctx.time.delta().as_secs_f32());
        
        // 5. 更新 UI
        self.ui_manager.update(&self.input_system, ctx.time.delta().as_secs_f32());
        
        // 6. 更新摄像机 (跟随玩家)
        if let Some(user) = self.object_manager.user() {
            self.camera.follow(user.position());
        }
        
        // 7. 处理输入动作 (移动、攻击、拾取)
        self.handle_input_actions();
        
        Ok(())
    }
    
    /// 处理输入动作
    fn handle_input_actions(&mut self) {
        let input = &self.input_system;
        
        // 鼠标右键移动
        if input.mouse().right_button == ButtonState::Pressed {
            let world_pos = self.camera.screen_to_world(input.mouse().position);
            self.move_player_to(world_pos);
        }
        
        // 鼠标左键攻击/拾取
        if input.mouse().left_button == ButtonState::Pressed {
            let mouse_pos = input.mouse().position;
            
            // 检查是否点击对象
            if let Some(obj) = self.object_manager.pick_object_at(mouse_pos, &self.camera) {
                match obj.object_type() {
                    ObjectType::Monster => self.attack_target(obj.id()),
                    ObjectType::Item => self.pickup_item(obj.id()),
                    ObjectType::Npc => self.talk_to_npc(obj.id()),
                    _ => {}
                }
            }
        }
        
        // 快捷键处理 (ESC关闭对话框)
        if input.check_keybind(GameAction::CloseAll) {
            self.ui_manager.close_all();
        }
    }
    
    /// 渲染场景
    pub fn draw(&mut self, ctx: &mut Context) -> GameResult {
        // 清空屏幕
        graphics::clear(ctx, Color::BLACK);
        
        // 执行8步渲染管线
        self.rendering_pipeline.render(
            ctx,
            &self.object_manager,
            &self.camera,
            LightSetting::Day,
        )?;
        
        // 绘制 UI
        self.ui_manager.draw(ctx)?;
        
        Ok(())
    }
    
    /// 创建游戏上下文 (用于子系统间通信)
    fn create_context(&mut self) -> GameContext {
        GameContext {
            objects: &mut self.object_manager,
            effects: &mut self.effect_system,
            ui: &mut self.ui_manager,
            data: &self.data_manager,
            camera: &self.camera,
        }
    }
}
```

---

## 📂 文件结构

```
ClientRust/
├── src/
│   ├── scenes/
│   │   └── game_scene.rs         (500行: 场景主入口,组合所有子系统)
│   │
│   ├── systems/                  (子系统目录)
│   │   ├── mod.rs
│   │   ├── input_system.rs       (输入系统: 鼠标键盘处理)
│   │   ├── object_manager.rs     (对象管理: 玩家、怪物、NPC)
│   │   ├── ui_manager.rs         (UI管理: 对话框和控件)
│   │   ├── rendering_pipeline.rs (渲染管线: 8步渲染流程)
│   │   ├── network_handler.rs    (网络处理: 消息分发)
│   │   ├── effect_system.rs      (特效系统: 动画、粒子、音效)
│   │   └── data_manager.rs       (数据管理: 静态数据缓存)
│   │
│   ├── objects/                  (对象类型)
│   │   ├── mod.rs
│   │   ├── map_object.rs         (基础对象 trait)
│   │   ├── user_object.rs        (玩家对象)
│   │   ├── monster_object.rs     (怪物对象)
│   │   ├── npc_object.rs         (NPC对象)
│   │   └── item_object.rs        (掉落物对象)
│   │
│   ├── ui/                       (UI对话框)
│   │   ├── mod.rs
│   │   ├── dialog.rs             (对话框 trait)
│   │   ├── inventory_dialog.rs   (背包对话框)
│   │   ├── character_dialog.rs   (角色对话框)
│   │   └── ...                   (50+ 对话框)
│   │
│   └── rendering/                (渲染模块)
│       ├── mod.rs
│       ├── map_renderer.rs       (地图渲染器)
│       ├── effect_renderer.rs    (特效渲染器)
│       ├── weather_renderer.rs   (天气渲染器)
│       └── light_renderer.rs     (光照渲染器)
```

---

## 🔄 数据流

```
┌─────────────┐
│   ggez      │  ← 游戏引擎事件循环
│ EventHandler│
└──────┬──────┘
       │
       ▼
┌─────────────────────────────────────┐
│         GameScene                   │
│  ┌─────────────────────────────┐   │
│  │  update()                    │   │
│  │  1. input_system.update()   │   │  ← 读取输入状态
│  │  2. network_handler.process()│  │  ← 处理网络消息
│  │  3. object_manager.update() │   │  ← 更新对象
│  │  4. effect_system.update()  │   │  ← 更新特效
│  │  5. ui_manager.update()     │   │  ← 更新UI
│  │  6. camera.follow()         │   │  ← 跟随玩家
│  │  7. handle_input_actions()  │   │  ← 处理动作
│  └─────────────────────────────┘   │
│  ┌─────────────────────────────┐   │
│  │  draw()                      │   │
│  │  1. rendering_pipeline.render()│ │  ← 8步渲染管线
│  │  2. ui_manager.draw()       │   │  ← 绘制UI
│  └─────────────────────────────┘   │
└─────────────────────────────────────┘
```

---

## ✅ 重构优势

### 1. **可维护性**
- 每个子系统职责明确,修改不影响其他模块
- 代码行数从 13605 行降到 ~500 行主文件 + 8 个子系统

### 2. **可测试性**
- 每个子系统可以独立单元测试
- 不依赖全局静态变量

### 3. **性能优化**
- 空间索引加速鼠标检测
- 视野裁剪减少不必要的更新
- 消息处理限流防止卡顿

### 4. **扩展性**
- 新增功能只需添加新的子系统或处理器
- 支持插件和MOD (通过 trait 注册)

### 5. **并行化**
- Rust 的 Send/Sync 天然支持多线程
- 渲染准备和物理更新可以并行

---

## 🚀 实施计划

### 阶段 1: 搭建基础架构 (3天)
1. ✅ 创建 `systems/` 目录和基础 trait
2. ✅ 实现 `InputSystem` (输入系统)
3. ✅ 实现 `ObjectManager` (对象管理)

### 阶段 2: 迁移现有代码 (5天)
1. 从 `game_scene.rs` 提取输入处理代码到 `InputSystem`
2. 从 `game_scene.rs` 提取对象管理代码到 `ObjectManager`
3. 从 `game_scene.rs` 提取渲染代码到 `RenderingPipeline`

### 阶段 3: 实现 UI 系统 (7天)
1. 实现 `UIManager` 和 `Dialog` trait
2. 迁移 `InventoryDialog`、`CharacterDialog` 等核心对话框
3. 实现对话框栈和输入事件传递

### 阶段 4: 实现网络和特效 (5天)
1. 实现 `NetworkHandler` 和 `PacketHandler` trait
2. 实现 `EffectSystem` 和 `SoundManager`
3. 实现 `DataManager` 数据缓存

### 阶段 5: 测试和优化 (5天)
1. 单元测试每个子系统
2. 性能分析和优化 (profiling)
3. 修复 BUG 和内存泄漏

**总计**: ~25 天完成重构

---

## 📝 注意事项

1. **渐进式重构**: 不要一次性重写所有代码,保持可运行状态
2. **保留旧代码**: 在 `game_scene_old.rs` 保留旧代码作为参考
3. **单元测试**: 每个子系统完成后立即编写测试
4. **文档先行**: 先设计接口和文档,再实现代码
5. **性能监控**: 使用 `tracing` 和 `flamegraph` 监控性能

---

## 🔗 参考资料

- [Rust ECS 架构](https://github.com/amethyst/specs)
- [游戏引擎架构](https://www.gameenginebook.com/)
- [Data-Oriented Design](https://www.dataorienteddesign.com/)
- [ggez 最佳实践](https://github.com/ggez/ggez/wiki/FAQ)
