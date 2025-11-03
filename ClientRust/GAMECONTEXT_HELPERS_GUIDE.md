# GameContext 便捷方法使用指南

**目的**: 展示 GameContext 的便捷 API,简化系统开发

---

## 📋 概览

GameContext 提供了丰富的便捷方法,避免直接访问 ggez API,让代码更简洁易读。

### 方法分类

1. **时间相关** - FPS、帧间隔、运行时间
2. **屏幕尺寸** - 宽度、高度、尺寸
3. **鼠标输入** - 按键状态、位置
4. **ECS 查询** - 实体数量、存在性检查
5. **辅助工具** - 鼠标状态快照、几何计算

---

## 🎯 使用示例

### 1. 时间相关方法

```rust
impl SystemV2 for MySystem {
    fn update(&mut self, ctx: &mut GameContext, dt: f32) -> GameResult {
        // 获取帧间隔 (通常直接用 dt 参数即可)
        let delta = ctx.delta_time();  // f32, 单位: 秒
        
        // 获取当前 FPS
        let fps = ctx.fps();  // f32
        if fps < 30.0 {
            tracing::warn!("FPS 过低: {:.1}", fps);
        }
        
        // 获取游戏运行时间
        let time = ctx.time_since_start();  // f64, 单位: 秒
        
        Ok(())
    }
}
```

### 2. 屏幕尺寸方法

```rust
// 获取屏幕宽度和高度
let width = ctx.screen_width();   // f32
let height = ctx.screen_height(); // f32

// 或者一次性获取
let (w, h) = ctx.screen_size();

// 计算屏幕中心
let center_x = ctx.screen_width() / 2.0;
let center_y = ctx.screen_height() / 2.0;
```

### 3. 鼠标输入方法

```rust
// ===== 按键状态 =====

// 检查鼠标按键
if ctx.mouse_left_pressed() {
    println!("左键按下");
}

if ctx.mouse_right_pressed() {
    println!("右键按下");
}

if ctx.mouse_middle_pressed() {
    println!("中键按下");
}

// ===== 鼠标位置 =====

// 获取鼠标位置
let (mx, my) = ctx.mouse_position();
println!("鼠标位置: ({}, {})", mx, my);
```

### 4. ECS 查询方法

```rust
// 获取实体总数
let count = ctx.entity_count();
println!("场景中有 {} 个实体", count);

// 检查实体是否存在
if ctx.entity_exists(player_entity) {
    // 实体存在,安全操作
}
```

### 5. InputContext 辅助器

```rust
// 获取 InputContext (更多便捷方法)
let input = ctx.input();

// 鼠标方法
if input.mouse_left() {
    let x = input.mouse_x();
    let y = input.mouse_y();
}

// 时间方法
let delta = input.delta();
let fps = input.fps();

// 屏幕方法
let width = input.width();
let height = input.height();

// 鼠标是否在屏幕内
if input.mouse_in_bounds() {
    // 鼠标在窗口内
}
```

### 6. MouseState 快照

```rust
// 当需要多次访问鼠标状态时,使用快照避免重复查询
let mouse = ctx.mouse_state();

println!("位置: ({}, {})", mouse.x(), mouse.y());
println!("左键: {}", mouse.left_pressed);
println!("右键: {}", mouse.right_pressed);
println!("中键: {}", mouse.middle_pressed);
```

### 7. GameContextExt 扩展方法

```rust
use crate::ecs::GameContextExt;

// 判断鼠标是否在矩形内
let button_rect = (100.0, 200.0, 150.0, 50.0);  // (x, y, w, h)
if ctx.mouse_in_rect(button_rect.0, button_rect.1, button_rect.2, button_rect.3) {
    println!("鼠标悬停在按钮上");
}

// 计算鼠标到某点的距离
let target_x = 500.0;
let target_y = 300.0;
let distance = ctx.mouse_distance_to(target_x, target_y);
if distance < 50.0 {
    println!("鼠标接近目标");
}

// 计算两点距离
let dist = ctx.distance(x1, y1, x2, y2);
```

---

## 📝 完整系统示例

### 示例 1: 相机跟随系统

