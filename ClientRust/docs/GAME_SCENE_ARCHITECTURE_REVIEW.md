# 游戏场景架构审查报告

> 审查日期: 2025年10月25日  
> 审查目标: 确保 GameScene 符合 ECS 架构思想

---

## 📋 审查结果摘要

### ✅ 优势（符合ECS思想）

1. **核心数据存储在 World**
   - 所有游戏数据（玩家、怪物、地图、UI等）都存储在 `hecs::World`
   - GameScene 只持有 Entity ID 引用，不直接存储数据
   
2. **系统职责清晰**
   - 15个独立系统，每个系统负责单一职责
   - 系统通过 World 查询组件，符合 ECS 查询模式

3. **渲染层次分离**
   - RenderSystem: 游戏世界层（世界坐标）
   - UISystem: UI层（设计坐标 1024×768）
   - 两层独立，互不干扰

### ⚠️ 问题（违反ECS思想）

1. **GameScene 承担太多职责**
   - 1207行代码，包含大量UI事件处理逻辑
   - 直接处理键盘/鼠标输入，应由专门的 InputSystem 处理
   - 包含许多辅助方法（17个 `get_*_dialog_mut()` 等）

2. **事件处理耦合严重**
   - `on_key_down()` 方法 200+ 行，包含所有快捷键逻辑
   - `on_mouse_down()` 方法 150+ 行，直接操作对话框
   - 应该用事件系统解耦

3. **UI 管理混乱**
   - 同时持有 `UISystem`、`DialogManager` 和多个 dialog entity
   - 对话框状态同步需要手动调用 3 次（manager、entity、dialog）
   - 应该统一由 UISystem 管理

4. **坐标转换重复**
   - `window_to_ui_coords()` 在 GameScene 中实现
   - 每次鼠标事件都要转换，应该在事件阶段统一转换

---

## 🎯 架构问题详细分析

### 问题 1: GameScene 过于庞大

**当前结构**:
```rust
pub struct GameScene {
    // 🎯 Entity 引用 (符合ECS) ✅
    camera_entity: Entity,
    time_entity: Entity,
    config_entity: Entity,
    visible_area_entity: Entity,
    
    // 🎯 UI Entity 引用 (符合ECS) ✅
    main_dialog_entity: Entity,
    inventory_dialog_entity: Entity,
    character_dialog_entity: Entity,
    skillbar_entities: [Entity; 2],
    chat_dialog_entity: Entity,
    magic_learning_dialog_entity: Entity,
    quest_dialog_entity: Entity,
    trade_dialog_entity: Entity,
    
    // ⚠️ 系统实例 (应该是静态方法或独立系统) 
    network_system: NetworkSystem,
    ui_system: UISystem,
    dialog_manager: DialogManager,
    
    // 配置
    ui_font_name: String,
}

impl Scene for GameScene {
    // ❌ 1207 行代码，包含大量业务逻辑
    fn update(...) { /* 130+ 行 */ }
    fn draw(...) { /* 100+ 行 */ }
    fn on_key_down(...) { /* 200+ 行 */ }
    fn on_mouse_down(...) { /* 150+ 行 */ }
    fn on_mouse_up(...) { /* 60+ 行 */ }
    fn on_mouse_move(...) { /* 40+ 行 */ }
    // ...
}
```

**问题**:
- GameScene 变成了"上帝类"，违反单一职责原则
- 所有输入事件都在这里处理，应该分发给专门的系统
- 包含太多辅助方法（17个 getter），说明耦合太紧

---

### 问题 2: 输入处理耦合

