//! 全局事件系统组件
//! 
//! 这是一个事件驱动的 ECS 架构核心组件。
//! 
//! 设计原则：
//! 1. 所有游戏事件统一管理（键盘、鼠标、IME、网络等）
//! 2. 使用 Vec 缓存事件，支持多系统并发读取
//! 3. 网络命令使用 Channel 立即发送到网络线程
//! 4. 网络数据包使用 Vec 缓存（由 NetworkSyncSystem 写入）
//! 5. 每帧结束自动清理，防止事件重放
//! 6. 提供便捷的事件过滤方法

use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Sender, Receiver, channel};
use ggez::input::keyboard::KeyCode;
use ggez::winit::event::MouseButton;
use crate::network::handlers::GameEvent as NetworkGameEvent;

// 类型别名,便于代码兼容
type NetworkCommand = NetworkGameEvent;

// ============================================================================
// 事件类型定义
// ============================================================================

/// 键盘事件
#[derive(Debug, Clone)]
pub struct KeyboardEvent {
    pub keycode: KeyCode,
    pub pressed: bool,          // true=按下, false=释放
    pub repeat: bool,           // 是否是重复按键
    pub timestamp: std::time::Instant,
}

/// 鼠标事件
#[derive(Debug, Clone)]
pub enum MouseEvent {
    /// 鼠标移动
    Move {
        x: f32,
        y: f32,
        dx: f32,
        dy: f32,
    },
    /// 鼠标按钮按下
    ButtonDown {
        button: MouseButton,
        x: f32,
        y: f32,
    },
    /// 鼠标按钮释放
    ButtonUp {
        button: MouseButton,
        x: f32,
        y: f32,
    },
    /// 鼠标滚轮
    Wheel {
        x: f32,
        y: f32,
    },
}

/// IME 字符输入事件
#[derive(Debug, Clone)]
pub struct ImeEvent {
    pub character: char,
    pub timestamp: std::time::Instant,
}

/// 游戏逻辑事件（网络同步等）
#[derive(Debug, Clone)]
pub enum GameEvent {
    /// 玩家移动请求
    PlayerMoveRequest {
        target_x: f32,
        target_y: f32,
        run: bool,
    },
    /// 攻击请求
    AttackRequest {
        target_id: u32,
        spell_id: Option<u32>,
    },
    /// 拾取物品
    PickupItem {
        item_id: u32,
    },
    /// 使用物品
    UseItem {
        slot: u32,
    },
    /// 与 NPC 对话
    TalkToNpc {
        npc_id: u32,
    },
    /// 地图切换
    MapChange {
        map_name: String,
    },
    /// 自定义游戏事件
    Custom {
        event_type: String,
        data: Vec<u8>,
    },
}

/// 网络数据包事件（服务器→客户端）
#[derive(Debug, Clone)]
pub struct NetworkPacket {
    pub packet_type: String,
    pub data: Vec<u8>,
}

// ============================================================================
// 全局事件组件
// ============================================================================

/// 全局事件组件
/// 
/// 这是一个单例组件，应该只在世界中创建一个实例。
/// 所有系统通过查询这个组件来获取事件。
pub struct GlobalEvents {
    // ====== 缓存型事件（每帧收集，帧末清理）======
    /// 键盘事件队列
    pub keyboard_events: Vec<KeyboardEvent>,
    
    /// 鼠标事件队列
    pub mouse_events: Vec<MouseEvent>,
    
    /// IME 字符输入队列
    pub ime_events: Vec<ImeEvent>,
    
    /// 游戏逻辑事件队列
    pub game_events: Vec<GameEvent>,
    
    /// 网络包队列（服务器→客户端，由 NetworkSyncSystem 写入）
    pub network_incoming: Vec<NetworkPacket>,
    
    // ====== 立即发送型事件（Channel）======
    /// 网络命令发送通道（客户端→服务器）
    /// 游戏系统直接通过此 channel 发送命令到网络线程
    network_command_sender: Sender<NetworkGameEvent>,
    
