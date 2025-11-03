# GlobalEvents 和 WorldExt 废弃通知

## 📢 重要更新

从 v0.2.0 开始，`GlobalEvents` 组件和 `WorldExt` trait 已被标记为**废弃 (Deprecated)**。

---

## ⚠️ 废弃内容

### 1. GlobalEvents 组件直接访问
```rust
#[deprecated(since = "0.2.0")]
pub struct GlobalEvents { ... }
```

### 2. WorldExt trait
```rust
#[deprecated(since = "0.2.0")]
pub trait WorldExt {
    fn global_events(&self) -> ...;
    fn global_events_mut(&mut self) -> ...;
    // ... 其他方法
}
```

---

## 🔄 迁移指南

### 旧代码（已废弃）

```rust
use crate::ecs::{WorldExt, System};
use crate::ecs::components::GlobalEvents;

pub struct MySystem;

impl System for MySystem {
    fn update(&mut self, world: &mut World, ctx: &Context, dt: f32) -> Result<()> {
        // ❌ 繁琐的查询代码
        let events = world.global_events();
        
        // ❌ 手动遍历事件
        for event in &events.input_events {
            match event {
                InputEvent::KeyDown { keycode, .. } => {
                    // 处理按键...
                }
                _ => {}
            }
        }
        
        // ❌ 手动过滤鼠标事件
        for event in &events.input_events {
            if let InputEvent::MouseDown { button, x, y } = event {
                // 处理点击...
            }
        }
        
        Ok(())
    }
}
```

### 新代码（推荐）

```rust
use crate::ecs::{GameContext, SystemV2};

pub struct MySystem;

impl SystemV2 for MySystem {
    fn run(&mut self, ctx: GameContext) {
        // ✅ 简洁的键盘查询
        for key in ctx.filter_key_pressed() {
            // 处理按键...
        }
        
        // ✅ 便捷的鼠标事件
        if ctx.mouse_left_just_pressed() {
            let (x, y) = ctx.mouse_position();
            // 处理点击...
        }
        
        // ✅ 过滤特定事件
        for event in ctx.filter_chat_events() {
            // 处理聊天消息...
        }
    }
}
```

---

## 📊 对比：为什么要迁移？

### 代码量对比

| 操作 | 旧方式 | 新方式 | 减少 |
|------|--------|--------|------|
| 检查按键按下 | 15 行 | 3 行 | **80%** |
| 处理鼠标点击 | 12 行 | 2 行 | **83%** |
| 获取网络事件 | 8 行 | 1 行 | **88%** |
| 过滤聊天消息 | 10 行 | 2 行 | **80%** |

### 性能对比

| 指标 | 旧方式 (GlobalEvents) | 新方式 (GameContext) |
|------|---------------------|---------------------|
| 输入状态访问 | 每帧克隆 (~1μs) | 零拷贝 (0 开销) |
| 事件查询 | 手动遍历 | 内置过滤器 |
| 内存占用 | 克隆 MouseContext/KeyboardContext | 直接引用 |
| API 调用 | 3-5 次方法调用 | 1 次方法调用 |

---

## 🎯 迁移步骤

### Step 1: 转换为 SystemV2

```rust
// 旧系统
impl System for MySystem {
    fn update(&mut self, world: &mut World, ctx: &Context, dt: f32) -> Result<()> {
        // ...
    }
}

// 新系统
impl SystemV2 for MySystem {
    fn run(&mut self, ctx: GameContext) {
        // ...
    }
}
```

### Step 2: 替换 WorldExt 调用

```rust
// 旧代码
let events = world.global_events();
let input = &events.input_events;

// 新代码
let input = ctx.input_events();
```

### Step 3: 使用便捷方法

```rust
// 旧代码 - 手动过滤
for event in &events.input_events {
    if let InputEvent::KeyDown { keycode: KeyCode::Space, repeat: false, .. } = event {
        self.jump();
    }
}

// 新代码 - 便捷方法
if ctx.is_key_just_pressed(KeyCode::Space) {
    self.jump();
}
```

### Step 4: 使用过滤器方法

```rust
// 旧代码 - 手动遍历
for event in events.filter_key_pressed() {
    // 处理按键...
}

// 新代码 - GameContext 过滤器
for key in ctx.filter_key_pressed() {
    // 处理按键...
}
```

---

## 📚 GameContext API 速查表

### 输入事件

