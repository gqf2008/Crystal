// ============================================================================
// InputEvent - 输入事件定义
// ============================================================================
//
// 职责：
// - 定义所有输入相关的事件（键盘、鼠标、触摸、窗口）
// - 由 InputSystem 产生，被 PlayerControlSystem/CameraSystem/UISystem 消费
//
// 设计原则：
// - 细粒度：每种输入行为独立事件（方便精确处理）
// - 带时间戳：用于双击、长按等检测
// - 跨平台：支持PC（键鼠）和移动端（触摸）

use macroquad::prelude::{KeyCode, MouseButton};
use std::time::Instant;

// ============================================================================
// 输入事件枚举
// ============================================================================

#[derive(Debug, Clone)]
pub enum InputEvent {
    // ========================================================================
    // 键盘事件
    // ========================================================================
    
    /// 键盘按下（包含重复按键）
    /// 
    /// **用途**: 游戏控制（WASD移动、技能快捷键）
    /// **注意**: repeat=true 表示长按自动重复
    KeyDown {
        keycode: KeyCode,
        repeat: bool,
        modifiers: KeyModifiers,
        timestamp: Instant,
    },
    
    /// 键盘释放
    KeyUp {
        keycode: KeyCode,
        modifiers: KeyModifiers,
        timestamp: Instant,
    },
    
    /// 字符输入（IME/输入法）
    /// 
    /// **用途**: 文本输入框（聊天、搜索、角色名）
    /// **区别**: 与 KeyDown 分离，专门处理文本输入
    CharInput {
        character: char,
        timestamp: Instant,
    },
    
    // ========================================================================
    // 鼠标事件
    // ========================================================================
    
    /// 鼠标移动
    MouseMove {
        x: f32,
        y: f32,
        dx: f32,
        dy: f32,
        timestamp: Instant,
    },
    
    /// 鼠标按钮按下
    MouseDown {
        button: MouseButton,
        x: f32,
        y: f32,
        timestamp: Instant,
    },
    
    /// 鼠标按钮释放
    MouseUp {
        button: MouseButton,
        x: f32,
        y: f32,
        timestamp: Instant,
    },
    
    /// 鼠标滚轮
    MouseWheel {
        delta_x: f32,
        delta_y: f32,
        timestamp: Instant,
    },
    
    /// 鼠标进入/离开窗口
    MouseEnterOrLeave {
        entered: bool,
        timestamp: Instant,
    },
    
    // ========================================================================
    // 触摸事件（移动端/平板）
    // ========================================================================
    
    /// 触摸开始
    TouchStart {
        id: u64,           // 触摸点 ID（支持多点触控）
        x: f32,
        y: f32,
        timestamp: Instant,
    },
    
    /// 触摸移动
    TouchMove {
        id: u64,
        x: f32,
        y: f32,
        dx: f32,
        dy: f32,
        timestamp: Instant,
    },
    
    /// 触摸结束
    TouchEnd {
        id: u64,
        x: f32,
        y: f32,
        timestamp: Instant,
    },
    
    /// 触摸取消（被系统中断，如来电）
    TouchCancel {
        id: u64,
        timestamp: Instant,
    },
    
    // ========================================================================
    // 窗口事件
    // ========================================================================
    
    /// 窗口大小改变
    Resize {
        width: f32,
        height: f32,
        timestamp: Instant,
    },
    
    /// 窗口获得/失去焦点
    Focus {
        focused: bool,
        timestamp: Instant,
    },
}

// ============================================================================
// 辅助结构
// ============================================================================

/// 键盘修饰键状态
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeyModifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub logo: bool,  // Windows键/Command键
}

impl KeyModifiers {
    /// 创建空修饰键
    pub fn none() -> Self {
        Self::default()
    }
    
    /// 创建只有 Ctrl 的修饰键
    pub fn ctrl() -> Self {
        Self {
            ctrl: true,
            ..Default::default()
        }
    }
    
    /// 创建只有 Shift 的修饰键
    pub fn shift() -> Self {
        Self {
            shift: true,
            ..Default::default()
        }
    }
    
    /// 创建只有 Alt 的修饰键
    pub fn alt() -> Self {
        Self {
            alt: true,
            ..Default::default()
        }
    }
    
    /// 是否没有任何修饰键按下
    pub fn is_empty(&self) -> bool {
        !self.ctrl && !self.shift && !self.alt && !self.logo
    }
    
    /// 是否有任何修饰键按下
    pub fn any(&self) -> bool {
        !self.is_empty()
    }
}

// ============================================================================
// InputEvent 辅助方法
// ============================================================================

impl InputEvent {
    /// 获取事件时间戳
    pub fn timestamp(&self) -> Instant {
        match self {
            InputEvent::KeyDown { timestamp, .. } => *timestamp,
            InputEvent::KeyUp { timestamp, .. } => *timestamp,
            InputEvent::CharInput { timestamp, .. } => *timestamp,
            InputEvent::MouseMove { timestamp, .. } => *timestamp,
            InputEvent::MouseDown { timestamp, .. } => *timestamp,
            InputEvent::MouseUp { timestamp, .. } => *timestamp,
            InputEvent::MouseWheel { timestamp, .. } => *timestamp,
            InputEvent::MouseEnterOrLeave { timestamp, .. } => *timestamp,
            InputEvent::TouchStart { timestamp, .. } => *timestamp,
            InputEvent::TouchMove { timestamp, .. } => *timestamp,
            InputEvent::TouchEnd { timestamp, .. } => *timestamp,
            InputEvent::TouchCancel { timestamp, .. } => *timestamp,
            InputEvent::Resize { timestamp, .. } => *timestamp,
            InputEvent::Focus { timestamp, .. } => *timestamp,
        }
    }
    
    /// 是否是键盘事件
    pub fn is_keyboard(&self) -> bool {
        matches!(
            self,
            InputEvent::KeyDown { .. } | InputEvent::KeyUp { .. } | InputEvent::CharInput { .. }
        )
    }
    
    /// 是否是鼠标事件
    pub fn is_mouse(&self) -> bool {
        matches!(
            self,
            InputEvent::MouseMove { .. }
                | InputEvent::MouseDown { .. }
                | InputEvent::MouseUp { .. }
                | InputEvent::MouseWheel { .. }
                | InputEvent::MouseEnterOrLeave { .. }
        )
    }
    
    /// 是否是触摸事件
    pub fn is_touch(&self) -> bool {
        matches!(
            self,
            InputEvent::TouchStart { .. }
                | InputEvent::TouchMove { .. }
                | InputEvent::TouchEnd { .. }
                | InputEvent::TouchCancel { .. }
        )
    }
    
    /// 是否是窗口事件
    pub fn is_window(&self) -> bool {
        matches!(
            self,
            InputEvent::Resize { .. } | InputEvent::Focus { .. }
        )
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_key_modifiers() {
        let none = KeyModifiers::none();
        assert!(none.is_empty());
        assert!(!none.any());
        
        let ctrl = KeyModifiers::ctrl();
        assert!(!ctrl.is_empty());
        assert!(ctrl.any());
        assert!(ctrl.ctrl);
        assert!(!ctrl.shift);
    }
    
    #[test]
    fn test_event_classification() {
        let key_event = InputEvent::KeyDown {
            keycode: KeyCode::W,
            repeat: false,
            modifiers: KeyModifiers::none(),
            timestamp: Instant::now(),
        };
        
        assert!(key_event.is_keyboard());
        assert!(!key_event.is_mouse());
        assert!(!key_event.is_touch());
        assert!(!key_event.is_window());
    }
}
