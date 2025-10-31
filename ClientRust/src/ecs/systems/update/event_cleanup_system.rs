// ============================================================================
// Event Cleanup System - 全局事件清理系统
// ============================================================================
//
// 职责：
// - 每帧结束时清理 GlobalEvents 中的所有事件
// - 防止事件在下一帧被重复处理（事件污染）
// - 重置帧事件计数器
//
// 执行时机：
// - 优先级 900（最低优先级，所有系统执行完后清理）
// - 每帧最后执行
//
// 设计说明：
// - 这是一个纯清理系统，不产生副作用
// - 不清理网络命令 channel（由网络线程消费）
// - 保留事件统计数据（total_event_count）
//
// ============================================================================

use hecs::World;
use ggez::GameResult;
use crate::ecs::systems::{System, priority};
use crate::ecs::components::GlobalEvents;

/// 事件清理系统
/// 
/// 每帧结束时清理所有临时事件，防止事件在下一帧被重复处理。
pub struct EventCleanupSystem;

impl EventCleanupSystem {
    pub fn new() -> Self {
        Self
    }
}

impl System for EventCleanupSystem {
    fn name(&self) -> &'static str {
        "EventCleanupSystem"
    }
    
    fn priority(&self) -> u32 {
        900  // 最低优先级，确保所有系统都处理完事件后再清理
    }
    
    fn update(&mut self, world: &mut hecs::World, _delay_time: f32) -> GameResult {
        // 查询 GlobalEvents 组件
        let mut query = world.query::<&mut GlobalEvents>();
        
        if let Some((_, events)) = query.iter().next() {
            let frame_count = events.frame_event_count;
            
            // 清理所有事件队列
            events.keyboard_events.clear();
            events.mouse_events.clear();
            events.ime_events.clear();
            events.game_events.clear();
            events.network_incoming.clear();
            
            // 重置帧计数器
            events.frame_event_count = 0;
            
            // 日志输出（可选）
            if events.enable_logging && frame_count > 0 {
                tracing::debug!("🧹 清理 {} 个事件", frame_count);
            }
        }
        
        Ok(())
    }
}