```rust
use crate::ecs::{GameContext, SystemV2, GameContextExt};
use ggez::GameResult;

pub struct CameraFollowSystem;

impl SystemV2 for CameraFollowSystem {
    fn update(&mut self, ctx: &mut GameContext, dt: f32) -> GameResult {
        // 使用便捷方法
        let screen_w = ctx.screen_width();
        let screen_h = ctx.screen_height();
        
        // 查询玩家和相机
        for (_, (player_pos, _)) in ctx.world.query::<(&Position, &LocalPlayer)>().iter() {
            for (_, (camera, cam_pos)) in ctx.world.query::<(&mut Camera, &mut Position)>().iter() {
                // 计算目标位置
                cam_pos.x += (player_pos.x - cam_pos.x) * dt * 5.0;
                cam_pos.y += (player_pos.y - cam_pos.y) * dt * 5.0;
                
                // 更新相机尺寸
                camera.screen_width = screen_w;
                camera.screen_height = screen_h;
            }
        }
        
        Ok(())
    }
    
    fn priority(&self) -> u32 {
        500
    }
}
```

### 示例 2: UI 点击检测系统

```rust
use crate::ecs::{GameContext, SystemV2, GameContextExt};
use ggez::GameResult;

pub struct UIClickSystem;

impl SystemV2 for UIClickSystem {
    fn update(&mut self, ctx: &mut GameContext, _dt: f32) -> GameResult {
        // 使用 MouseState 快照
        let mouse = ctx.mouse_state();
        
        if mouse.left_pressed {
            // 遍历所有 UI 按钮
            for (_, (button, pos)) in ctx.world.query::<(&Button, &Position)>().iter() {
                // 使用扩展方法检测点击
                if ctx.mouse_in_rect(pos.x, pos.y, button.width, button.height) {
                    tracing::info!("按钮被点击: {}", button.label);
                    // 触发按钮事件
                }
            }
        }
        
        Ok(())
    }
    
    fn priority(&self) -> u32 {
        100
    }
}
```

### 示例 3: 性能监控系统

```rust
use crate::ecs::{GameContext, SystemV2};
use ggez::GameResult;

pub struct PerformanceMonitor {
    last_check: f64,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self { last_check: 0.0 }
    }
}

impl SystemV2 for PerformanceMonitor {
    fn update(&mut self, ctx: &mut GameContext, _dt: f32) -> GameResult {
        let now = ctx.time_since_start();
        
        // 每秒检查一次
        if now - self.last_check >= 1.0 {
            let fps = ctx.fps();
            let entities = ctx.entity_count();
            
            tracing::info!(
                "性能统计 | FPS: {:.1} | 实体: {} | 运行时间: {:.1}秒",
                fps, entities, now
            );
            
            self.last_check = now;
        }
        
        Ok(())
    }
    
    fn priority(&self) -> u32 {
        u32::MAX - 100  // 低优先级
    }
}
```

---

## 🎨 代码风格对比

### ❌ 旧方式 (直接访问 ggez API)

```rust
// 冗长且不直观
let screen_size = ctx.ctx.gfx.drawable_size();
let screen_w = screen_size.0;
let screen_h = screen_size.1;

let mouse_pos = ctx.ctx.mouse.position();
let mouse_x = mouse_pos.x;
let mouse_y = mouse_pos.y;

let left_pressed = ctx.ctx.mouse.button_pressed(MouseButton::Left);
let fps = ctx.ctx.time.fps() as f32;
```

### ✅ 新方式 (使用便捷方法)

```rust
// 简洁清晰
let (screen_w, screen_h) = ctx.screen_size();
let (mouse_x, mouse_y) = ctx.mouse_position();
let left_pressed = ctx.mouse_left_pressed();
let fps = ctx.fps();
```

---

## 🔧 扩展建议

如果你需要添加更多便捷方法,可以:

### 1. 扩展 GameContext

在 `game_context.rs` 中添加:

```rust
impl<'a> GameContext<'a> {
    /// 你的自定义方法
    pub fn my_helper_method(&self) -> f32 {
        // 实现
    }
}
```

### 2. 实现 GameContextExt trait

```rust
pub trait GameContextExt<'a> {
    fn my_domain_method(&self) -> bool;
}

impl<'a> GameContextExt<'a> for GameContext<'a> {
    fn my_domain_method(&self) -> bool {
        // 实现
    }
}
```

