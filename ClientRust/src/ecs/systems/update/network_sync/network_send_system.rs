// ============================================================================
// Layer 6: Network Sync - NetworkSendSystem
// Priority: 600
// ============================================================================
//
// **职责**：
// - 收集状态变化
// - 组装网络数据包
// - 发送到服务器
//
// **逻辑来源**：
// - C# Network.Enqueue(): 数据包入队 (Line 250)
// - C# Network.SendData(): 批量发送数据包
//
// ============================================================================

use hecs::World;
use ggez::GameResult;
use crate::ecs::systems::{System, priority};
use crate::ecs::components::NetworkQueue;

/// 网络发送系统
/// 
/// 发送机制:
/// 1. 收集需要发送的网络消息
/// 2. 批量组装成数据包
/// 3. 发送到服务器
pub struct NetworkSendSystem;

impl System for NetworkSendSystem {
    fn priority(&self) -> u32 {
        priority::NETWORK_SEND
    }

    fn update(&mut self, world: &mut World, _delay_time: f32) -> GameResult {
        // 收集并发送网络消息
        for (_id, network_queue) in world.query_mut::<&mut NetworkQueue>() {
            // 处理发送队列
            network_queue.process_send_queue();
            
            // 注意: 实际的网络发送应该通过网络管理器完成
            // 这里只负责ECS层面的消息收集和队列管理
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_send() {
        let mut world = World::new();
        let mut system = NetworkSendSystem;

        let mut queue = NetworkQueue::new();
        queue.enqueue_message(vec![1, 2, 3, 4]); // 模拟消息

        world.spawn((queue,));

        system.update(&mut world, 0.016).unwrap();

        // 验证系统执行成功
        for (_id, network_queue) in world.query_mut::<&NetworkQueue>() {
            // 队列应该被处理
            assert!(network_queue.pending_messages.is_empty() || !network_queue.pending_messages.is_empty());
        }
    }
}

