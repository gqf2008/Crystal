# CustomContext API 文档

## 概述

`CustomContext` 是对 `ggez::Context` 的扩展包装，集成了 ECS World、网络上下文和输入事件缓冲。所有 `GameContext` 的便捷方法都已移植到 `CustomContext` 中，提供统一的游戏资源访问接口。

## 核心组件

### ggez 核心组件
- `fs: Filesystem` - 文件系统
- `gfx: GraphicsContext` - 图形上下文
- `keyboard: KeyboardContext` - 键盘输入
- `mouse: MouseContext` - 鼠标输入
- `gamepad: GamepadContext` - 手柄输入
- `time: TimeContext` - 时间管理
- `fields: ContextFields` - 上下文字段

### 游戏特定组件
- `world: World` - ECS 世界
- `settings: ClientSettings` - 客户端设置
- `net_events: CategorizedEvents` - 网络事件（按类型分类）
- `frame_input_events: Vec<InputEvent>` - 输入事件缓冲

## API 分类

### 1. ECS 查询

```rust
ctx.entity_count() -> usize              // 获取实体数量
ctx.entity_exists(entity) -> bool        // 检查实体是否存在
```

### 2. 时间相关

```rust
ctx.delta_time() -> f32                  // 获取帧间隔时间（秒）
ctx.time_since_start() -> f64            // 获取游戏运行总时间（秒）
ctx.fps() -> f32                         // 获取当前 FPS
```

### 3. 屏幕尺寸

```rust
ctx.screen_width() -> f32                // 获取屏幕宽度
ctx.screen_height() -> f32               // 获取屏幕高度
ctx.screen_size() -> (f32, f32)          // 获取屏幕尺寸（宽，高）
```

### 4. 鼠标操作

#### 基础鼠标状态
```rust
ctx.mouse_left_pressed() -> bool         // 左键是否按下
ctx.mouse_right_pressed() -> bool        // 右键是否按下
ctx.mouse_middle_pressed() -> bool       // 中键是否按下
ctx.mouse_button_pressed(button) -> Option<(MouseButton, f32, f32)>
```

#### 鼠标位置
```rust
ctx.mouse_position() -> (f32, f32)       // 获取鼠标位置
ctx.mouse_x() -> f32                     // 获取鼠标 X 坐标
ctx.mouse_y() -> f32                     // 获取鼠标 Y 坐标
ctx.mouse_in_bounds() -> bool            // 鼠标是否在屏幕内
```

#### 鼠标事件
```rust
ctx.mouse_motion()                       // 鼠标移动事件迭代器
ctx.mouse_wheel()                        // 鼠标滚轮事件迭代器
ctx.mouse_entered_or_leaved() -> Option<bool>
ctx.mouse_entered() -> Option<bool>
ctx.mouse_leaved() -> Option<bool>
```

#### 鼠标计算
```rust
ctx.mouse_distance_to_center() -> f32    // 鼠标到屏幕中心的距离
ctx.mouse_angle_from_center() -> f32     // 鼠标相对屏幕中心的角度（弧度）
ctx.mouse_in_rect(x, y, w, h) -> bool    // 鼠标是否在矩形内
ctx.mouse_in_circle(cx, cy, r) -> bool   // 鼠标是否在圆形内
ctx.mouse_distance_to(x, y) -> f32       // 鼠标到指定点的距离
```

### 5. 键盘操作

#### 基础键盘状态
```rust
ctx.key_pressed(key) -> bool             // 指定键是否按下
ctx.pressed_keys()                       // 获取当前按下的所有按键迭代器
ctx.text_input()                         // 文本输入事件迭代器
ctx.pressed_key_count() -> usize         // 按下的键数量
ctx.any_key_pressed() -> bool            // 是否有任何键按下
```

#### 修饰键
```rust
ctx.shift_pressed() -> bool              // Shift 键是否按下
ctx.ctrl_pressed() -> bool               // Ctrl 键是否按下
ctx.alt_pressed() -> bool                // Alt 键是否按下
```

#### 常用键
```rust
ctx.space_pressed() -> bool              // 空格键
ctx.enter_pressed() -> bool              // 回车键
ctx.escape_pressed() -> bool             // ESC 键
ctx.tab_pressed() -> bool                // Tab 键
ctx.backspace_pressed() -> bool          // 退格键
ctx.delete_pressed() -> bool             // 删除键
```

#### 方向键
```rust
ctx.arrow_up_pressed() -> bool           // 上箭头键
ctx.arrow_down_pressed() -> bool         // 下箭头键
ctx.arrow_left_pressed() -> bool         // 左箭头键
ctx.arrow_right_pressed() -> bool        // 右箭头键
ctx.arrow_direction() -> (i32, i32)      // 方向向量（-1, 0, 1）
```

#### WASD 键
```rust
ctx.w_pressed() -> bool                  // W 键
ctx.a_pressed() -> bool                  // A 键
ctx.s_pressed() -> bool                  // S 键
ctx.d_pressed() -> bool                  // D 键
ctx.wasd_direction() -> (i32, i32)       // WASD 方向向量
```

