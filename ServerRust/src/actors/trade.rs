// Trade system - 交易数据结构
// 纯数据结构，由 WorldActor 调用

/// 交易阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradePhase {
    /// 正在协商（可添加物品/金币）
    Negotiating,
    /// 一方已确认锁定
    OneLocked,
    /// 双方确认，待执行
    BothLocked,
}

/// 交易中的物品条目
#[derive(Debug, Clone)]
pub struct TradeItem {
    /// 物品 unique_id
    pub uid: u64,
    /// 交易格子索引
    pub grid: u8,
    /// 堆叠数量（拆分的部分）
    pub count: u16,
}

/// 交易一方
#[derive(Debug, Clone)]
pub struct TradeSide {
    /// Session ID
    pub session_id: u64,
    /// 玩家名称
    pub name: String,
    /// 放入的物品
    pub items: Vec<TradeItem>,
    /// 放入的金币
    pub gold: u64,
    /// 是否已确认锁定
    pub locked: bool,
}

impl TradeSide {
    pub fn new(session_id: u64, name: String) -> Self {
        Self {
            session_id,
            name,
            items: Vec::new(),
            gold: 0,
            locked: false,
        }
    }

    /// 添加物品
    pub fn add_item(&mut self, uid: u64, grid: u8, count: u16) {
        if let Some(idx) = self.items.iter().position(|i| i.uid == uid) {
            self.items[idx].count = count;
        } else {
            self.items.push(TradeItem { uid, grid, count });
        }
    }

    /// 移除物品
    pub fn remove_item(&mut self, uid: u64) -> Option<TradeItem> {
        if let Some(idx) = self.items.iter().position(|i| i.uid == uid) {
            Some(self.items.remove(idx))
        } else {
            None
        }
    }

    /// 重置确认状态
    pub fn unlock(&mut self) {
        self.locked = false;
    }
}

/// 交易会话
#[derive(Debug, Clone)]
pub struct TradeSession {
    /// A 方
    pub side_a: TradeSide,
    /// B 方
    pub side_b: TradeSide,
    /// 当前阶段
    pub phase: TradePhase,
}

impl TradeSession {
    /// 创建新交易会话
    pub fn new(initiator_session: u64, initiator_name: String, target_session: u64, target_name: String) -> Self {
        Self {
            side_a: TradeSide::new(initiator_session, initiator_name),
            side_b: TradeSide::new(target_session, target_name),
            phase: TradePhase::Negotiating,
        }
    }

    /// 获取 session 对应的交易方
    pub fn side_of(&self, session_id: u64) -> Option<&TradeSide> {
        if self.side_a.session_id == session_id {
            Some(&self.side_a)
        } else if self.side_b.session_id == session_id {
            Some(&self.side_b)
        } else {
            None
        }
    }

    /// 获取 session 对应的交易方（可变引用）
    pub fn side_of_mut(&mut self, session_id: u64) -> Option<&mut TradeSide> {
        if self.side_a.session_id == session_id {
            Some(&mut self.side_a)
        } else if self.side_b.session_id == session_id {
            Some(&mut self.side_b)
        } else {
            None
        }
    }

    /// 获取对方的 session ID
    pub fn other_session(&self, session_id: u64) -> Option<u64> {
        if self.side_a.session_id == session_id {
            Some(self.side_b.session_id)
        } else if self.side_b.session_id == session_id {
            Some(self.side_a.session_id)
        } else {
            None
        }
    }

    /// 获取对方交易方
    pub fn other_side(&self, session_id: u64) -> Option<&TradeSide> {
        if self.side_a.session_id == session_id {
            Some(&self.side_b)
        } else if self.side_b.session_id == session_id {
            Some(&self.side_a)
        } else {
            None
        }
    }

    /// 双方都锁定时返回 true
    pub fn both_locked(&self) -> bool {
        self.side_a.locked && self.side_b.locked
    }

    /// 获取两个参与者的 session IDs
    pub fn participant_sessions(&self) -> (u64, u64) {
        (self.side_a.session_id, self.side_b.session_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_session() -> TradeSession {
        TradeSession::new(1, "Alice".into(), 2, "Bob".into())
    }

    #[test]
    fn test_create_trade() {
        let session = make_session();
        assert_eq!(session.side_a.name, "Alice");
        assert_eq!(session.side_b.name, "Bob");
        assert!(session.side_a.items.is_empty());
        assert_eq!(session.side_a.gold, 0);
        assert!(!session.side_a.locked);
    }

    #[test]
    fn test_add_items_and_gold() {
        let mut session = make_session();
        let side_a = session.side_of_mut(1).unwrap();
        side_a.add_item(100, 0, 5);
        side_a.add_item(200, 1, 1);
        side_a.gold = 500;

        assert_eq!(side_a.items.len(), 2);
        assert_eq!(side_a.gold, 500);
    }

    #[test]
    fn test_lock_mechanism() {
        let mut session = make_session();
        let side_a = session.side_of_mut(1).unwrap();
        side_a.locked = true;
        assert!(session.side_of(1).unwrap().locked);
        assert!(!session.both_locked());

        let side_b = session.side_of_mut(2).unwrap();
        side_b.locked = true;
        assert!(session.both_locked());
    }

    #[test]
    fn test_other_session() {
        let session = make_session();
        assert_eq!(session.other_session(1), Some(2));
        assert_eq!(session.other_session(2), Some(1));
        assert_eq!(session.other_session(99), None);
    }

    #[test]
    fn test_item_replacement() {
        let mut session = make_session();
        let side_a = session.side_of_mut(1).unwrap();
        side_a.add_item(100, 0, 5);
        side_a.add_item(100, 0, 10); // same uid, should update count
        assert_eq!(side_a.items.len(), 1);
        assert_eq!(side_a.items[0].count, 10);
    }
}