**当前 `on_key_down()` 方法 (200+ 行)**:
```rust
fn on_key_down(...) {
    match keycode {
        KeyCode::Space => { /* 拾取物品 */ }
        KeyCode::Escape => { /* 返回菜单 */ }
        KeyCode::KeyK => { /* 打开技能学习 */ }
        KeyCode::KeyQ => { /* 打开任务 */ }
        KeyCode::KeyT => { /* 打开交易 */ }
        KeyCode::F1..F8 => { /* 施放技能 */ }
        KeyCode::Digit1..8 => { /* 使用物品 */ }
        KeyCode::KeyZ => { /* 整理背包 */ }
        KeyCode::KeyN => { /* 与NPC对话 */ }
        KeyCode::Tab => { /* 切换目标 */ }
        KeyCode::KeyB => { /* 调试：边框 */ }
        KeyCode::KeyG => { /* 调试：网格 */ }
        KeyCode::KeyO => { /* 调试：障碍物 */ }
        KeyCode::KeyP => { /* 调试：路径 */ }
        KeyCode::KeyI => { /* 切换背包 */ }
        KeyCode::KeyC => { /* 切换角色 */ }
        KeyCode::KeyS => { /* 切换技能 */ }
        _ => {}
    }
}
```

**问题**:
- 所有快捷键逻辑硬编码在 GameScene
- 不同系统的操作混在一起（UI、战斗、物品、调试）
- 无法动态配置快捷键
- 违反开放封闭原则

**ECS 正确做法**:
```rust
// 应该有一个 InputSystem 负责分发输入事件
pub struct InputSystem;

impl InputSystem {
    pub fn process_keyboard(world: &mut World, keycode: KeyCode) {
        // 根据当前游戏状态分发事件
        if let Some(ui_focus) = Self::get_ui_focus(world) {
            // 有UI焦点，交给UI系统处理
            UISystem::handle_keyboard(world, keycode);
        } else {
            // 游戏世界焦点，交给游戏系统处理
            match keycode {
                KeyCode::Space => ItemSystem::pickup(world),
                KeyCode::F1..F8 => MagicCastSystem::cast(world, slot),
                KeyCode::Tab => TargetSystem::cycle(world),
                // ...
            }
        }
    }
}
```

---

### 问题 3: UI 状态同步混乱

**当前对话框切换代码**:
```rust
// ❌ 需要同步 3 个地方的状态！
MainDialogButton::Inventory => {
    // 1️⃣ 更新 DialogManager
    self.dialog_manager.toggle(DialogType::Inventory);
    
    // 2️⃣ 更新对话框组件的 is_open 字段
    if let Some(mut inv_dialog) = self.get_inventory_dialog_mut(world) {
        let is_visible = self.dialog_manager.is_visible(DialogType::Inventory);
        inv_dialog.is_open = is_visible;
        
        // 3️⃣ 更新内部对话框的可见性
        inv_dialog.dialog.set_visible(is_visible);
    }
}
```

**问题**:
- 一个操作需要同步 3 个地方
- 容易出现状态不一致的 bug
- 违反单一数据源原则

**ECS 正确做法**:
```rust
// 方案A: DialogManager 本身就是一个 ECS 组件
#[derive(Component)]
pub struct DialogState {
    pub dialog_type: DialogType,
    pub is_open: bool,
    pub z_order: i32,
}

// UISystem 统一管理所有对话框状态
impl UISystem {
    pub fn toggle_dialog(world: &mut World, dialog_type: DialogType) {
        // 直接查询和修改组件，自动保持同步
        for (_, state) in world.query_mut::<&mut DialogState>() {
            if state.dialog_type == dialog_type {
                state.is_open = !state.is_open;
            }
        }
    }
}
```

---

### 问题 4: 鼠标事件处理复杂

**当前 `on_mouse_down()` 方法 (150+ 行)**:
```rust
fn on_mouse_down(...) {
    // 1. 转换坐标
    let (ui_x, ui_y) = self.window_to_ui_coords(ctx, x, y);
    
    // 2. 检查角色对话框
    if let Some(mut char_dialog) = self.get_character_dialog_mut(world) {
        if let Some(action) = char_dialog.dialog.on_mouse_down(ui_x, ui_y) {
            // 处理点击...
            return Ok(());
        }
    }
    
    // 3. 检查背包对话框
    if let Some(mut inv_dialog) = self.get_inventory_dialog_mut(world) {
        if let Some(action) = inv_dialog.dialog.on_mouse_down(ui_x, ui_y) {
            // 处理点击...
            return Ok(());
        }
    }
    
    // 4. 检查主对话框
    if button == MouseButton::Left {
        let clicked_button = {
            if let Some(mut main_dialog) = self.get_main_dialog_mut(world) {
                main_dialog.dialog.on_mouse_down(ui_x, ui_y)
            } else {
                None
            }
        };
        // 处理点击...
    }
    
    // 5. 更新鼠标输入状态
    if let Some((_, mouse_input)) = world.query_mut::<&mut MouseInput>().into_iter().next() {
        // ...
    }
}
```

