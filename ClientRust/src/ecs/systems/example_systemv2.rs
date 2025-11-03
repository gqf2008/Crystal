/// GameContext 使用示例 - 演示如何使用新的零拷贝输入访问
/// 
/// 此文件展示如何将系统从旧的 System trait 迁移到新的 SystemV2 trait

use crate::ecs::{GameContext, SystemV2};
use crate::ecs::components::GlobalEvents;
use ggez::GameResult;
use ggez::input::mouse::MouseButton;

/// 示例系统 - 使用 GameContext 的零拷贝输入访问
pub struct ExampleSystemV2;

impl ExampleSystemV2 {
    pub fn new() -> Self {
        Self
    }
}

impl SystemV2 for ExampleSystemV2 {
    fn priority(&self) -> u32 {
        110  // 在 PlayerControl 之后
    }
    
    fn update(&mut self, ctx: &mut GameContext, _delta_time: f32) -> GameResult {
        // ✅ 新方式 - 直接从 GameContext 访问输入，零拷贝！
        
        // 方式 1: 直接访问 ggez Context
        let mouse_left = ctx.ctx.mouse.button_pressed(MouseButton::Left);
        let mouse_pos = ctx.ctx.mouse.position();
        
        // 方式 2: 使用 InputContext 辅助器
        let input = ctx.input();
        let mouse_right = input.mouse_button_pressed(MouseButton::Right);
        let (x, y) = input.mouse_position();
        
        // 访问 World
        let mut player_query = ctx.world.query::<&GlobalEvents>();
        if let Some((_, events)) = player_query.iter().next() {
            // 处理网络事件等
            tracing::debug!("网络事件数量: {}", events.net_events.server_messages.len());
        }
        
        // 访问网络上下文
        // let network = ctx.network;
        
        if mouse_left {
            tracing::debug!("🖱️ 左键按下, 位置: ({:.1}, {:.1})", mouse_pos.x, mouse_pos.y);
        }
        
        Ok(())
    }
}

// ============================================================================
// 迁移指南
// ============================================================================
//
// ## 如何将系统从 System 迁移到 SystemV2
//
// ### 步骤 1: 修改 trait 声明
// ```rust
// // 旧版本
// impl System for MySystem {
//     fn update(&mut self, world: &mut World, dt: f32) -> GameResult {
//         // ...
//     }
// }
//
// // 新版本
// impl SystemV2 for MySystem {
//     fn update(&mut self, ctx: &mut GameContext, dt: f32) -> GameResult {
//         // ...
//     }
// }
// ```
//
// ### 步骤 2: 替换输入访问方式
// ```rust
// // 旧版本 - 从 GlobalEvents 克隆
// let events = world.global_events();
// let mouse_pressed = events.mouse.button_pressed(MouseButton::Left);
//
// // 新版本 - 直接访问，零拷贝
// let mouse_pressed = ctx.ctx.mouse.button_pressed(MouseButton::Left);
// ```
//
// ### 步骤 3: 访问 World
// ```rust
// // 旧版本
// let mut query = world.query::<&MyComponent>();
//
// // 新版本
// let mut query = ctx.world.query::<&MyComponent>();
// ```
//
// ### 步骤 4: 常见模式迁移
//
// #### 鼠标输入
// ```rust
// // 旧版本
// let events = world.global_events();
// let left = events.mouse.button_pressed(MouseButton::Left);
// let pos = events.mouse.position();
//
// // 新版本
// let left = ctx.ctx.mouse.button_pressed(MouseButton::Left);
// let pos = ctx.ctx.mouse.position();
// ```
//
// #### 键盘输入
// ```rust
// // 旧版本 - 通过 InputEvent 迭代
// let ctrl = events.input_events.iter()
//     .any(|e| matches!(e, InputEvent::KeyDown { keycode: KeyCode::ControlLeft, .. }));
//
// // 新版本 - 使用 keyboard context
// // TODO: ggez 0.9 的键盘 API 需要进一步研究
// ```
//
// #### 网络事件
// ```rust
// // 旧版本和新版本都相同
// let mut query = ctx.world.query::<&GlobalEvents>();
// if let Some((_, events)) = query.iter().next() {
//     for msg in &events.net_events.server_messages {
//         // 处理消息
//     }
// }
// ```
//
// ## 性能提升
//
// - 旧方式: 每帧克隆 MouseContext + KeyboardContext (~1μs)
// - 新方式: 直接引用访问 (几乎零开销)
// - 预期提升: ~96% (参考 CameraSystem 优化)
//
// ## 迁移优先级
//
// 1. **高优先级** (频繁访问输入的系统):
//    - PlayerControlSystem (已部分迁移)
//    - CameraSystem (已部分迁移)
//
// 2. **中优先级** (偶尔访问输入):
//    - AnimationSystem
//    - ParticleSystem
//
// 3. **低优先级** (不访问输入):
//    - MovementSystem
//    - CollisionSystem
//    - 大部分逻辑系统
//
// ## 注意事项
//
// 1. **生命周期**: GameContext 的生命周期确保所有引用在同一帧内有效
// 2. **借用规则**: 不能同时持有 ctx.ctx 和 ctx.world 的多个可变引用
// 3. **向后兼容**: 旧的 System trait 仍然可用，可以渐进式迁移
//
// ============================================================================