    /// 网络命令接收通道（网络线程持有）
    network_command_receiver: Arc<Mutex<Receiver<NetworkGameEvent>>>,
    
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
        let (command_sender, command_receiver) = channel();
        Self {
            keyboard_events: Vec::new(),
            mouse_events: Vec::new(),
            ime_events: Vec::new(),
            game_events: Vec::new(),
            network_incoming: Vec::new(),
            network_command_sender: command_sender,
            network_command_receiver: Arc::new(Mutex::new(command_receiver)),
            frame_event_count: 0,
            total_event_count: 0,
            enable_logging: false,
        }
    }
    
    /// 获取网络命令接收端（供网络线程使用）
    pub fn get_command_receiver(&self) -> Arc<Mutex<Receiver<NetworkGameEvent>>> {
        Arc::clone(&self.network_command_receiver)
    }
    
    // ========================================================================
    // 事件添加方法
    // ========================================================================
    
    /// 添加键盘事件
    pub fn push_keyboard(&mut self, keycode: KeyCode, pressed: bool, repeat: bool) {
        let event = KeyboardEvent {
            keycode,
            pressed,
            repeat,
            timestamp: std::time::Instant::now(),
        };
        
        if self.enable_logging {
            println!("🎹 键盘事件: {:?} {}", keycode, if pressed { "按下" } else { "释放" });
        }
        
        self.keyboard_events.push(event);
        self.frame_event_count += 1;
        self.total_event_count += 1;
    }
    
    /// 添加鼠标事件
    pub fn push_mouse(&mut self, event: MouseEvent) {
        if self.enable_logging {
            println!("🖱️  鼠标事件: {:?}", event);
        }
        
        self.mouse_events.push(event);
        self.frame_event_count += 1;
        self.total_event_count += 1;
    }
    
    /// 添加 IME 字符事件
    pub fn push_ime(&mut self, character: char) {
        let event = ImeEvent {
            character,
            timestamp: std::time::Instant::now(),
        };
        
        if self.enable_logging {
            println!("✏️  IME 输入: '{}'", character);
        }
        
        self.ime_events.push(event);
        self.frame_event_count += 1;
        self.total_event_count += 1;
    }
    
    /// 添加游戏事件
    pub fn push_game_event(&mut self, event: GameEvent) {
        if self.enable_logging {
            println!("🎮 游戏事件: {:?}", event);
        }
        
        self.game_events.push(event);
        self.frame_event_count += 1;
        self.total_event_count += 1;
    }
    
    /// 发送网络命令到网络线程（立即发送）
    /// 
    /// 游戏系统调用此方法将命令发送到网络线程处理
    /// 例如: MovementSystem → SendCommand(Walk) → NetworkThread
    pub fn send_network_command(&self, command: NetworkCommand) {
        if self.enable_logging {
            println!("📡 发送网络命令: {:?}", command);
        }
        
        // 忽略发送错误（接收端可能已关闭）
        let _ = self.network_command_sender.send(command);
    }
    
    /// 添加接收到的网络包（由 NetworkSyncSystem 调用）
    pub fn push_incoming_packet(&mut self, packet: NetworkPacket) {
        if self.enable_logging {
            println!("📥 接收网络包: {}", packet.packet_type);
        }
        
        self.network_incoming.push(packet);
        self.frame_event_count += 1;
        self.total_event_count += 1;
    }
    
    // ========================================================================
    // 事件过滤方法（为不同系统提供便捷访问）
    // ========================================================================
    
    /// 过滤键盘按下事件
    pub fn filter_key_pressed(&self) -> impl Iterator<Item = &KeyboardEvent> {
        self.keyboard_events.iter().filter(|e| e.pressed && !e.repeat)
    }
    
    /// 过滤键盘释放事件
    pub fn filter_key_released(&self) -> impl Iterator<Item = &KeyboardEvent> {
        self.keyboard_events.iter().filter(|e| !e.pressed)
    }
    
    /// 过滤特定按键
    pub fn filter_key(&self, keycode: KeyCode) -> impl Iterator<Item = &KeyboardEvent> {
        self.keyboard_events.iter().filter(move |e| e.keycode == keycode)
    }
    
    /// 过滤鼠标移动事件
    pub fn filter_mouse_move(&self) -> impl Iterator<Item = &MouseEvent> {
        self.mouse_events.iter().filter(|e| matches!(e, MouseEvent::Move { .. }))
    }
    
    /// 过滤鼠标按钮按下
    pub fn filter_mouse_button_down(&self, button: MouseButton) -> impl Iterator<Item = &MouseEvent> {
        self.mouse_events.iter().filter(move |e| {
            matches!(e, MouseEvent::ButtonDown { button: b, .. } if *b == button)
        })
    }
    
    /// 过滤鼠标滚轮
    pub fn filter_mouse_wheel(&self) -> impl Iterator<Item = &MouseEvent> {
        self.mouse_events.iter().filter(|e| matches!(e, MouseEvent::Wheel { .. }))
    }
    
    /// 过滤特定类型的游戏事件
    pub fn filter_game_events<F>(&self, predicate: F) -> impl Iterator<Item = &GameEvent>
    where
        F: Fn(&GameEvent) -> bool,
    {
        self.game_events.iter().filter(move |e| predicate(e))
    }
    
    /// 消费网络包队列（PacketProcessingSystem 使用）
    pub fn drain_incoming_packets(&mut self) -> impl Iterator<Item = NetworkPacket> + '_ {
        self.network_incoming.drain(..)
    }
    
    // ========================================================================
    // 帧管理方法
    // ========================================================================
    
    /// 清理当前帧的所有事件
    /// 
    /// 应该在每帧结束时调用，防止事件被重放
    pub fn clear_frame_events(&mut self) {
        self.keyboard_events.clear();
        self.mouse_events.clear();
        self.ime_events.clear();
        self.game_events.clear();
        self.network_incoming.clear();
        
        if self.enable_logging && self.frame_event_count > 0 {
            println!("🧹 清理事件: {} 个", self.frame_event_count);
        }
        
        self.frame_event_count = 0;
    }
    
    /// 获取当前帧事件统计
    pub fn get_frame_stats(&self) -> EventStats {
        EventStats {
            keyboard_count: self.keyboard_events.len(),
            mouse_count: self.mouse_events.len(),
            ime_count: self.ime_events.len(),
            game_count: self.game_events.len(),
            network_count: self.network_incoming.len(),
            total_count: self.frame_event_count,
        }
    }
    
    /// 检查是否有事件
    pub fn has_events(&self) -> bool {
        !self.keyboard_events.is_empty()
            || !self.mouse_events.is_empty()
            || !self.ime_events.is_empty()
            || !self.game_events.is_empty()
            || !self.network_incoming.is_empty()
    }
}

impl Default for GlobalEvents {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 辅助结构
// ============================================================================

/// 事件统计信息
#[derive(Debug, Clone, Copy)]
pub struct EventStats {
    pub keyboard_count: usize,
    pub mouse_count: usize,
    pub ime_count: usize,
    pub game_count: usize,
    pub network_count: usize,
    pub total_count: usize,
}