**问题**:
- UI 点击检测顺序硬编码（应该按 z-order）
- 每个对话框单独检测（O(n) 复杂度）
- 坐标转换重复计算
- 应该有统一的事件分发机制

---

## 🔧 重构建议

### 建议 1: 创建 InputSystem

**目标**: 将所有输入处理从 GameScene 移到专门的系统

```rust
// src/ecs/systems/input_system.rs

use hecs::World;
use ggez::input::keyboard::KeyCode;
use ggez::winit::event::MouseButton;

/// 输入系统 - 负责处理所有键盘、鼠标输入
pub struct InputSystem;

impl InputSystem {
    /// 处理键盘输入
    pub fn process_keyboard(world: &mut World, keycode: KeyCode, network_tx: &mpsc::UnboundedSender<NetworkCommand>) {
        // 检查是否有UI焦点
        if Self::has_ui_focus(world) {
            Self::handle_ui_keyboard(world, keycode);
        } else {
            Self::handle_game_keyboard(world, keycode, network_tx);
        }
    }
    
    /// 处理 UI 快捷键
    fn handle_ui_keyboard(world: &mut World, keycode: KeyCode) {
        use KeyCode::*;
        match keycode {
            KeyI => UISystem::toggle_dialog(world, DialogType::Inventory),
            KeyC => UISystem::toggle_dialog(world, DialogType::Character),
            KeyS => UISystem::toggle_dialog(world, DialogType::Skills),
            KeyQ => UISystem::toggle_dialog(world, DialogType::Quest),
            KeyK => UISystem::toggle_dialog(world, DialogType::MagicLearning),
            Escape => UISystem::close_top_dialog(world),
            _ => {}
        }
    }
    
    /// 处理游戏世界快捷键
    fn handle_game_keyboard(world: &mut World, keycode: KeyCode, network_tx: &mpsc::UnboundedSender<NetworkCommand>) {
        use KeyCode::*;
        match keycode {
            Space => ItemSystem::pickup_nearby(world, network_tx),
            Tab => TargetSystem::cycle_target(world),
            KeyN => NPCSystem::interact_nearest(world, network_tx),
            KeyZ => ItemSystem::organize_inventory(world),
            
            // 技能快捷键 F1-F8
            F1 => MagicCastSystem::cast_spell_slot(world, 0, network_tx),
            F2 => MagicCastSystem::cast_spell_slot(world, 1, network_tx),
            F3 => MagicCastSystem::cast_spell_slot(world, 2, network_tx),
            F4 => MagicCastSystem::cast_spell_slot(world, 3, network_tx),
            F5 => MagicCastSystem::cast_spell_slot(world, 4, network_tx),
            F6 => MagicCastSystem::cast_spell_slot(world, 5, network_tx),
            F7 => MagicCastSystem::cast_spell_slot(world, 6, network_tx),
            F8 => MagicCastSystem::cast_spell_slot(world, 7, network_tx),
            
            // 物品快捷键 1-8
            Digit1 => ItemSystem::use_item(world, 0, network_tx),
            Digit2 => ItemSystem::use_item(world, 1, network_tx),
            Digit3 => ItemSystem::use_item(world, 2, network_tx),
            Digit4 => ItemSystem::use_item(world, 3, network_tx),
            Digit5 => ItemSystem::use_item(world, 4, network_tx),
            Digit6 => ItemSystem::use_item(world, 5, network_tx),
            Digit7 => ItemSystem::use_item(world, 6, network_tx),
            Digit8 => ItemSystem::use_item(world, 7, network_tx),
            
            _ => {}
        }
    }
    
    /// 处理鼠标点击
    pub fn process_mouse_click(
        world: &mut World, 
        button: MouseButton, 
        ui_x: f32, 
        ui_y: f32,
        window_x: f32,
        window_y: f32,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>
    ) {
        // 先检查 UI 层
        if UISystem::handle_click(world, button, ui_x, ui_y) {
            return; // UI 消费了事件
        }
        
        // 再处理游戏世界点击
        Self::handle_world_click(world, button, window_x, window_y, network_tx);
    }
    
    /// 处理游戏世界点击
    fn handle_world_click(
        world: &mut World, 
        button: MouseButton, 
        x: f32, 
        y: f32,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>
    ) {
        match button {
            MouseButton::Left => {
                // 左键单击：移动
                PlayerSystem::move_to_position(world, x, y, network_tx);
            }
            MouseButton::Right => {
                // 右键：攻击/拾取/交互
                Self::handle_right_click(world, x, y, network_tx);
            }
            _ => {}
        }
    }
    
    /// 检查是否有 UI 焦点
    fn has_ui_focus(world: &World) -> bool {
        // 检查是否有对话框打开
        for (_, state) in world.query::<&DialogState>().iter() {
            if state.is_open && state.blocks_input {
                return true;
            }
        }
        false
    }
}
```