#### 数字键
```rust
ctx.digit_pressed(digit: u8) -> bool     // 主键盘数字键 0-9
ctx.numpad_digit_pressed(digit: u8) -> bool // 小键盘数字键 0-9
```

#### 功能键
```rust
ctx.function_key_pressed(num: u8) -> bool // F1-F12 功能键
```

#### 组合键
```rust
ctx.ctrl_key(key) -> bool                // Ctrl + 键
ctx.shift_key(key) -> bool               // Shift + 键
ctx.alt_key(key) -> bool                 // Alt + 键
```

#### 常用快捷键
```rust
ctx.is_copy() -> bool                    // Ctrl+C
ctx.is_paste() -> bool                   // Ctrl+V
ctx.is_cut() -> bool                     // Ctrl+X
ctx.is_undo() -> bool                    // Ctrl+Z
ctx.is_redo() -> bool                    // Ctrl+Y 或 Ctrl+Shift+Z
ctx.is_select_all() -> bool              // Ctrl+A
ctx.is_save() -> bool                    // Ctrl+S
ctx.is_open() -> bool                    // Ctrl+O
ctx.is_new() -> bool                     // Ctrl+N
ctx.is_find() -> bool                    // Ctrl+F
```

### 6. 网络事件

#### 基础网络事件访问
```rust
ctx.net_events() -> &CategorizedEvents   // 获取所有网络事件
ctx.net_event_count() -> usize           // 网络事件总数
ctx.has_net_events() -> bool             // 是否有网络事件
```

#### 连接事件
```rust
ctx.connection_events() -> &[GameEvent]  // 获取连接事件
ctx.has_connection_events() -> bool      // 是否有连接事件
ctx.is_disconnected() -> bool            // 是否已断开连接
```

#### 认证事件
```rust
ctx.auth_events() -> &[GameEvent]        // 获取认证事件
ctx.has_auth_events() -> bool            // 是否有认证事件
ctx.is_login_success() -> bool           // 是否登录成功
ctx.is_login_failed() -> bool            // 是否登录失败
```

#### 角色管理事件
```rust
ctx.character_events() -> &[GameEvent]   // 获取角色管理事件
ctx.has_character_events() -> bool       // 是否有角色管理事件
ctx.has_user_information() -> bool       // 是否收到用户信息
```

#### 玩家状态事件
```rust
ctx.player_state_events() -> &[GameEvent] // 获取玩家状态事件
ctx.has_player_state_events() -> bool    // 是否有玩家状态事件
```

#### 战斗事件
```rust
ctx.combat_events() -> &[GameEvent]      // 获取战斗事件
ctx.has_combat_events() -> bool          // 是否有战斗事件
```

#### 聊天事件
```rust
ctx.chat_events() -> &[GameEvent]        // 获取聊天事件
ctx.has_chat_events() -> bool            // 是否有聊天消息
```

#### 世界对象事件
```rust
ctx.world_object_events() -> &[GameEvent] // 获取世界对象事件
ctx.has_world_object_events() -> bool    // 是否有世界对象事件
```

#### 地图事件
```rust
ctx.map_events() -> &[GameEvent]         // 获取地图事件
ctx.has_map_events() -> bool             // 是否有地图事件
ctx.has_map_changed() -> bool            // 是否有地图切换事件
```

#### 物品事件
```rust
ctx.item_events() -> &[GameEvent]        // 获取物品事件
ctx.has_item_events() -> bool            // 是否有物品事件
```

#### NPC 事件
```rust
ctx.npc_events() -> &[GameEvent]         // 获取 NPC 事件
ctx.has_npc_events() -> bool             // 是否有 NPC 事件
```

#### 其他事件
```rust
ctx.other_events() -> &[GameEvent]       // 获取其他事件
ctx.has_other_events() -> bool           // 是否有其他事件
```

#### 事件查询
```rust
ctx.iter_all_net_events()                // 遍历所有网络事件
ctx.find_event(predicate)                // 查找特定事件
ctx.filter_events(predicate)             // 过滤特定事件
```

### 7. 输入事件

```rust
ctx.input_events() -> &[InputEvent]      // 获取本帧的输入事件列表
ctx.push_input_event(event)              // 添加输入事件
ctx.clear_frame_events()                 // 清空输入事件
```

### 8. 工具方法

```rust
ctx.point_in_rect(x, y, rx, ry, rw, rh) -> bool // 点是否在矩形内
ctx.distance(x1, y1, x2, y2) -> f32      // 两点距离
```

### 9. 资源访问

```rust
ctx.settings() -> Option<Ref<ClientSettings>>  // 获取客户端设置
ctx.network() -> Ref<NetContext>         // 获取网络上下文
ctx.collect_network_events()             // 收集网络事件
ctx.as_ggez_context() -> &mut Context    // 获取 ggez::Context 引用
```

## 使用示例

### 基础示例

