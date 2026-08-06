// ============================================================================
// EventBus - 游戏事件总线
// ============================================================================
//
// 职责：
// - 管理所有类型的游戏事件
// - 提供类型安全的事件发送/订阅接口
// - 自动管理事件生命周期（帧内有效）
//
// 设计原则：
// - 类型安全：每种事件独立存储，编译期检查
// - 零拷贝：通过迭代器访问，避免克隆
// - 帧内有效：每帧结束自动清空
// - 统计监控：记录事件数量，方便性能分析

mod input_event;
mod logic_event;
mod presentation_event;
mod ui_event;

// 重新导出所有事件类型
pub use input_event::*;
pub use logic_event::*;
pub use presentation_event::*;
pub use ui_event::*;

// NetworkEvent 直接使用 network 模块中的定义
pub use crate::network::handlers::NetworkEvent;

use std::collections::VecDeque;

// ============================================================================
// EventBus 核心结构
// ============================================================================

/// 游戏事件总线
///
/// **使用示例**:
/// ```rust
/// // 发送事件
/// ctx.events_mut().send_input(InputEvent::KeyDown { ... });
/// ctx.events_mut().send_logic(GameLogicEvent::DamageDealt { ... });
///
/// // 读取事件（零拷贝）
/// for event in ctx.events().input_events() {
///     match event {
///         InputEvent::KeyDown { keycode, .. } => { /* 处理 */ },
///         _ => {}
///     }
/// }
///
/// // 帧结束时清空
/// ctx.events_mut().clear_frame();
/// ```
pub struct EventBus {
    // 输入事件队列
    input_events: VecDeque<InputEvent>,

    // 网络事件队列
    network_events: VecDeque<NetworkEvent>,

    // 游戏逻辑事件队列
    logic_events: VecDeque<GameLogicEvent>,

    // UI事件队列
    ui_events: VecDeque<UIEvent>,

    // 表现层事件队列
    presentation_events: VecDeque<PresentationEvent>,

    // 统计信息
    stats: EventBusStats,
}

/// 事件总线统计信息
#[derive(Debug, Default, Clone)]
pub struct EventBusStats {
    /// 总共处理的事件数
    pub total_events_processed: u64,

    /// 当前帧事件数
    pub events_this_frame: usize,

    /// 历史最大队列大小
    pub peak_queue_size: usize,

    /// 各类型事件计数
    pub input_count: usize,
    pub network_count: usize,
    pub logic_count: usize,
    pub ui_count: usize,
    pub presentation_count: usize,
}

impl EventBus {
    /// 创建新的事件总线
    pub fn new() -> Self {
        Self {
            input_events: VecDeque::with_capacity(64),
            network_events: VecDeque::with_capacity(128),
            logic_events: VecDeque::with_capacity(256),
            ui_events: VecDeque::with_capacity(32),
            presentation_events: VecDeque::with_capacity(128),
            stats: EventBusStats::default(),
        }
    }

    // ========================================================================
    // 生产者 API - 发送事件
    // ========================================================================

    /// 发送输入事件
    ///
    /// **生产者**: InputSystem
    /// **消费者**: PlayerControlSystem, CameraSystem, UISystem
    #[inline]
    pub fn send_input(&mut self, event: InputEvent) {
        self.input_events.push_back(event);
        self.stats.input_count += 1;
        self.update_stats();
    }

    /// 批量发送输入事件
    #[inline]
    pub fn send_input_batch(&mut self, events: impl IntoIterator<Item = InputEvent>) {
        for event in events {
            self.send_input(event);
        }
    }

    /// 发送网络事件
    ///
    /// **生产者**: NetworkSystem
    /// **消费者**: 各种逻辑系统
    #[inline]
    pub fn send_network(&mut self, event: NetworkEvent) {
        self.network_events.push_back(event);
        self.stats.network_count += 1;
        self.update_stats();
    }

    /// 批量发送网络事件
    #[inline]
    pub fn send_network_batch(&mut self, events: impl IntoIterator<Item = NetworkEvent>) {
        for event in events {
            self.send_network(event);
        }
    }

    /// 发送游戏逻辑事件
    ///
    /// **生产者**: 各种逻辑系统
    /// **消费者**: 其他逻辑系统、表现层系统
    #[inline]
    pub fn send_logic(&mut self, event: GameLogicEvent) {
        self.logic_events.push_back(event);
        self.stats.logic_count += 1;
        self.update_stats();
    }

    /// 发送UI事件
    ///
    /// **生产者**: UI系统
    /// **消费者**: 逻辑系统、网络系统
    #[inline]
    pub fn send_ui(&mut self, event: UIEvent) {
        self.ui_events.push_back(event);
        self.stats.ui_count += 1;
        self.update_stats();
    }

    /// 发送表现层事件
    ///
    /// **生产者**: 逻辑系统
    /// **消费者**: 渲染系统、音效系统、粒子系统
    #[inline]
    pub fn send_presentation(&mut self, event: PresentationEvent) {
        self.presentation_events.push_back(event);
        self.stats.presentation_count += 1;
        self.update_stats();
    }

    // ========================================================================
    // 消费者 API - 读取事件（零拷贝）
    // ========================================================================

    /// 读取所有输入事件
    #[inline]
    pub fn input_events(&self) -> impl Iterator<Item = &InputEvent> {
        self.input_events.iter()
    }

    /// 读取所有网络事件
    #[inline]
    pub fn network_events(&self) -> impl Iterator<Item = &NetworkEvent> {
        self.network_events.iter()
    }