**GameScene 简化后**:
```rust
impl Scene for GameScene {
    fn on_key_down(&mut self, _ctx: &mut Context, world: &mut World, input: KeyInput, network_tx: &mpsc::UnboundedSender<NetworkCommand>) -> GameResult<Option<SceneType>> {
        if let KeyEvent { physical_key: PhysicalKey::Code(keycode), .. } = input.event {
            // ✅ 只需一行！所有逻辑在 InputSystem
            InputSystem::process_keyboard(world, keycode, network_tx);
        }
        Ok(None)
    }
    
    fn on_mouse_down(&mut self, ctx: &mut Context, world: &mut World, button: MouseButton, x: f32, y: f32, network_tx: &mpsc::UnboundedSender<NetworkCommand>) -> GameResult {
        let (ui_x, ui_y) = self.window_to_ui_coords(ctx, x, y);
        
        // ✅ 只需一行！所有逻辑在 InputSystem
        InputSystem::process_mouse_click(world, button, ui_x, ui_y, x, y, network_tx);
        
        Ok(())
    }
}
```

---

### 建议 2: 统一 UI 管理

**目标**: 所有 UI 状态由 UISystem 统一管理，移除 DialogManager

```rust
// src/ecs/systems/ui_system.rs

/// UI 组件 - 对话框状态
#[derive(Component)]
pub struct DialogState {
    pub dialog_type: DialogType,
    pub is_open: bool,
    pub z_order: i32,
    pub blocks_input: bool,  // 是否阻止游戏世界输入
}

pub struct UISystem {
    // 不再持有任何状态，纯粹的系统
}

impl UISystem {
    /// 切换对话框
    pub fn toggle_dialog(world: &mut World, dialog_type: DialogType) {
        for (_, state) in world.query_mut::<&mut DialogState>() {
            if state.dialog_type == dialog_type {
                state.is_open = !state.is_open;
                
                // 打开时自动提升到最上层
                if state.is_open {
                    Self::bring_to_front(world, dialog_type);
                }
                break;
            }
        }
    }
    
    /// 关闭所有对话框
    pub fn close_all_dialogs(world: &mut World) {
        for (_, state) in world.query_mut::<&mut DialogState>() {
            state.is_open = false;
        }
    }
    
    /// 关闭最上层对话框
    pub fn close_top_dialog(world: &mut World) {
        let mut top_dialog: Option<(Entity, i32)> = None;
        
        for (entity, state) in world.query::<&DialogState>().iter() {
            if state.is_open {
                if let Some((_, z)) = top_dialog {
                    if state.z_order > z {
                        top_dialog = Some((entity, state.z_order));
                    }
                } else {
                    top_dialog = Some((entity, state.z_order));
                }
            }
        }
        
        if let Some((entity, _)) = top_dialog {
            if let Ok(mut state) = world.get::<&mut DialogState>(entity) {
                state.is_open = false;
            }
        }
    }
    
    /// 将对话框提升到最前
    fn bring_to_front(world: &mut World, dialog_type: DialogType) {
        let max_z = world.query::<&DialogState>()
            .iter()
            .map(|(_, s)| s.z_order)
            .max()
            .unwrap_or(0);
        
        for (_, state) in world.query_mut::<&mut DialogState>() {
            if state.dialog_type == dialog_type {
                state.z_order = max_z + 1;
                break;
            }
        }
    }
    
    /// 处理 UI 点击（按 z-order 顺序）
    pub fn handle_click(world: &mut World, button: MouseButton, ui_x: f32, ui_y: f32) -> bool {
        // 收集所有打开的对话框，按 z-order 排序
        let mut dialogs: Vec<(Entity, i32, DialogType)> = world.query::<&DialogState>()
            .iter()
            .filter(|(_, state)| state.is_open)
            .map(|(entity, state)| (entity, state.z_order, state.dialog_type))
            .collect();
        
        // 从上到下检测点击
        dialogs.sort_by_key(|(_, z, _)| -z);
        
        for (entity, _, dialog_type) in dialogs {
            // 根据对话框类型分发点击
            let consumed = match dialog_type {
                DialogType::Inventory => Self::handle_inventory_click(world, entity, button, ui_x, ui_y),
                DialogType::Character => Self::handle_character_click(world, entity, button, ui_x, ui_y),
                DialogType::Main => Self::handle_main_click(world, entity, button, ui_x, ui_y),
                // ...
                _ => false,
            };
            
            if consumed {
                return true;  // 事件被消费
            }
        }
        
        false  // 事件未被 UI 消费，传递到游戏世界
    }
    
    /// 绘制所有 UI（按 z-order 顺序）
    pub fn draw(ctx: &mut Context, canvas: &mut Canvas, world: &World, _current_time: u64) -> GameResult {
        // 收集所有打开的对话框，按 z-order 排序
        let mut dialogs: Vec<(Entity, i32, DialogType)> = world.query::<&DialogState>()
            .iter()
            .filter(|(_, state)| state.is_open)
            .map(|(entity, state)| (entity, state.z_order, state.dialog_type))
            .collect();
        
        // 从下到上绘制
        dialogs.sort_by_key(|(_, z, _)| *z);
        
        for (entity, _, dialog_type) in dialogs {
            // 根据对话框类型分发绘制
            match dialog_type {
                DialogType::Main => Self::draw_main_dialog(ctx, canvas, world, entity)?,
                DialogType::Inventory => Self::draw_inventory_dialog(ctx, canvas, world, entity)?,
                DialogType::Character => Self::draw_character_dialog(ctx, canvas, world, entity)?,
                // ...
                _ => {}
            }
        }
        
        Ok(())
    }
}
```

