# CustomContext 使用指南

## 概述

`CustomContext` 是对 `ggez::Context` 的自定义包装，将 ECS World 和网络上下文集成到 Context 中，支持直接用于 `ggez::event::run`。

## 核心特性

1. **完全兼容 ggez** - 实现了所有 `Has`/`HasMut` trait
2. **集成 ECS** - 内置 `hecs::World`
3. **集成网络** - 内置 `NetContext`
4. **事件缓冲** - 内置输入事件缓冲区

## 基本用法

### 1. 创建 CustomContext

```rust
use mir2_client::ecs::CustomContext;

fn main() -> ggez::GameResult {
    let cb = ggez::ContextBuilder::new("crystal", "gqf2008");
    
    // 使用 custom_build 创建 CustomContext
    let (mut ctx, event_loop) = cb.custom_build::<CustomContext>(CustomContext::builder)?;
    
    let state = MyState::new(&mut ctx)?;
    ggez::event::run(ctx, event_loop, state)
}
```

### 2. 实现 EventHandler

```rust
use mir2_client::ecs::{CustomContext, GameContext};

struct MyState {
    // 游戏状态
}

impl ggez::event::EventHandler<CustomContext> for MyState {
    fn update(&mut self, ctx: &mut CustomContext) -> ggez::GameResult {
        // 方式 1: 直接访问 World
        for (entity, pos) in ctx.world.query::<&mut Position>().iter() {
            // 处理实体
        }
        
        // 方式 2: 创建 GameContext（推荐用于系统）
        let mut game_ctx = {
            use mir2_client::ecs::WorldExt;
            use std::mem;
            
            // 接收网络事件
            let net_events = {
                let net_ctx = ctx.world.network();
                net_ctx.recv_categorized()
            };
            
            // 移出输入事件
            let input_events = mem::take(&mut ctx.frame_input_events);
            
            GameContext {
                ctx: ctx.as_ggez_context(),
                world: &mut ctx.world,
                net_events,
                input_events,
            }
        };
        
        // 使用 GameContext
        if game_ctx.input().key_pressed(KeyCode::Space) {
            tracing::info!("空格键按下");
        }
        
        // 运行系统
        my_system.update(&mut game_ctx, ctx.time.delta().as_secs_f32())?;
        
        // 清空事件
        ctx.clear_frame_events();
        
        Ok(())
    }
    
    fn draw(&mut self, ctx: &mut CustomContext) -> ggez::GameResult {
        // 使用 ggez 的渲染 API
        let mut canvas = graphics::Canvas::from_frame(
            ctx, 
            graphics::Color::BLACK
        );
        
        // 渲染...
        
        canvas.finish(ctx)?;
        Ok(())
    }
    
    // 输入事件收集
    fn mouse_button_down_event(
        &mut self,
        ctx: &mut CustomContext,
        button: MouseButton,
        x: f32,
        y: f32,
    ) -> ggez::GameResult {
        ctx.push_input_event(InputEvent::MouseMove { x, y, dx: 0.0, dy: 0.0 });
        Ok(())
    }
    
    fn mouse_wheel_event(
        &mut self,
        ctx: &mut CustomContext,
        x: f32,
        y: f32,
    ) -> ggez::GameResult {
        ctx.push_input_event(InputEvent::MouseWheel { x, y });
        Ok(())
    }
    
    fn text_input_event(
        &mut self,
        ctx: &mut CustomContext,
        character: char,
    ) -> ggez::GameResult {
        ctx.push_input_event(InputEvent::Ime { character, preedit: None });
        Ok(())
    }
}
```

## 高级用法

### 访问游戏资源

```rust
impl ggez::event::EventHandler<CustomContext> for MyState {
    fn update(&mut self, ctx: &mut CustomContext) -> ggez::GameResult {
        // 访问 ClientSettings
        if let Some(settings) = ctx.settings() {
            tracing::info!("分辨率: {}x{}", settings.width, settings.height);
        }
        
        // 访问 NetContext
        if let Some(network) = ctx.network() {
            if network.is_connected() {
                tracing::info!("网络已连接");
            }
        }
        
        // 可变访问 NetContext
        if let Some(mut network) = ctx.network_mut() {
            network.send(/* ... */);
        }
        
        Ok(())
    }
}
```

### 使用系统调度器

```rust
use mir2_client::ecs::systems::SystemScheduler;

struct MyState {
    scheduler: SystemScheduler,
}

impl MyState {
    fn new(ctx: &mut CustomContext) -> ggez::GameResult<Self> {
        let mut scheduler = SystemScheduler::new();
        scheduler
            .add_system(PlayerControlSystem::new())
            .add_system(MovementSystem)
            .add_system(CameraSystem::new());
        
        Ok(Self { scheduler })
    }
}

impl ggez::event::EventHandler<CustomContext> for MyState {
    fn update(&mut self, ctx: &mut CustomContext) -> ggez::GameResult {
        // 创建 GameContext
        let mut game_ctx = create_game_context(ctx);
        
        // 运行所有系统
        let dt = ctx.time.delta().as_secs_f32();
        self.scheduler.run_all(&mut game_ctx, dt)?;
        
        // 清空事件
        ctx.clear_frame_events();
        
        Ok(())
    }
}

// 辅助函数：创建 GameContext
fn create_game_context(ctx: &mut CustomContext) -> GameContext<'_> {
    use mir2_client::ecs::WorldExt;
    use std::mem;
    
    let net_events = {
        let net_ctx = ctx.world.network();
        net_ctx.recv_categorized()
    };
    
    let input_events = mem::take(&mut ctx.frame_input_events);
    
    GameContext {
        ctx: ctx.as_ggez_context(),
        world: &mut ctx.world,
        net_events,
        input_events,
    }
}
```

## 与 ggez::Context 的区别

| 特性 | ggez::Context | CustomContext |
|------|---------------|---------------|
| 渲染 API | ✅ | ✅ (通过 Has trait) |
| 输入 API | ✅ | ✅ (通过 Has trait) |
| 时间 API | ✅ | ✅ (通过 Has trait) |
| ECS World | ❌ | ✅ (内置) |
| 网络上下文 | ❌ | ✅ (内置) |
| 输入事件缓冲 | ❌ | ✅ (frame_input_events) |
| event::run 兼容 | ✅ | ✅ (完全兼容) |

## 性能考虑

1. **零拷贝访问** - `as_ggez_context()` 使用 transmute，无额外开销
2. **事件缓冲** - `frame_input_events` 避免每帧分配，使用 `mem::take` 移动所有权
3. **网络事件** - 每帧调用一次 `recv_categorized()`，事件已预分类

## 最佳实践

1. **事件清理** - 每帧结束调用 `ctx.clear_frame_events()`
2. **GameContext 创建** - 使用辅助函数统一创建逻辑
3. **系统组织** - 使用 `SystemScheduler` 管理多个系统
4. **输入收集** - 在所有输入事件回调中调用 `ctx.push_input_event()`

## 完整示例

参考 `src/bin/map_viewer_v3.rs` 获取完整的使用示例。

## 注意事项

- `as_ggez_context()` 使用了 `unsafe` transmute，依赖 Has/HasMut trait 的正确实现
- `CustomContext` 必须与 `ggez::Context` 的内存布局保持一致
- 不要在多个地方同时持有 `GameContext`，会导致借用冲突