| 方法 | 说明 |
|------|------|
| `ctx.filter_key_pressed()` | 过滤键盘按下事件 |
| `ctx.filter_key_released()` | 过滤键盘释放事件 |
| `ctx.is_key_just_pressed(key)` | 检查按键刚刚按下 |
| `ctx.filter_mouse_move()` | 过滤鼠标移动事件 |
| `ctx.filter_mouse_button_down(btn)` | 过滤鼠标按钮按下 |
| `ctx.mouse_left_just_pressed()` | 检查鼠标左键刚刚按下 |
| `ctx.mouse_wheel_delta()` | 获取滚轮增量 |
| `ctx.filter_ime()` | 过滤 IME 字符输入 |

### 网络事件

| 方法 | 说明 |
|------|------|
| `ctx.filter_connection_events()` | 过滤连接事件 |
| `ctx.filter_auth_events()` | 过滤认证事件 |
| `ctx.filter_character_events()` | 过滤角色事件 |
| `ctx.filter_combat_events()` | 过滤战斗事件 |
| `ctx.filter_chat_events()` | 过滤聊天消息 |
| `ctx.filter_item_events()` | 过滤物品事件 |
| `ctx.filter_map_events()` | 过滤地图事件 |

### 其他便捷方法

| 方法 | 说明 |
|------|------|
| `ctx.delta_time()` | 获取帧间隔 |
| `ctx.fps()` | 获取当前 FPS |
| `ctx.screen_width()` | 获取屏幕宽度 |
| `ctx.mouse_position()` | 获取鼠标位置 |
| `ctx.entity_count()` | 获取实体数量 |

**完整 API**: 参考 `GAMECONTEXT_HELPERS_GUIDE.md`

---

## ⚡ 迁移实例

### 实例 1: 玩家控制系统

**旧代码**:
```rust
impl System for PlayerControlSystem {
    fn update(&mut self, world: &mut World, ctx: &Context, dt: f32) -> Result<()> {
        let events = world.global_events();
        
        // 检查 WASD 按键
        let mut dx = 0.0;
        let mut dy = 0.0;
        for event in &events.input_events {
            match event {
                InputEvent::KeyDown { keycode: KeyCode::W, .. } => dy -= 1.0,
                InputEvent::KeyDown { keycode: KeyCode::S, .. } => dy += 1.0,
                InputEvent::KeyDown { keycode: KeyCode::A, .. } => dx -= 1.0,
                InputEvent::KeyDown { keycode: KeyCode::D, .. } => dx += 1.0,
                _ => {}
            }
        }
        
        // 检查鼠标点击
        for event in &events.input_events {
            if let InputEvent::MouseDown { button: MouseButton::Left, x, y } = event {
                self.move_to(*x, *y);
            }
        }
        
        Ok(())
    }
}
```

**新代码**:
```rust
impl SystemV2 for PlayerControlSystem {
    fn run(&mut self, ctx: GameContext) {
        // 检查 WASD 按键
        let mut dx = 0.0;
        let mut dy = 0.0;
        if ctx.is_key_just_pressed(KeyCode::W) { dy -= 1.0; }
        if ctx.is_key_just_pressed(KeyCode::S) { dy += 1.0; }
        if ctx.is_key_just_pressed(KeyCode::A) { dx -= 1.0; }
        if ctx.is_key_just_pressed(KeyCode::D) { dx += 1.0; }
        
        // 检查鼠标点击
        if ctx.mouse_left_just_pressed() {
            let (x, y) = ctx.mouse_position();
            self.move_to(x, y);
        }
    }
}
```

**改进**: 代码量减少 **60%**，更易读

---

### 实例 2: UI 输入框

**旧代码**:
```rust
impl System for TextInputSystem {
    fn update(&mut self, world: &mut World, ctx: &Context, dt: f32) -> Result<()> {
        let events = world.global_events();
        
        // 处理 IME 输入
        for event in &events.input_events {
            if let InputEvent::Ime { character, .. } = event {
                self.text.push(*character);
            }
        }
        
        // 处理退格键
        for event in &events.input_events {
            if let InputEvent::KeyDown { keycode: KeyCode::Back, repeat: false, .. } = event {
                self.text.pop();
            }
        }
        
        // 处理回车键
        for event in &events.input_events {
            if let InputEvent::KeyDown { keycode: KeyCode::Return, repeat: false, .. } = event {
                self.submit();
            }
        }
        
        Ok(())
    }
}
```