**GameScene 简化后**:
```rust
pub struct GameScene {
    // ❌ 移除这些字段
    // dialog_manager: DialogManager,
    // main_dialog_entity: Entity,
    // inventory_dialog_entity: Entity,
    // character_dialog_entity: Entity,
    // ...
    
    // ✅ 只保留必要的
    camera_entity: Entity,
    time_entity: Entity,
    config_entity: Entity,
    
    // UI 系统不需要持有，可以是静态方法
    // ui_system: UISystem,
}

impl GameScene {
    pub fn new(ctx: &mut Context, world: &mut World) -> GameResult<Self> {
        // 创建对话框实体时同时添加 DialogState 组件
        let _main_dialog = world.spawn((
            MainDialogComp::new(DESIGN_WIDTH, DESIGN_HEIGHT),
            DialogState {
                dialog_type: DialogType::Main,
                is_open: true,
                z_order: 0,
                blocks_input: false,
            },
        ));
        
        let _inventory_dialog = world.spawn((
            InventoryDialogComp::new(),
            DialogState {
                dialog_type: DialogType::Inventory,
                is_open: false,
                z_order: 1,
                blocks_input: true,
            },
        ));
        
        // ... 其他对话框
        
        Ok(Self {
            camera_entity,
            time_entity,
            config_entity,
        })
    }
}

impl Scene for GameScene {
    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, world: &World) -> GameResult {
        // ... 绘制游戏世界 ...
        
        // ✅ UI 绘制只需一行！
        UISystem::draw(ctx, canvas, world, 0)?;
        
        Ok(())
    }
}
```

