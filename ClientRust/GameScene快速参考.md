# GameScene 快速参考指南

## 🎯 核心数据结构

### GameSceneState (资源)
```rust
pub struct GameSceneState {
    pub player_entity: Option<Entity>,      // 玩家实体引用
    pub map_name: String,                  // 当前地图名称
    pub level: u32,                        // 玩家等级
    pub experience: u32,                   // 经验值
    pub current_hp: f32,                   // 当前 HP
    pub max_hp: f32,                       // 最大 HP
    pub current_mana: f32,                 // 当前 Mana
    pub max_mana: f32,                     // 最大 Mana
    pub show_chat: bool,                   // 是否显示聊天
    pub show_inventory: bool,              // 是否显示背包
    pub show_skills: bool,                 // 是否显示技能
    pub show_character: bool,              // 是否显示角色面板
    pub show_party: bool,                  // 是否显示队伍
    pub game_time: f32,                    // 游戏时间累计
    pub is_paused: bool,                   // 是否暂停
}
```

### Player (组件)
```rust
pub struct Player {
    pub character_id: u32,    // 角色 ID
    pub name: String,         // 角色名
    pub class: String,        // 职业
    pub gender: u8,           // 性别 (0=男, 1=女)
    pub level: u32,           // 等级
}
```

### PlayerMovement (组件)
```rust
pub struct PlayerMovement {
    pub speed: f32,           // 移动速度
    pub direction: Vec2,      // 移动方向
    pub is_moving: bool,      // 是否在移动
}
```

---

## 📡 消息类型

所有消息都实现 `Message + Clone + Default`

```rust
// 移动消息
pub struct PlayerMoveMessage { pub direction: Vec2 }
pub struct PlayerStopMessage;

// 聊天消息
pub struct OpenChatMessage;
pub struct CloseChatMessage;
pub struct SendChatMessage { pub text: String }

// 背包消息
pub struct OpenInventoryMessage;
pub struct CloseInventoryMessage;

// 技能消息
pub struct OpenSkillsMessage;
pub struct CloseSkillsMessage;

// 角色/队伍消息
pub struct OpenCharacterMessage;
pub struct CloseCharacterMessage;

// 游戏消息
pub struct PauseGameMessage { pub pause: bool }
pub struct ExitGameMessage;
pub struct InteractWithNpcMessage { pub npc_id: u32 }
pub struct UseSkillMessage { pub skill_id: u32 }
```

---

## 🎮 系统调用流程

### 启动流程
```
main_bevy.rs
  ↓
GameState::Game 状态激活
  ↓
OnEnter(GameState::Game)
  ├─ setup_game_scene() → 创建 HUD UI
  ├─ spawn_test_player() → 生成测试玩家
  └─ setup_map_system() → 加载地图
  ↓
Update 系统每帧执行
  ├─ update_game_time() → 更新时间
  ├─ handle_player_input() → 读取键盘
  ├─ handle_player_movement() → 计算方向
  ├─ update_player_position() → 更新位置
  ├─ update_hud_display() → 刷新 UI 显示
  ├─ handle_quickslot_hover() → UI 交互反馈
  └─ message_handle_*() → 处理消息
  ↓
OnExit(GameState::Game)
  └─ cleanup_game_scene() → 清理 UI 和资源
```

---

## 🖱️ 输入处理

| 按键 | 功能 | 处理器 |
|------|------|--------|
| W/A/S/D | 移动 | handle_player_input() |
| Enter | 打开聊天 | handle_player_input() |
| 1-0 | 快捷技能 | handle_player_input() |
| Esc | 暂停/恢复 | handle_player_input() |

---

## 🎨 UI 组件层级

```
GameSceneRoot (root)
├── HudRoot (container)
│   ├── PlayerInfoHud (text display)
│   │   └── [level, exp, hp/mp text]
│   ├── SkillBar (button grid)
│   │   └── [QuickSlotButton x 12]
│   ├── MiniMap (image display)
│   └── ChatPanel (flex column)
│       ├── ChatMessageList (scroll)
│       │   └── [ChatMessage x N]
│       └── ChatInput (input field)
```

