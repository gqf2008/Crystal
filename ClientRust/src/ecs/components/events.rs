

use ggez::input::keyboard::KeyCode;
use ggez::winit::event::MouseButton;
use ggez::winit::keyboard::SmolStr;

// ============================================================================
// 事件类型定义
// ============================================================================

#[derive(Debug, Clone)]
pub enum InputEvent {
    // KeyDown {
    //     keycode: KeyCode,
    //     repeat: bool, // 是否是重复按键
    //     text: Option<SmolStr>,
    //     timestamp: std::time::Instant,
    // },
    // KeyUp {
    //     keycode: KeyCode,
    //     text: Option<SmolStr>,
    //     timestamp: std::time::Instant,
    // },
    MouseMove {
        x: f32,
        y: f32,
        dx: f32,
        dy: f32,
    },
    // /// 鼠标按钮按下
    // MouseDown {
    //     button: MouseButton,
    //     x: f32,
    //     y: f32,
    // },
    // /// 鼠标按钮释放
    // MouseUp {
    //     button: MouseButton,
    //     x: f32,
    //     y: f32,
    // },
    /// 鼠标滚轮
    MouseWheel {
        x: f32,
        y: f32,
    },
    /// 鼠标进入/离开窗口
    MouseEnterOrLeave {
        entered: bool,
    },
    Ime {
        character: char,
        timestamp: std::time::Instant,
    },
    Resize {
        width: f32,
        height: f32,
    },
}

pub use crate::network::handlers::GameEvent;