---

### 建议 3: 创建事件系统

**目标**: 用事件总线解耦系统间通信

```rust
// src/ecs/systems/event_system.rs

use hecs::{World, Entity};
use std::collections::VecDeque;

/// 游戏事件
#[derive(Debug, Clone)]
pub enum GameEvent {
    // UI 事件
    DialogOpened(DialogType),
    DialogClosed(DialogType),
    ButtonClicked(ButtonId),
    
    // 玩家事件
    PlayerMoved(f32, f32),
    PlayerDied,
    LevelUp(u32),
    
    // 战斗事件
    AttackHit { attacker: Entity, target: Entity, damage: u32 },
    SpellCast { caster: Entity, spell: SpellType },
    
    // 物品事件
    ItemPickedUp(u32),  // item_id
    ItemUsed(u32),
    ItemDropped(u32),
    
    // 任务事件
    QuestAccepted(u32),  // quest_id
    QuestCompleted(u32),
    ObjectiveProgress { quest_id: u32, objective_index: usize, progress: u32 },
}

/// 事件系统 - 发布/订阅模式
#[derive(Component)]
pub struct EventQueue {
    events: VecDeque<GameEvent>,
}

impl EventQueue {
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
        }
    }
    
    /// 发布事件
    pub fn publish(world: &mut World, event: GameEvent) {
        if let Some((_, queue)) = world.query_mut::<&mut EventQueue>().into_iter().next() {
            queue.events.push_back(event);
        }
    }
    
    /// 处理所有待处理事件
    pub fn process_events(world: &mut World) {
        // 取出所有事件
        let events: Vec<GameEvent> = {
            if let Some((_, queue)) = world.query_mut::<&mut EventQueue>().into_iter().next() {
                queue.events.drain(..).collect()
            } else {
                Vec::new()
            }
        };
        
        // 分发事件到各个系统
        for event in events {
            Self::dispatch_event(world, event);
        }
    }
    
    /// 分发事件到对应系统
    fn dispatch_event(world: &mut World, event: GameEvent) {
        match event {
            GameEvent::AttackHit { attacker, target, damage } => {
                CombatSystem::apply_damage(world, attacker, target, damage);
            }
            GameEvent::ItemPickedUp(item_id) => {
                ItemSystem::add_to_inventory(world, item_id);
            }
            GameEvent::QuestAccepted(quest_id) => {
                QuestSystem::start_quest(world, quest_id);
            }
            GameEvent::ObjectiveProgress { quest_id, objective_index, progress } => {
                QuestSystem::update_progress(world, quest_id, objective_index, progress);
            }
            GameEvent::SpellCast { caster, spell } => {
                MagicCastSystem::execute_spell(world, caster, spell);
            }
            _ => {}
        }
    }
}
```

**使用示例**:
```rust
// 发布事件（任何系统都可以发布）
EventQueue::publish(world, GameEvent::ItemPickedUp(123));

// 在主循环中处理事件
fn update(&mut self, world: &mut World) {
    // 1. 更新所有系统
    PlayerSystem::update(world);
    MonsterSystem::update(world, delta);
    
    // 2. 处理所有事件
    EventQueue::process_events(world);
    
    // 3. 网络同步
    NetworkSystem::sync(world);
}
```

