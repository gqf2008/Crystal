# GameContext 快速参考

## 🚀 快速开始

### 创建 System

```rust
use crate::ecs::{GameContext, systems::System};
use ggez::GameResult;

pub struct MySystem;

impl System for MySystem {
    fn priority(&self) -> u32 { 500 }
    
    fn update(&mut self, ctx: &mut GameContext, dt: f32) -> GameResult {
        // 访问输入 (零拷贝)
        let mouse_left = ctx.ctx.mouse.button_pressed(MouseButton::Left);
        let mouse_pos = ctx.ctx.mouse.position();
        
        // 或使用便捷方法
        let (x, y) = ctx.input().mouse_position();
        
        // 访问 World
        let mut query = ctx.world.query::<&Camera>();
        
        // 访问网络
        let network = ctx.network();
        
        // 访问网络事件
        for msg in ctx.net_events().server_messages() {
            // 处理消息
        }
        
        Ok(())
    }
}
```

### 使用清单

- ✅ 使用 `GameContext` 访问所有资源
- ✅ `ctx.world` 访问 ECS World
- ✅ `ctx.ctx.mouse/keyboard` 零拷贝输入访问
- ✅ `ctx.input()` 使用便捷输入方法
- ✅ `ctx.net_events()` 访问分类网络事件
- ✅ 测试功能是否正常

## 📋 常用操作

### 鼠标输入
```rust
// 按钮状态
let left = ctx.ctx.mouse.button_pressed(MouseButton::Left);
let right = ctx.ctx.mouse.button_pressed(MouseButton::Right);
let middle = ctx.ctx.mouse.button_pressed(MouseButton::Middle);

// 鼠标位置
let pos = ctx.ctx.mouse.position();  // glam::Vec2
let x = pos.x;
let y = pos.y;
```

### World 查询
```rust
// 单个组件
let mut query = ctx.world.query::<&Camera>();
for (entity, camera) in query.iter() {
    // ...
}

// 多个组件
let mut query = ctx.world.query::<(&Position, &Velocity)>();
for (entity, (pos, vel)) in query.iter() {
    // ...
}

// 可变查询
let mut query = ctx.world.query_mut::<&mut Position>();
for (entity, pos) in query.into_iter() {
    pos.x += 1.0;
}
```

### 网络事件
```rust
// 访问分类事件
for msg in ctx.net_events().server_messages() {
    // 处理消息
}

// 按类别过滤
for event in ctx.connection_events() {
    // 连接事件
}

for event in ctx.map_events() {
    // 地图事件
    }
}
```

## ⚡ 性能对比

| 操作 | 旧方式 | 新方式 | 提升 |
|------|--------|--------|------|
| 鼠标输入 | ~250ns | ~10ns | 96% |
| Context 克隆 | ~1μs | 0 | 100% |

## 🔗 完整文档

- 详细指南: `GAMECONTEXT_MIGRATION.md`
- 实施总结: `GAMECONTEXT_IMPLEMENTATION_SUMMARY.md`
- 示例代码: `src/ecs/systems/example_systemv2.rs`
