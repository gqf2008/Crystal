

use ggez::input::keyboard::KeyCode;
use ggez::winit::event::MouseButton;
use ggez::winit::keyboard::SmolStr;

// ============================================================================
// 事件类型定义
// ============================================================================

#[derive(Debug, Clone)]
pub enum InputEvent {
    KeyDown {
        keycode: KeyCode,
        repeat: bool, // 是否是重复按键
        text: Option<SmolStr>,
        timestamp: std::time::Instant,
    },
    KeyUp {
        keycode: KeyCode,
        text: Option<SmolStr>,
        timestamp: std::time::Instant,
    },
    MouseMove {
        x: f32,
        y: f32,
        dx: f32,
        dy: f32,
    },
    /// 鼠标按钮按下
    MouseDown {
        button: MouseButton,
        x: f32,
        y: f32,
    },
    /// 鼠标按钮释放
    MouseUp {
        button: MouseButton,
        x: f32,
        y: f32,
    },
    /// 鼠标滚轮
    MouseWheel {
        x: f32,
        y: f32,
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
// ============================================================================
// 游戏事件 - 引用网络模块定义
// ============================================================================

use crate::network::builder::CategorizedEvents;
/// 游戏事件 (统一类型)
///
/// **重要**: 使用 network::handlers::GameEvent,不重复定义!
///
/// 用于游戏场景的所有事件通信:
/// - 系统间通信 (PlayerControlSystem → MovementSystem)
/// - 网络通信 (服务器 ↔ 客户端)
/// - 游戏逻辑事件 (开始游戏、地图切换等)
///
/// **架构**:
/// - network 模块定义 GameEvent 枚举及所有变体
/// - GlobalEvents 组件引用此类型
/// - NetworkSyncSystem 负责协议转换 (GameEvent ↔ ServerPacket)
pub use crate::network::handlers::GameEvent;

// ============================================================================
// 全局事件组件
// ============================================================================

/// 全局事件组件
///
/// **单例组件**: 应该只在世界中创建一个实例。
/// 所有系统通过查询这个组件来获取事件。
///
/// ## 事件类型
///
/// ### Vec 缓存型 (每帧清理)
/// - `keyboard_events` - 键盘输入 (InputSystem 写入)
/// - `mouse_events` - 鼠标输入 (InputSystem 写入)
/// - `ime_events` - IME 输入 (InputSystem 写入)
/// - `game_events` - 游戏逻辑事件 (各系统写入/读取)
/// - `network_incoming` - 服务器→客户端 (NetworkSyncSystem 写入)
///
/// ### Channel 立即发送型
/// - `network_outgoing` - 客户端→服务器 (各系统调用 send_network_command)
///
/// ## 使用示例
///
/// ```rust
/// // 读取鼠标事件
/// if let Some((_, events)) = world.query::<&GlobalEvents>().iter().next() {
///     for event in &events.mouse_events {
///         // 处理鼠标事件
///     }
/// }
///
/// // 发送网络命令 (立即发送)
/// if let Some((_, events)) = world.query::<&GlobalEvents>().iter().next() {
///     events.send_network_command(GameEvent::PlayerMoveRequest {
///         target_x: 100.0,
///         target_y: 200.0,
///         run: true,
///     });
/// }
/// ```
pub struct GlobalEvents {
    pub input_events: Vec<InputEvent>,

    pub net_events: CategorizedEvents,

    // ====== 事件统计 ======
    /// 当前帧事件计数
    pub frame_event_count: usize,

    /// 总事件计数
    pub total_event_count: u64,

    /// 是否启用事件日志
    pub enable_logging: bool,
}

impl GlobalEvents {
    /// 创建新的全局事件组件
    pub fn new() -> Self {
        Self {
            input_events: Vec::new(),
            net_events: CategorizedEvents::default(),
            frame_event_count: 0,
            total_event_count: 0,
            enable_logging: false,
        }
    }

    // // ========================================================================
    // // 事件添加方法
    // // ========================================================================

    // /// 添加键盘事件
    // pub fn push_keyboard(
    //     &mut self,
    //     keycode: KeyCode,
    //     pressed: bool,
    //     repeat: bool,
    //     text: Option<SmolStr>,
    // ) {
    //     let event = KeyboardEvent {
    //         keycode,
    //         pressed,
    //         repeat,
    //         text,
    //         timestamp: std::time::Instant::now(),
    //     };

    //     if self.enable_logging {
    //         println!(
    //             "🎹 键盘事件: {:?} {}",
    //             keycode,
    //             if pressed { "按下" } else { "释放" }
    //         );
    //     }

    //     self.keyboard_events.push(event);
    //     self.frame_event_count += 1;
    //     self.total_event_count += 1;
    // }

    // /// 添加鼠标事件
    // pub fn push_mouse(&mut self, event: MouseEvent) {
    //     if self.enable_logging {
    //         println!("🖱️  鼠标事件: {:?}", event);
    //     }

    //     self.mouse_events.push(event);
    //     self.frame_event_count += 1;
    //     self.total_event_count += 1;
    // }

    // /// 添加 IME 字符事件
    // pub fn push_ime(&mut self, character: char) {
    //     let event = ImeEvent {
    //         character,
    //         timestamp: std::time::Instant::now(),
    //     };

    //     if self.enable_logging {
    //         println!("✏️  IME 输入: '{}'", character);
    //     }

    //     self.ime_events.push(event);
    //     self.frame_event_count += 1;
    //     self.total_event_count += 1;
    // }

    // // ========================================================================
    // // 事件过滤方法（为不同系统提供便捷访问）
    // // ========================================================================

    // /// 过滤键盘按下事件
    // pub fn filter_key_pressed(&self) -> impl Iterator<Item = &KeyboardEvent> {
    //     self.keyboard_events
    //         .iter()
    //         .filter(|e| e.pressed && !e.repeat)
    // }

    // /// 过滤键盘释放事件
    // pub fn filter_key_released(&self) -> impl Iterator<Item = &KeyboardEvent> {
    //     self.keyboard_events.iter().filter(|e| !e.pressed)
    // }

    // /// 过滤特定按键
    // pub fn filter_key(&self, keycode: KeyCode) -> impl Iterator<Item = &KeyboardEvent> {
    //     self.keyboard_events
    //         .iter()
    //         .filter(move |e| e.keycode == keycode)
    // }

    // /// 过滤鼠标移动事件
    // pub fn filter_mouse_move(&self) -> impl Iterator<Item = &MouseEvent> {
    //     self.mouse_events
    //         .iter()
    //         .filter(|e| matches!(e, MouseEvent::Move { .. }))
    // }

    // /// 过滤鼠标按钮按下
    // pub fn filter_mouse_button_down(
    //     &self,
    //     button: MouseButton,
    // ) -> impl Iterator<Item = &MouseEvent> {
    //     self.mouse_events
    //         .iter()
    //         .filter(move |e| matches!(e, MouseEvent::ButtonDown { button: b, .. } if *b == button))
    // }

    // /// 过滤鼠标滚轮
    // pub fn filter_mouse_wheel(&self) -> impl Iterator<Item = &MouseEvent> {
    //     self.mouse_events
    //         .iter()
    //         .filter(|e| matches!(e, MouseEvent::Wheel { .. }))
    // }

    // ========================================================================
    // 帧管理方法
    // ========================================================================

    /// 清理当前帧的所有事件
    ///
    /// 应该在每帧结束时调用，防止事件被重放
    pub fn clear_frame_events(&mut self) {
        if self.has_events() {
            self.input_events.clear();
            self.net_events.clear();
        }

        if self.enable_logging && self.frame_event_count > 0 {
            tracing::debug!("🧹 清理事件: {} 个", self.frame_event_count);
        }
        self.frame_event_count = 0;
    }

    /// 获取当前帧事件统计
    pub fn get_frame_stats(&self) -> EventStats {
        EventStats {
            input_count: self.input_events.len(),
            net_count: self.net_events.total_count(),
            total_count: self.frame_event_count,
        }
    }

    /// 检查是否有事件
    pub fn has_events(&self) -> bool {
        !self.input_events.is_empty() || !self.net_events.is_empty()
    }
}

// ============================================================================
// 辅助结构
// ============================================================================

/// 事件统计信息
#[derive(Debug, Clone, Copy)]
pub struct EventStats {
    pub input_count: usize,
    pub net_count: usize,
    pub total_count: usize,
}