### 3. 创建新的辅助结构体

```rust
pub struct MyHelper<'a> {
    ctx: &'a Context,
}

impl<'a> MyHelper<'a> {
    pub fn do_something(&self) {
        // ...
    }
}

impl<'a> GameContext<'a> {
    pub fn my_helper(&self) -> MyHelper<'_> {
        MyHelper::new(self.ctx)
    }
}
```

---

## 📚 API 参考

### GameContext 方法

| 方法 | 返回类型 | 说明 |
|------|----------|------|
| `delta_time()` | `f32` | 帧间隔 (秒) |
| `time_since_start()` | `f64` | 游戏运行时间 (秒) |
| `fps()` | `f32` | 当前 FPS |
| `screen_width()` | `f32` | 屏幕宽度 |
| `screen_height()` | `f32` | 屏幕高度 |
| `screen_size()` | `(f32, f32)` | 屏幕尺寸 (宽, 高) |
| `mouse_left_pressed()` | `bool` | 鼠标左键是否按下 |
| `mouse_right_pressed()` | `bool` | 鼠标右键是否按下 |
| `mouse_middle_pressed()` | `bool` | 鼠标中键是否按下 |
| `mouse_position()` | `(f32, f32)` | 鼠标位置 |
| `entity_count()` | `usize` | 实体数量 |
| `entity_exists(entity)` | `bool` | 实体是否存在 |
| `input()` | `InputContext` | 输入辅助器 |
| `mouse_state()` | `MouseState` | 鼠标状态快照 |

### InputContext 方法

| 方法 | 返回类型 | 说明 |
|------|----------|------|
| `mouse_left()` | `bool` | 左键按下 |
| `mouse_right()` | `bool` | 右键按下 |
| `mouse_middle()` | `bool` | 中键按下 |
| `mouse_x()` | `f32` | 鼠标 X 坐标 |
| `mouse_y()` | `f32` | 鼠标 Y 坐标 |
| `mouse_position()` | `(f32, f32)` | 鼠标位置 |
| `delta()` | `f32` | 帧间隔 |
| `fps()` | `f32` | FPS |
| `width()` | `f32` | 屏幕宽度 |
| `height()` | `f32` | 屏幕高度 |
| `size()` | `(f32, f32)` | 屏幕尺寸 |
| `mouse_in_bounds()` | `bool` | 鼠标是否在屏幕内 |

### GameContextExt 方法

| 方法 | 返回类型 | 说明 |
|------|----------|------|
| `mouse_in_rect(x, y, w, h)` | `bool` | 鼠标是否在矩形内 |
| `mouse_distance_to(x, y)` | `f32` | 鼠标到点的距离 |
| `distance(x1, y1, x2, y2)` | `f32` | 两点距离 |
| `point_in_rect(...)` | `bool` | 点是否在矩形内 |

---

## ⚠️ 注意事项

### 1. 滚轮事件

鼠标滚轮通过 `InputContext` 访问:

```rust
// ✅ 使用 InputContext
for (x, y) in ctx.input().mouse_wheel() {
    tracing::info!("滚轮: x={}, y={}", x, y);
}
```

### 2. 键盘输入

键盘输入通过 `InputContext` 的便捷方法访问:

```rust
// 键盘状态查询
if ctx.input().key_pressed(KeyCode::W) { /* ... */ }
if ctx.input().ctrl_pressed() { /* ... */ }

// 按键事件迭代
for (key, text) in ctx.input().pressed_keys() {
    // 处理按键
}
```

### 3. 性能考虑

- `mouse_state()` 会创建快照,如果只需要一两个值,直接调用对应方法更高效
- `entity_count()` 是 O(1) 操作,可以频繁调用
- 扩展方法 (`GameContextExt`) 不会产生额外开销

---

## 🎉 总结

GameContext 便捷方法让代码:
- ✅ **更简洁** - 减少样板代码
- ✅ **更易读** - 意图清晰
- ✅ **更安全** - 统一接口,减少错误
- ✅ **零开销** - 编译期内联,无性能损失

享受更流畅的开发体验! 🚀
