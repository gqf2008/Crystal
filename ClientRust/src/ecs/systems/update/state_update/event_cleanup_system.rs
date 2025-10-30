//! 事件清理系统
//! 
//! 在每帧结束时清理全局事件，防止事件重放

use hecs::World;
use ggez::GameResult;
use crate::ecs::components::GlobalEvents;
use crate::ecs::systems::System;

/// 事件清理系统
/// 
/// 这个系统应该在所有其他系统之后执行（最低优先级）
pub struct EventCleanupSystem;

impl System for EventCleanupSystem {
    fn name(&self) -> &'static str {
        "EventCleanupSystem"
    }
    
    fn priority(&self) -> u32 {
        9999 // 最低优先级,最后执行
    }
    
    fn update(&mut self, world: &mut World, _delay_time: f32) -> GameResult {
        Self::cleanup(world)
    }
}

impl EventCleanupSystem {
    /// 清理当前帧的所有事件
    /// 
    /// 应该在帧末尾调用（所有其他系统执行完毕后）
    pub fn cleanup(world: &mut World) -> GameResult {
        // 查询 GlobalEvents 组件
        for (_, events) in world.query_mut::<&mut GlobalEvents>() {
            // 清理事件
            events.clear_frame_events();
        }
        
        Ok(())
    }
    
    /// 打印事件统计信息（调试用）
    pub fn print_stats(world: &World) {
        for (_, events) in world.query::<&GlobalEvents>().iter() {
            let stats = events.get_frame_stats();
            if stats.total_count > 0 {
                println!("📊 事件统计: 键盘={} 鼠标={} IME={} 游戏={} 总计={}",
                         stats.keyboard_count,
                         stats.mouse_count,
                         stats.ime_count,
                         stats.game_count,
                         stats.total_count);
            }
        }
    }
}

/// 事件收集系统
/// 
/// 从 GGEZ 的事件处理器收集事件到 GlobalEvents 组件
pub struct EventCollectorSystem;

impl EventCollectorSystem {
    /// 确保 GlobalEvents 组件存在
    /// 
    /// 如果不存在则创建一个
    pub fn ensure_global_events(world: &mut World) {
        let has_events = {
            let mut query = world.query::<&GlobalEvents>();
            query.iter().next().is_some()
        };
        
        if !has_events {
            world.spawn((GlobalEvents::new(),));
            println!("✅ 创建全局事件组件");
        }
    }
    
    /// 使用闭包获取 GlobalEvents 的可变引用
    /// 
    /// 这是一个便捷方法，供其他系统使用
    /// 
    /// # 示例
    /// ```ignore
    /// EventCollectorSystem::with_global_events_mut(&mut world, |events| {
    ///     events.push_keyboard(KeyCode::KeyW, true, false);
    /// });
    /// ```
    pub fn with_global_events_mut<F, R>(world: &mut World, f: F) -> Option<R>
    where
        F: FnOnce(&mut GlobalEvents) -> R,
    {
        for (_, events) in world.query::<&mut GlobalEvents>().iter() {
            return Some(f(events));
        }
        None
    }
    
    /// 使用闭包获取 GlobalEvents 的不可变引用
    /// 
    /// # 示例
    /// ```ignore
    /// EventCollectorSystem::with_global_events(&world, |events| {
    ///     for key in events.filter_key_pressed() {
    ///         println!("按键: {:?}", key.keycode);
    ///     }
    /// });
    /// ```
    pub fn with_global_events<F, R>(world: &World, f: F) -> Option<R>
    where
        F: FnOnce(&GlobalEvents) -> R,
    {
        for (_, events) in world.query::<&GlobalEvents>().iter() {
            return Some(f(events));
        }
        None
    }
}
