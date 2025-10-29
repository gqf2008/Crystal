// ============================================================================
// Input Module Usage Example
// 展示如何使用 input 模块的4个系统
// ============================================================================

use crate::ecs::systems::update::input::*;
use crate::ecs::systems::System;
use hecs::World;

/// 示例：如何在游戏场景中使用 input 模块
pub fn example_usage() {
    let mut world = World::new();
    
    // 1. 创建系统实例
    let mut network_recv = NetworkRecvSystem::new();
    let mut player_control = PlayerControlSystem;
    let mut game_event = GameEventSystem::new();
    
    // 2. 设置网络接收器（从 NetworkManager 获取）
    // let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
    // network_recv.set_receiver(event_rx);
    
    // 3. 每帧更新（按优先级顺序）
    let delta_time = 0.016; // 60 FPS
    
    // 优先级 50: 网络接收
    let _ = network_recv.update(&mut world, delta_time);
    
    // 优先级 100: 输入收集（需要 ggez::Context）
    // InputSystem::update(&mut world, ctx);
    
    // 优先级 110: 玩家控制
    let _ = player_control.update(&mut world, delta_time);
    
    // 优先级 120: 游戏事件
    let _ = game_event.update(&mut world, delta_time);
}

/// 系统优先级说明
/// 
/// Layer 1 (Input & Network) - 50-199
/// ├─ NetworkRecvSystem(50)    - 接收网络事件
/// ├─ InputSystem(100)         - 处理鼠标/键盘输入
/// ├─ PlayerControlSystem(110) - 转换输入为游戏指令
/// └─ GameEventSystem(120)     - 分发事件到其他系统
/// 
/// 数据流：
/// 网络数据 → NetworkRecvSystem → GameEventSystem
/// 用户输入 → InputSystem → PlayerInput → PlayerControlSystem → Player组件