---

## 📊 性能指标

| 指标 | 值 | 说明 |
|------|-----|------|
| 文件大小 | 842 行 | components.rs (285) + mod.rs (557) |
| 系统数量 | 15 个 | 2 生命周期 + 4 游戏循环 + 2 UI + 7 消息处理 |
| 消息类型 | 13 个 | 全部注册到应用 |
| UI 组件 | 11 个 | UI 标记组件 |
| 编译时间 | 0.49s | cargo check |
| 编译错误 | 0 | ✅ 完全通过 |

---

## 🔧 常用修改点

### 添加新消息类型
1. 在 `components.rs` 中定义消息结构体
   ```rust
   #[derive(Message, Clone, Default)]
   pub struct MyMessage { pub field: Type }
   ```

2. 在 `main_bevy.rs` 中注册
   ```rust
   app.register_message::<MyMessage>();
   ```

3. 在 `mod.rs` 中添加处理器
   ```rust
   pub fn message_handle_my_message(mut reader: EventReader<MyMessage>) {
       for event in reader.read() { /* ... */ }
   }
   ```

4. 在 `main_bevy.rs` 的 Update 系统中添加
   ```rust
   app.add_systems(Update, message_handle_my_message.run_if(in_state(GameState::Game)));
   ```

### 修改 UI 布局
编辑 `setup_game_scene()` 函数中的 UI 节点配置：
- 位置: `position_type`, `top/bottom/left/right` (绝对定位)
- 大小: `width`, `height` (使用 `Val::Px()` 或 `Val::Percent()`)
- 布局: `flex_direction`, `column_gap`, `row_gap`
- 样式: `BackgroundColor()`, `BorderColor::all()`

### 添加新系统
1. 在 `mod.rs` 中实现系统函数
   ```rust
   pub fn my_system(query: Query<&MyComponent>) { /* ... */ }
   ```

2. 在 `main_bevy.rs` 中注册
   ```rust
   app.add_systems(Update, my_system.run_if(in_state(GameState::Game)));
   ```

---

## 🐛 调试提示

### 查看 HUD 是否显示
```rust
// 在 update_hud_display() 中检查
info!("HUD 显示更新: 等级={}, HP={}/{}", 
    state.level, state.current_hp, state.max_hp);
```

### 检查输入是否被读取
```rust
// 在 handle_player_input() 中增加日志
if keyboard.just_pressed(KeyCode::KeyW) {
    warn!("W 键被按下!");
}
```

### 追踪消息传递
```rust
// 在消息处理器中增加日志
pub fn message_handle_player_move(mut reader: EventReader<PlayerMoveMessage>) {
    for event in reader.read() {
        debug!("收到 PlayerMoveMessage: {:?}", event);
    }
}
```

---

## 📚 相关文件位置

| 文件 | 用途 |
|------|------|
| src/bevy/scenes/game_scene/components.rs | 数据类型定义 |
| src/bevy/scenes/game_scene/mod.rs | 系统实现 |
| src/bevy/scenes/mod.rs | 模块导出 |
| src/bin/main_bevy.rs | 应用配置 |
| src/bevy/systems/test.rs | 测试生成 |

---

## ✨ 最佳实践

1. **消息处理**
   - 使用 `EventReader` 读取消息
   - 在消息处理器中只做单一职责的事
   - 使用 `ResMut<GameSceneState>` 共享状态

2. **UI 更新**
   - 在专门的 UI 系统中更新文本
   - 使用 `Query` 查询 UI 组件
   - 避免在其他系统中直接修改 UI

3. **系统组织**
   - 按功能分组系统 (游戏循环、UI、消息处理)
   - 使用 `run_if(in_state())` 条件执行
   - 按依赖顺序排列系统

4. **性能优化**
   - 只在需要时更新 UI 文本
   - 使用 `Changed<Component>` 检测变化
   - 批量处理消息事件

---

## 🎓 学习资源

- Bevy 官方文档: https://bevyengine.org/learn/
- ECS 系统设计: https://www.ecsdocs.io/
- 消息系统教程: Bevy 社区论坛

