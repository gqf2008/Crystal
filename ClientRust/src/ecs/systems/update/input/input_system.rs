// ============================================================================
// Input System - 输入系统 (重构版)
// ============================================================================
//
// **新职责** (GlobalEvents 架构):
// - 从 GGEZ 事件接收原始输入 (键盘/鼠标)
// - 将输入事件写入 GlobalEvents 组件
// - 不再直接修改 MouseInput/KeyboardInput 组件
// - 不处理游戏逻辑（由 PlayerControlSystem 等处理）
//
// **数据流**:
// ```
// GGEZ Events → InputSystem
//     ↓
// GlobalEvents.keyboard_events
// GlobalEvents.mouse_events  
//     ↓
// PlayerControlSystem/其他系统读取并处理
//     ↓
// EventCleanupSystem 清理
// ```
//
// **设计理念**:
// - InputSystem 只做 "输入捕获" 和 "事件记录"
// - 所有输入逻辑（双击检测、长按检测）由消费系统处理
// - GlobalEvents 作为事件总线
//
// ============================================================================

use hecs::World;
use ggez::Context;
use ggez::winit::keyboard::KeyCode;
use ggez::winit::event::MouseButton;
use ggez::GameResult;

use crate::ecs::components::{GlobalEvents, KeyboardEvent, MouseEvent, ImeEvent};
use crate::ecs::systems::System;

/// 输入系统 (重构版)
/// 
/// 职责: 将 GGEZ 事件写入 GlobalEvents
pub struct InputSystem;

impl InputSystem {
    /// 创建新的输入系统
    pub fn new() -> Self {
        Self
    }
    
    /// 向后兼容的update方法（空实现）
    /// 
    /// 新架构下，InputSystem 通过 GGEZ 事件回调调用 process_xxx 方法
    /// 此方法仅用于兼容旧调度器
    pub fn update(_world: &mut World, _ctx: &mut Context) -> GameResult {
        Ok(())
    }
    
    /// 处理鼠标按下事件 (写入 GlobalEvents)
    pub fn process_mouse_down(
        world: &mut World,
        button: MouseButton,
        x: f32,
        y: f32,
    ) {
        if let Some((_, events)) = world.query_mut::<&mut GlobalEvents>().into_iter().next() {
            events.mouse_events.push(MouseEvent::ButtonDown {
                button,
                x,
                y,
            });
            tracing::trace!("🖱️ 鼠标按下: {:?} at ({:.1}, {:.1})", button, x, y);
        }
    }
    
    /// 处理鼠标抬起事件 (写入 GlobalEvents)
    pub fn process_mouse_up(
        world: &mut World,
        button: MouseButton,
        x: f32,
        y: f32,
    ) {
        if let Some((_, events)) = world.query_mut::<&mut GlobalEvents>().into_iter().next() {
            events.mouse_events.push(MouseEvent::ButtonUp {
                button,
                x,
                y,
            });
            tracing::trace!("🖱️ 鼠标抬起: {:?} at ({:.1}, {:.1})", button, x, y);
        }
    }
    
    /// 处理鼠标移动 (写入 GlobalEvents)
    pub fn process_mouse_move(
        world: &mut World,
        x: f32,
        y: f32,
        dx: f32,
        dy: f32,
    ) {
        if let Some((_, events)) = world.query_mut::<&mut GlobalEvents>().into_iter().next() {
            events.mouse_events.push(MouseEvent::Move {
                x,
                y,
                dx,
                dy,
            });
            // 鼠标移动事件太频繁，不记录日志
        }
    }
    
    /// 处理鼠标滚轮 (写入 GlobalEvents)
    pub fn process_mouse_wheel(
        world: &mut World,
        x: f32,
        y: f32,
    ) {
        if let Some((_, events)) = world.query_mut::<&mut GlobalEvents>().into_iter().next() {
            events.mouse_events.push(MouseEvent::Wheel {
                x,
                y,
            });
            tracing::trace!("🖱️ 鼠标滚轮: ({:.1}, {:.1})", x, y);
        }
    }
    
    /// 处理键盘按键按下 (写入 GlobalEvents)
    pub fn process_key_down(
        world: &mut World,
        keycode: KeyCode,
        repeat: bool,
    ) {
        if let Some((_, events)) = world.query_mut::<&mut GlobalEvents>().into_iter().next() {
            events.keyboard_events.push(KeyboardEvent {
                keycode,
                pressed: true,
                repeat,
                timestamp: std::time::Instant::now(),
            });
            tracing::trace!("⌨️ 键盘按下: {:?} (repeat={})", keycode, repeat);
        }
    }
    
    /// 处理键盘按键释放 (写入 GlobalEvents)
    pub fn process_key_up(
        world: &mut World,
        keycode: KeyCode,
    ) {
        if let Some((_, events)) = world.query_mut::<&mut GlobalEvents>().into_iter().next() {
            events.keyboard_events.push(KeyboardEvent {
                keycode,
                pressed: false,
                repeat: false,
                timestamp: std::time::Instant::now(),
            });
            tracing::trace!("⌨️ 键盘释放: {:?}", keycode);
        }
    }
    
    /// 处理文本输入 (IME) (写入 GlobalEvents)
    pub fn process_text_input(
        world: &mut World,
        character: char,
    ) {
        if let Some((_, events)) = world.query_mut::<&mut GlobalEvents>().into_iter().next() {
            events.ime_events.push(ImeEvent {
                character,
                timestamp: std::time::Instant::now(),
            });
            tracing::trace!("📝 文本输入: '{}'", character);
        }
    }
}

impl Default for InputSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl System for InputSystem {
    fn name(&self) -> &'static str {
        "InputSystem"
    }
    
    fn priority(&self) -> u32 {
        crate::ecs::systems::priority::INPUT
    }

    fn update(&mut self, world: &mut hecs::World, _delay_time: f32) -> GameResult {
        // 🎯 新架构: InputSystem 只负责捕获输入，不处理游戏逻辑
        // 实际的输入事件捕获在 GGEZ 事件回调中完成 (mouse_button_down_event 等)
        // 这些回调会调用 InputSystem::process_xxx 方法将事件写入 GlobalEvents
        
        // 此 update 方法保留用于未来可能的输入状态轮询
        // 当前架构下，所有输入都通过事件驱动
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_system_creation() {
        let system = InputSystem::new();
        assert_eq!(system.priority(), crate::ecs::systems::priority::INPUT);
    }
}