```rust
impl ggez::event::EventHandler<CustomContext> for MyGame {
    fn update(&mut self, ctx: &mut CustomContext) -> GameResult {
        // 收集网络事件
        ctx.collect_network_events();
        
        // 方式 1: 直接在 CustomContext 上调用方法
        if ctx.escape_pressed() {
            println!("ESC 键被按下");
        }
        
        // 方式 2: 使用 InputContext（推荐，语义更清晰）
        let input = ctx.input();
        
        // 检查 WASD 方向
        let (x, y) = input.wasd_direction();
        if x != 0 || y != 0 {
            println!("移动方向: ({}, {})", x, y);
        }
        
        // 检查鼠标
        if input.mouse_left_pressed() {
            let (mx, my) = input.mouse_position();
            println!("鼠标左键在 ({}, {}) 按下", mx, my);
        }
        
        // 检查快捷键
        if input.is_save() {
            println!("Ctrl+S 保存");
        }
        
        // 检查网络事件
        if ctx.is_login_success() {
            println!("登录成功！");
        }
        
        // 访问 ECS World
        for (entity, player) in ctx.world.query::<&Player>().iter() {
            // 处理玩家实体
        }
        
        Ok(())
    }
    
    fn draw(&mut self, ctx: &mut CustomContext) -> GameResult {
        // 获取 ggez::Context 用于渲染
        let ggez_ctx = ctx.as_ggez_context();
        let mut canvas = ggez::graphics::Canvas::from_frame(ggez_ctx, Color::BLACK);
        
        // 渲染逻辑...
        
        canvas.finish(ggez_ctx)?;
        Ok(())
    }
    
    fn mouse_button_down_event(
        &mut self,
        ctx: &mut CustomContext,
        button: MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
        // 添加输入事件
        ctx.push_input_event(InputEvent::MouseButtonDown { button, x, y });
        Ok(())
    }
}
```

### 创建 CustomContext

```rust
let settings = ClientSettings::load_or_default();
let (mut ctx, event_loop) = ggez::ContextBuilder::new("game", "author")
    .custom_build::<CustomContext>(|game_id, conf, fs, _| {
        CustomContext::builder(game_id, conf, fs, settings)
    })?;

let state = MyGame::new(&mut ctx)?;
ggez::event::run(ctx, event_loop, state);
```

## InputContext - 输入上下文辅助器

`CustomContext` 提供了两种访问输入方法的方式：

### 方式 1: 直接调用（便捷方法）

```rust
ctx.escape_pressed()      // 直接在 CustomContext 上调用
ctx.mouse_left_pressed()
ctx.wasd_direction()
```

### 方式 2: 通过 InputContext（推荐）

```rust
let input = ctx.input();  // 获取 InputContext
input.escape_pressed()    // 通过 InputContext 调用
input.mouse_left_pressed()
input.wasd_direction()
```

**推荐使用方式 2**，因为：
- 语义更清晰（明确表示这是输入相关操作）
- 与 `GameContext` 的 API 保持一致
- 更容易重构和维护

## 迁移指南

从 `GameContext` 迁移到 `CustomContext` 非常简单，API 完全兼容：

**之前（使用 GameContext）:**
```rust
fn my_system(ctx: &mut GameContext) {
    if ctx.input().escape_pressed() {
        // ...
    }
}
```

**之后（使用 CustomContext）:**
```rust
impl EventHandler<CustomContext> for MyState {
    fn update(&mut self, ctx: &mut CustomContext) -> GameResult {
        // 方式 1: 保持与 GameContext 一致
        if ctx.input().escape_pressed() {
            // ...
        }
        
        // 方式 2: 直接调用（更简洁）
        if ctx.escape_pressed() {
            // ...
        }
        
        Ok(())
    }
}
```

## 设计优势

1. **零拷贝访问** - 直接访问 ggez 组件，无需每帧克隆
2. **统一接口** - 所有资源（输入、网络、ECS）在同一个 Context 中
3. **类型安全** - 通过 Has/HasMut trait 保证类型安全
4. **便捷方法** - 丰富的便捷方法简化常见操作
5. **与 ggez 兼容** - 完全兼容 ggez 的事件系统

## 注意事项

1. `frame_input_events` 需要在每帧开始时清空（调用 `clear_frame_events()`）
2. 网络事件需要手动收集（调用 `collect_network_events()`）
3. `as_ggez_context()` 使用 unsafe 代码，但内存布局保证安全
4. 所有便捷方法都是零开销抽象，直接内联

## 性能特征

- **输入查询**: O(1) - 直接访问键盘/鼠标状态
- **网络事件**: O(n) - 线性遍历事件列表
- **ECS 查询**: 取决于 hecs 的查询性能
- **内存开销**: 约 1KB（事件缓冲）+ World 大小

## 相关文档

- [CUSTOM_CONTEXT_GUIDE.md](./CUSTOM_CONTEXT_GUIDE.md) - 使用指南
- [ARCHITECTURE.md](./ARCHITECTURE.md) - 架构文档
- [GameContext API](./src/ecs/game_context.rs) - 原始 GameContext 实现