---

### 建议 4: 简化坐标转换

**目标**: 在事件进入系统前统一转换坐标

```rust
// src/ecs/systems/coordinate_system.rs

/// 坐标系统 - 负责各种坐标转换
pub struct CoordinateSystem;

impl CoordinateSystem {
    /// 窗口坐标 -> UI 设计坐标 (1024×768)
    pub fn window_to_ui(ctx: &Context, window_x: f32, window_y: f32) -> (f32, f32) {
        let (window_width, window_height) = ctx.gfx.drawable_size();
        
        const DESIGN_WIDTH: f32 = 1024.0;
        const DESIGN_HEIGHT: f32 = 768.0;
        const ASPECT_RATIO: f32 = 4.0 / 3.0;
        
        let current_ratio = window_width / window_height;
        
        let (viewport_width, viewport_height) = if current_ratio > ASPECT_RATIO {
            (window_height * ASPECT_RATIO, window_height)
        } else {
            (window_width, window_width / ASPECT_RATIO)
        };
        
        let offset_x = (window_width - viewport_width) / 2.0;
        let offset_y = (window_height - viewport_height) / 2.0;
        
        let viewport_x = window_x - offset_x;
        let viewport_y = window_y - offset_y;
        
        let design_x = (viewport_x / viewport_width) * DESIGN_WIDTH;
        let design_y = (viewport_y / viewport_height) * DESIGN_HEIGHT;
        
        (design_x, design_y)
    }
    
    /// 窗口坐标 -> 世界坐标
    pub fn window_to_world(world: &World, camera_entity: Entity, window_x: f32, window_y: f32) -> (f32, f32) {
        let pos = world.get::<&Position>(camera_entity).unwrap();
        let camera = world.get::<&Camera>(camera_entity).unwrap();
        
        CameraSystem::screen_to_world(pos, camera, window_x, window_y)
    }
    
    /// 世界坐标 -> 窗口坐标
    pub fn world_to_window(world: &World, camera_entity: Entity, world_x: f32, world_y: f32) -> (f32, f32) {
        let pos = world.get::<&Position>(camera_entity).unwrap();
        let camera = world.get::<&Camera>(camera_entity).unwrap();
        
        CameraSystem::world_to_screen(pos, camera, world_x, world_y)
    }
}
```

**GameScene 简化后**:
```rust
impl Scene for GameScene {
    fn on_mouse_down(&mut self, ctx: &mut Context, world: &mut World, button: MouseButton, x: f32, y: f32, network_tx: &mpsc::UnboundedSender<NetworkCommand>) -> GameResult {
        // ✅ 统一转换坐标
        let (ui_x, ui_y) = CoordinateSystem::window_to_ui(ctx, x, y);
        let (world_x, world_y) = CoordinateSystem::window_to_world(world, self.camera_entity, x, y);
        
        // ✅ 分发给 InputSystem
        InputSystem::process_mouse_click(world, button, ui_x, ui_y, world_x, world_y, network_tx);
        
        Ok(())
    }
}
```

---

## 📊 重构前后对比

### 重构前（当前状态）

```
GameScene (1207 行)
├── update() - 130 行
├── draw() - 100 行
├── on_key_down() - 200 行 ❌
├── on_mouse_down() - 150 行 ❌
├── on_mouse_up() - 60 行 ❌
├── on_mouse_move() - 40 行 ❌
├── 17 个 get_*_dialog_mut() 辅助方法 ❌
└── window_to_ui_coords() ❌

问题:
- GameScene 承担太多职责
- 输入处理耦合严重
- UI 状态同步混乱（3个地方）
- 坐标转换重复计算
```

### 重构后（建议）