    /// 读取所有游戏逻辑事件
    #[inline]
    pub fn logic_events(&self) -> impl Iterator<Item = &GameLogicEvent> {
        self.logic_events.iter()
    }

    /// 读取所有UI事件
    #[inline]
    pub fn ui_events(&self) -> impl Iterator<Item = &UIEvent> {
        self.ui_events.iter()
    }

    /// 读取所有表现层事件
    #[inline]
    pub fn presentation_events(&self) -> impl Iterator<Item = &PresentationEvent> {
        self.presentation_events.iter()
    }

    // ========================================================================
    // 查询 API - 检查事件状态
    // ========================================================================

    /// 是否有输入事件
    #[inline]
    pub fn has_input_events(&self) -> bool {
        !self.input_events.is_empty()
    }

    /// 是否有网络事件
    #[inline]
    pub fn has_network_events(&self) -> bool {
        !self.network_events.is_empty()
    }

    /// 是否有游戏逻辑事件
    #[inline]
    pub fn has_logic_events(&self) -> bool {
        !self.logic_events.is_empty()
    }

    /// 是否有UI事件
    #[inline]
    pub fn has_ui_events(&self) -> bool {
        !self.ui_events.is_empty()
    }

    /// 是否有表现层事件
    #[inline]
    pub fn has_presentation_events(&self) -> bool {
        !self.presentation_events.is_empty()
    }

    /// 获取当前所有事件总数
    #[inline]
    pub fn total_event_count(&self) -> usize {
        self.input_events.len()
            + self.network_events.len()
            + self.logic_events.len()
            + self.ui_events.len()
            + self.presentation_events.len()
    }

    // ========================================================================
    // 帧管理 API
    // ========================================================================

    /// 清空当前帧的所有事件
    ///
    /// **调用时机**: 每帧结束时由主循环调用
    ///
    /// **示例**:
    /// ```rust
    /// loop {
    ///     scheduler.update(&mut ctx, dt)?;
    ///     scheduler.draw(&mut ctx, dt)?;
    ///     ctx.events_mut().clear_frame(); // 👈 清空帧事件
    ///     next_frame().await;
    /// }
    /// ```
    pub fn clear_frame(&mut self) {
        self.input_events.clear();
        self.network_events.clear();
        self.logic_events.clear();
        self.ui_events.clear();
        self.presentation_events.clear();

        // 重置帧统计
        self.stats.events_this_frame = 0;
        self.stats.input_count = 0;
        self.stats.network_count = 0;
        self.stats.logic_count = 0;
        self.stats.ui_count = 0;
        self.stats.presentation_count = 0;
    }

    /// 获取统计信息
    #[inline]
    pub fn stats(&self) -> &EventBusStats {
        &self.stats
    }

    /// 获取可变统计信息
    #[inline]
    pub fn stats_mut(&mut self) -> &mut EventBusStats {
        &mut self.stats
    }

    // ========================================================================
    // 内部方法
    // ========================================================================

    #[inline]
    fn update_stats(&mut self) {
        self.stats.events_this_frame += 1;
        self.stats.total_events_processed += 1;

        let total = self.total_event_count();
        if total > self.stats.peak_queue_size {
            self.stats.peak_queue_size = total;
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Debug 实现
// ============================================================================

impl std::fmt::Debug for EventBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventBus")
            .field("input_events", &self.input_events.len())
            .field("network_events", &self.network_events.len())
            .field("logic_events", &self.logic_events.len())
            .field("ui_events", &self.ui_events.len())
            .field("presentation_events", &self.presentation_events.len())
            .field("stats", &self.stats)
            .finish()
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use macroquad::prelude::KeyCode;
    use std::time::Instant;

    #[test]
    fn test_event_bus_basic() {
        let mut bus = EventBus::new();

        // 初始状态
        assert_eq!(bus.total_event_count(), 0);
        assert!(!bus.has_input_events());

        // 发送事件
        bus.send_input(InputEvent::KeyDown {
            keycode: KeyCode::W,
            repeat: false,
            modifiers: KeyModifiers::none(),
            timestamp: Instant::now(),
        });

        assert_eq!(bus.total_event_count(), 1);
        assert!(bus.has_input_events());

        // 清空
        bus.clear_frame();
        assert_eq!(bus.total_event_count(), 0);
    }

    #[test]
    fn test_event_iteration() {
        let mut bus = EventBus::new();

        bus.send_input(InputEvent::KeyDown {
            keycode: KeyCode::W,
            repeat: false,
            modifiers: KeyModifiers::none(),
            timestamp: Instant::now(),
        });

        bus.send_input(InputEvent::KeyUp {
            keycode: KeyCode::W,
            modifiers: KeyModifiers::none(),
            timestamp: Instant::now(),
        });

        let count = bus.input_events().count();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_stats() {
        let mut bus = EventBus::new();

        bus.send_input(InputEvent::KeyDown {
            keycode: KeyCode::W,
            repeat: false,
            modifiers: KeyModifiers::none(),
            timestamp: Instant::now(),
        });

        assert_eq!(bus.stats().events_this_frame, 1);
        assert_eq!(bus.stats().total_events_processed, 1);
        assert_eq!(bus.stats().input_count, 1);

        bus.clear_frame();
        assert_eq!(bus.stats().events_this_frame, 0);
        assert_eq!(bus.stats().input_count, 0);
    }
}
