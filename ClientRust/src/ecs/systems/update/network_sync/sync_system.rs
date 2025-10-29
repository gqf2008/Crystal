// ============================================================================
// Layer 6: Network Sync - SyncSystem
// Priority: 610
// ============================================================================
//
// **职责**：
// - 状态同步验证
// - 网络对象生命周期管理
// - 断线重连处理
//
// **逻辑来源**：
// - C# MapObject管理: 对象创建、移除
// - NetworkSync: 服务器权威状态
//
// ============================================================================

use hecs::World;
use ggez::GameResult;
use crate::ecs::systems::{System, priority};
use crate::ecs::components::{NetworkSync, Lifetime};

/// 同步系统
/// 
/// 同步机制:
/// 1. 管理网络对象的生命周期
/// 2. 清理过期的临时对象(特效、掉落物)
/// 3. 验证状态一致性
pub struct SyncSystem;

impl System for SyncSystem {
    fn priority(&self) -> u32 {
        priority::SYNC
    }

    fn update(&mut self, world: &mut World, delay_time: f32) -> GameResult {
        let delta_ms = (delay_time * 1000.0) as u32;

        // 1. 更新和清理有生命周期的对象
        let mut to_remove = Vec::new();
        for (entity_id, lifetime) in world.query_mut::<&mut Lifetime>() {
            if lifetime.update(delta_ms) {
                // 生命周期结束,标记删除
                to_remove.push(entity_id);
            }
        }

        // 删除过期实体
        for entity_id in to_remove {
            let _ = world.despawn(entity_id);
        }

        // 2. 更新网络同步状态
        for (_id, network_sync) in world.query_mut::<&mut NetworkSync>() {
            network_sync.last_update = std::time::Instant::now();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::components::NetworkObjectType;

    #[test]
    fn test_lifetime_expiration() {
        let mut world = World::new();
        let mut system = SyncSystem;

        // 创建一个生命周期为100ms的对象
        world.spawn((
            Lifetime::new(100),
            NetworkSync::new(1, NetworkObjectType::Spell),
        ));

        // 第一次更新: 50ms
        system.update(&mut world, 0.05).unwrap();
        assert_eq!(world.len(), 1); // 对象还在 (100-50=50ms剩余)

        // 第二次更新: 60ms = 总共110ms
        system.update(&mut world, 0.06).unwrap();
        assert_eq!(world.len(), 0); // 超过100ms,被删除
    }

    #[test]
    fn test_sync_update() {
        let mut world = World::new();
        let mut system = SyncSystem;

        let entity = world.spawn((
            NetworkSync::new(123, NetworkObjectType::Player),
        ));

        let start_time = world.get::<&NetworkSync>(entity).unwrap().last_update;

        std::thread::sleep(std::time::Duration::from_millis(10));

        system.update(&mut world, 0.016).unwrap();

        let end_time = world.get::<&NetworkSync>(entity).unwrap().last_update;
        assert!(end_time > start_time); // 时间应该被更新
    }
}

