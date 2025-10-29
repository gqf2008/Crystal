// ============================================================================
// 网络同步组件
// ============================================================================

use std::time::Instant;
use std::collections::VecDeque;

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

/// 网络发送队列组件
#[derive(Debug, Clone)]
pub struct NetworkQueue {
    /// 待发送的消息队列
    pub pending_messages: VecDeque<Vec<u8>>,
    /// 最大队列长度
    pub max_queue_size: usize,
}

impl NetworkQueue {
    pub fn new() -> Self {
        Self {
            pending_messages: VecDeque::new(),
            max_queue_size: 100,
        }
    }

    /// 入队消息
    pub fn enqueue_message(&mut self, message: Vec<u8>) {
        if self.pending_messages.len() < self.max_queue_size {
            self.pending_messages.push_back(message);
        }
    }

    /// 处理发送队列
    pub fn process_send_queue(&mut self) {
        // 实际发送逻辑应该由网络管理器处理
        // 这里只是清空队列作为示例
        // 在实际使用中,应该将消息传递给网络层
        self.pending_messages.clear();
    }

    /// 获取队列大小
    pub fn queue_size(&self) -> usize {
        self.pending_messages.len()
    }
}

impl Default for NetworkQueue {
    fn default() -> Self {
        Self::new()
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