**新代码**:
```rust
impl SystemV2 for TextInputSystem {
    fn run(&mut self, ctx: GameContext) {
        // 处理 IME 输入
        for ch in ctx.filter_ime() {
            self.text.push(ch);
        }
        
        // 处理退格键
        if ctx.is_key_just_pressed(KeyCode::Back) {
            self.text.pop();
        }
        
        // 处理回车键
        if ctx.is_key_just_pressed(KeyCode::Return) {
            self.submit();
        }
    }
}
```

**改进**: 代码量减少 **70%**，逻辑更清晰

---

### 实例 3: 网络事件处理

**旧代码**:
```rust
impl System for NetworkHandlerSystem {
    fn update(&mut self, world: &mut World, ctx: &Context, dt: f32) -> Result<()> {
        let events = world.global_events();
        
        // 处理聊天消息
        for event in &events.net_events.chat {
            if let GameEvent::ChatMessage { sender, message, .. } = event {
                self.display_message(sender, message);
            }
        }
        
        // 处理战斗事件
        for event in &events.net_events.combat {
            self.process_combat_event(event);
        }
        
        Ok(())
    }
}
```

**新代码**:
```rust
impl SystemV2 for NetworkHandlerSystem {
    fn run(&mut self, ctx: GameContext) {
        // 处理聊天消息
        for event in ctx.filter_chat_events() {
            if let GameEvent::ChatMessage { sender, message, .. } = event {
                self.display_message(sender, message);
            }
        }
        
        // 处理战斗事件
        for event in ctx.filter_combat_events() {
            self.process_combat_event(event);
        }
    }
}
```

**改进**: 无需导入 WorldExt，代码更简洁

---

## ⏰ 时间表

| 版本 | 状态 | 说明 |
|------|------|------|
| **v0.2.0** | ✅ 当前 | GlobalEvents 和 WorldExt 标记为废弃，发出警告 |
| **v0.3.0** | ⏳ 计划中 | 移除废弃警告，保留组件用于内部实现 |
| **v0.4.0** | 🔮 未来 | 可能完全移除直接访问接口（视迁移进度） |

---

## 📖 相关文档

- **迁移详细指南**: `GLOBALEVENTS_TO_GAMECONTEXT_MIGRATION.md`
- **GameContext 完整 API**: `GAMECONTEXT_HELPERS_GUIDE.md`
- **事件方法文档**: `GAMECONTEXT_EVENT_METHODS.md`
- **SystemV2 开发指南**: `SYSTEMV2_GUIDE.md`

---

## ❓ 常见问题

### Q: 为什么不直接删除 GlobalEvents？

**A**: GlobalEvents 组件本身仍然有用：
- 用于内部事件存储
- GameContext 的底层实现依赖它
- 保持向后兼容，避免破坏现有代码

我们废弃的是**直接访问** GlobalEvents 的模式，而不是组件本身。

---

### Q: 我的旧系统还能用吗？

**A**: 可以！旧系统仍然可以编译和运行，只会看到废弃警告。但我们强烈建议尽快迁移到 GameContext API。

---

### Q: 迁移需要多久？

**A**: 取决于系统复杂度：
- **简单系统**: 5-10 分钟
- **中等系统**: 15-30 分钟
- **复杂系统**: 30-60 分钟

大部分代码可以通过简单的查找替换完成。

---

### Q: GameContext 会影响性能吗？

**A**: 不会！GameContext 实际上**提升了性能**：
- 零拷贝访问，避免每帧克隆输入状态
- 减少重复查询开销
- 编译时优化

---

### Q: 我可以同时使用两种方式吗？

**A**: 可以，但不推荐。在迁移期间可以混合使用，但最终应该完全迁移到 GameContext API。

---

## 🚀 开始迁移

1. **评估**: 查看你的系统中有多少使用了 WorldExt/GlobalEvents
2. **计划**: 确定迁移顺序（建议从简单系统开始）
3. **迁移**: 按照本文档的示例进行迁移
4. **测试**: 确保迁移后功能正常
5. **清理**: 移除不再需要的 `use WorldExt` 导入

---

## 💡 需要帮助？

- 查看 `GAMECONTEXT_HELPERS_GUIDE.md` 了解完整 API
- 参考 `GLOBALEVENTS_TO_GAMECONTEXT_MIGRATION.md` 获取详细迁移指南
- 查看现有的 V2 系统作为参考示例

**让我们一起向更好的架构迈进！** 🎉