```
GameScene (约 200-300 行)
├── update() - 50 行
├── draw() - 50 行
├── on_key_down() - 10 行 ✅
├── on_mouse_down() - 15 行 ✅
├── on_mouse_up() - 10 行 ✅
├── on_mouse_move() - 10 行 ✅
└── (无辅助方法) ✅

新增系统:
├── InputSystem - 负责所有输入处理
├── CoordinateSystem - 负责坐标转换
├── EventSystem - 负责事件分发
└── UISystem (重构) - 统一管理所有 UI

优势:
✅ 单一职责原则
✅ 开放封闭原则
✅ 系统间解耦
✅ 代码易于测试和维护
```

---

## 🎯 分阶段重构计划

### 第一阶段: 创建 InputSystem（优先级：高）

**工作量**: 1-2天

**步骤**:
1. 创建 `src/ecs/systems/input_system.rs`
2. 将所有键盘处理逻辑移到 `InputSystem::process_keyboard()`
3. 将所有鼠标处理逻辑移到 `InputSystem::process_mouse_*()`
4. 简化 GameScene 的事件处理方法（每个只保留 1-2 行）

**预期效果**:
- GameScene 代码减少 400+ 行
- 输入处理逻辑集中管理
- 可以动态配置快捷键

---

### 第二阶段: 重构 UISystem（优先级：高）

**工作量**: 2-3天

**步骤**:
1. 移除 DialogManager，改用 DialogState 组件
2. 将所有对话框状态管理移到 UISystem
3. 实现 z-order 排序和点击检测
4. 简化 GameScene，移除所有 dialog entity 引用

**预期效果**:
- UI 状态同步简化（1个地方）
- 对话框层级管理自动化
- GameScene 代码减少 200+ 行

---

### 第三阶段: 创建 EventSystem（优先级：中）

**工作量**: 1-2天

**步骤**:
1. 创建 `src/ecs/systems/event_system.rs`
2. 定义 GameEvent 枚举
3. 实现事件队列和分发机制
4. 将系统间通信改为事件驱动

**预期效果**:
- 系统间解耦
- 支持异步事件处理
- 易于添加新功能

---

### 第四阶段: 创建 CoordinateSystem（优先级：低）

**工作量**: 半天

**步骤**:
1. 创建 `src/ecs/systems/coordinate_system.rs`
2. 将坐标转换方法移到这里
3. 统一所有坐标转换逻辑

**预期效果**:
- 坐标转换统一管理
- 减少重复计算
- 代码更清晰

---

## 📝 总结

### 当前架构问题

| 问题 | 严重程度 | 影响 |
|------|---------|------|
| GameScene 过于庞大 | 🔴 高 | 难以维护，违反单一职责 |
| 输入处理耦合 | 🔴 高 | 无法扩展，硬编码严重 |
| UI 状态同步混乱 | 🟡 中 | 容易出 bug，需要手动同步 3 处 |
| 坐标转换重复 | 🟢 低 | 性能影响小，但代码冗余 |

### 重构优先级

1. **第一阶段: InputSystem** 🔴 - 立即执行
   - 影响最大，收益最高
   - 简化 GameScene 400+ 行代码
   
2. **第二阶段: UISystem 重构** 🔴 - 近期执行
   - 解决 UI 状态同步问题
   - 简化 GameScene 200+ 行代码
   
3. **第三阶段: EventSystem** 🟡 - 中期执行
   - 系统间解耦
   - 为后续功能打基础
   
4. **第四阶段: CoordinateSystem** 🟢 - 可选
   - 代码整理，影响较小

### 架构目标

✅ **符合 ECS 思想**:
- 所有数据在 World 中（Component）
- 所有逻辑在 System 中
- Scene 只负责系统调度

✅ **单一职责原则**:
- 每个 System 只负责一类功能
- Scene 不包含业务逻辑

✅ **开放封闭原则**:
- 易于添加新功能（新 System）
- 不需要修改现有代码

✅ **解耦**:
- 系统间通过事件通信
- 不直接相互调用

---

**审查结论**: 当前架构虽然使用了 ECS，但 **GameScene 承担了太多职责**，违反了 ECS 的核心思想。建议按照上述 4 个阶段进行重构，最终达到 GameScene 只负责系统调度的目标。

**文档更新**: 2025年10月25日
