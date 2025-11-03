# GameContext 快速参考

## 🚀 快速开始

### 创建 SystemV2

```rust
use crate::ecs::{GameContext, SystemV2};
use ggez::GameResult;

pub struct MySystem;

impl SystemV2 for MySystem {
    fn priority(&self) -> u32 { 500 }
    
    fn update(&mut self, ctx: &mut GameContext, dt: f32) -> GameResult {
        // 访问输入
        let mouse_left = ctx.ctx.mouse.button_pressed(MouseButton::Left);
        let mouse_pos = ctx.ctx.mouse.position();
        
        // 访问 World
        let mut query = ctx.world.query::<&Camera>();
        
        // 访问网络
        // let net = ctx.network;
        
        Ok(())
    }
}
```

### 迁移清单

- [ ] `impl System` → `impl SystemV2`
- [ ] `world` → `ctx.world`
- [ ] `world.global_events().mouse` → `ctx.ctx.mouse`
- [ ] 测试功能是否正常

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

### 网络事件 (临时)
```rust
let mut query = ctx.world.query::<&GlobalEvents>();
if let Some((_, events)) = query.iter().next() {
    for msg in &events.net_events.server_messages {
        // 处理消息
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
