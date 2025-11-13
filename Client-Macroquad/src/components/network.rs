// ============================================================================
// 网络同步组件
// ============================================================================

use std::time::Instant;

/// 网络同步标记 (需要同步的实体)
#[derive(Debug, Clone)]
pub struct NetworkSync {
    /// 服务器对象ID
    pub object_id: u32,
    /// 最后更新时间
    pub last_update: Instant,
    /// 对象类型
    pub object_type: NetworkObjectType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkObjectType {
    Player,      // 其他玩家
    NPC,         // NPC
    Monster,     // 怪物
    Item,        // 地面物品
    Spell,       // 技能特效
}

impl NetworkSync {
    pub fn new(object_id: u32, object_type: NetworkObjectType) -> Self {
        Self {
            object_id,
            last_update: Instant::now(),
            object_type,
        }
    }
}

/// 生命周期组件 (技能特效/掉落物等有时间限制的实体)
#[derive(Debug, Clone, Copy)]
pub struct Lifetime {
    pub remaining_ms: u32,
}

impl Lifetime {
    pub fn new(duration_ms: u32) -> Self {
        Self { remaining_ms: duration_ms }
    }

    pub fn update(&mut self, delta_ms: u32) -> bool {
        if self.remaining_ms > delta_ms {
            self.remaining_ms -= delta_ms;
            false
        } else {
            self.remaining_ms = 0;
            true // 生命周期结束
        }
    }
}

/// 网络上下文（全局网络状态）
#[derive(Debug, Clone)]
pub struct NetworkContext {
    pub connected: bool,
}

impl NetworkContext {
    pub fn new() -> Self {
        Self { connected: false }
    }
}