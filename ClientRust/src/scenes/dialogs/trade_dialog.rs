// Trade Dialog - 玩家交易对话框
// 用于玩家之间的物品和金币交易

use super::Dialog;
use crate::network::protocol::UserItem;

/// 交易状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeState {
    Pending,    // 待确认
    Trading,    // 交易中
    Locked,     // 已锁定
    Completed,  // 完成
    Cancelled,  // 取消
}

/// 交易方信息
#[derive(Debug, Clone)]
pub struct TradeParty {
    pub name: String,
    pub items: Vec<Option<UserItem>>, // 10个交易槽位 (5x2)
    pub gold: u32,
    pub locked: bool, // 是否已锁定
}

impl TradeParty {
    /// 创建新的交易方
    pub fn new(name: String) -> Self {
        Self {
            name,
            items: vec![None; 10],
            gold: 0,
            locked: false,
        }
    }

    /// 重置交易数据
    pub fn reset(&mut self) {
        self.items.iter_mut().for_each(|slot| *slot = None);
        self.gold = 0;
        self.locked = false;
    }

    /// 添加物品到交易槽
    pub fn add_item(&mut self, slot: usize, item: UserItem) -> bool {
        if slot < self.items.len() {
            self.items[slot] = Some(item);
            return true;
        }
        false
    }

    /// 移除交易槽中的物品
    pub fn remove_item(&mut self, slot: usize) -> Option<UserItem> {
        if slot < self.items.len() {
            self.items[slot].take()
        } else {
            None
        }
    }

    /// 设置金币数量
    pub fn set_gold(&mut self, amount: u32) {
        self.gold = amount;
    }

    /// 锁定/解锁交易
    pub fn set_locked(&mut self, locked: bool) {
        self.locked = locked;
    }

    /// 查找空槽位
    pub fn find_empty_slot(&self) -> Option<usize> {
        self.items.iter().position(|slot| slot.is_none())
    }

    /// 检查是否已满
    pub fn is_full(&self) -> bool {
        self.find_empty_slot().is_none()
    }

    /// 统计物品数量
    pub fn item_count(&self) -> usize {
        self.items.iter().filter(|slot| slot.is_some()).count()
    }
}

/// 交易对话框
pub struct TradeDialog {
    visible: bool,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    state: TradeState,

    // 交易双方
    pub host: TradeParty,  // 自己
    pub guest: TradeParty, // 对方

    // 选中的槽位
    pub selected_slot: Option<usize>,
}

impl TradeDialog {
    /// 创建新的交易对话框
    pub fn new() -> Self {
        Self {
            visible: false,
            x: 200,
            y: 150,
            width: 600,
            height: 400,
            state: TradeState::Pending,
            host: TradeParty::new(String::new()),
            guest: TradeParty::new(String::new()),
            selected_slot: None,
        }
    }

    /// 开始交易
    pub fn start_trade(&mut self, host_name: String, guest_name: String) {
        self.host = TradeParty::new(host_name);
        self.guest = TradeParty::new(guest_name);
        self.state = TradeState::Trading;
        self.selected_slot = None;
        self.visible = true;
    }

    /// 结束交易
    pub fn end_trade(&mut self) {
        self.host.reset();
        self.guest.reset();
        self.state = TradeState::Pending;
        self.selected_slot = None;
        self.visible = false;
    }

    /// 取消交易
    pub fn cancel_trade(&mut self) {
        self.state = TradeState::Cancelled;
        self.end_trade();
    }

    /// 完成交易
    pub fn complete_trade(&mut self) {
        self.state = TradeState::Completed;
        // 交易完成后由服务器处理物品转移
        self.end_trade();
    }

    /// 获取交易状态
    pub fn get_state(&self) -> TradeState {
        self.state
    }

    /// 主方添加物品
    pub fn add_host_item(&mut self, slot: usize, item: UserItem) -> bool {
        if self.state == TradeState::Trading && !self.host.locked {
            self.host.add_item(slot, item)
        } else {
            false
        }
    }

    /// 主方移除物品
    pub fn remove_host_item(&mut self, slot: usize) -> Option<UserItem> {
        if self.state == TradeState::Trading && !self.host.locked {
            self.host.remove_item(slot)
        } else {
            None
        }
    }

    /// 主方设置金币
    pub fn set_host_gold(&mut self, amount: u32) {
        if self.state == TradeState::Trading && !self.host.locked {
            self.host.set_gold(amount);
        }
    }

    /// 客方添加物品 (由服务器更新)
    pub fn add_guest_item(&mut self, slot: usize, item: UserItem) -> bool {
        self.guest.add_item(slot, item)
    }

    /// 客方移除物品 (由服务器更新)
    pub fn remove_guest_item(&mut self, slot: usize) -> Option<UserItem> {
        self.guest.remove_item(slot)
    }

    /// 客方设置金币 (由服务器更新)
    pub fn set_guest_gold(&mut self, amount: u32) {
        self.guest.set_gold(amount);
    }

    /// 主方锁定/解锁交易
    pub fn toggle_host_lock(&mut self) -> bool {
        if self.state == TradeState::Trading {
            self.host.locked = !self.host.locked;
            self.check_both_locked();
            return true;
        }
        false
    }

    /// 设置主方锁定状态
    pub fn set_host_locked(&mut self, locked: bool) {
        if self.state == TradeState::Trading {
            self.host.locked = locked;
            self.check_both_locked();
        }
    }

    /// 设置客方锁定状态 (由服务器更新)
    pub fn set_guest_locked(&mut self, locked: bool) {
        self.guest.locked = locked;
        self.check_both_locked();
    }

    /// 检查双方是否都已锁定
    fn check_both_locked(&mut self) {
        if self.host.locked && self.guest.locked {
            self.state = TradeState::Locked;
        }
    }

    /// 检查是否可以交易
    pub fn can_trade(&self) -> bool {
        self.state == TradeState::Trading
    }

    /// 检查是否已锁定
    pub fn is_locked(&self) -> bool {
        self.state == TradeState::Locked || self.host.locked
    }

    /// 选中槽位
    pub fn select_slot(&mut self, slot: usize) {
        if slot < 10 {
            self.selected_slot = Some(slot);
        }
    }

    /// 取消选中
    pub fn deselect(&mut self) {
        self.selected_slot = None;
    }

    /// 获取总交易价值信息
    pub fn get_trade_summary(&self) -> TradeSummary {
        TradeSummary {
            host_item_count: self.host.item_count(),
            host_gold: self.host.gold,
            guest_item_count: self.guest.item_count(),
            guest_gold: self.guest.gold,
            both_locked: self.host.locked && self.guest.locked,
        }
    }
}

/// 交易摘要
#[derive(Debug, Clone, Copy)]
pub struct TradeSummary {
    pub host_item_count: usize,
    pub host_gold: u32,
    pub guest_item_count: usize,
    pub guest_gold: u32,
    pub both_locked: bool,
}

impl Default for TradeDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl Dialog for TradeDialog {
    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
        if self.state == TradeState::Trading || self.state == TradeState::Locked {
            self.cancel_trade();
        }
    }

    fn update(&mut self, _delta_time: f32) {
        // 更新逻辑 (如交易超时等)
    }

    fn draw(&self) {
        if !self.visible {
            return;
        }
        // TODO: 实际渲染逻辑
        // 绘制交易窗口、物品格子、金币显示、锁定按钮等
    }

    fn is_visible(&self) -> bool {
        self.visible
    }
    
    fn name(&self) -> &str {
        "TradeDialog"
    }
    
    fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.width &&
        y >= self.y && y < self.y + self.height
    }
    
    fn position(&self) -> (i32, i32) {
        (self.x, self.y)
    }
    
    fn size(&self) -> (i32, i32) {
        (self.width, self.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trade_party_creation() {
        let party = TradeParty::new("Player1".to_string());
        assert_eq!(party.name, "Player1");
        assert_eq!(party.items.len(), 10);
        assert_eq!(party.gold, 0);
        assert!(!party.locked);
    }

    #[test]
    fn test_trade_party_items() {
        let mut party = TradeParty::new("Test".to_string());
        
        let item = UserItem {
            unique_id: 1001,
            item_index: 42,
            count: 1,
            ..Default::default()
        };
        
        assert!(party.add_item(0, item.clone()));
        assert_eq!(party.item_count(), 1);
        
        let removed = party.remove_item(0);
        assert!(removed.is_some());
        assert_eq!(party.item_count(), 0);
    }

    #[test]
    fn test_trade_party_gold() {
        let mut party = TradeParty::new("Test".to_string());
        party.set_gold(1000);
        assert_eq!(party.gold, 1000);
    }

    #[test]
    fn test_trade_party_full() {
        let mut party = TradeParty::new("Test".to_string());
        let item = UserItem::default();
        
        for i in 0..10 {
            assert!(party.add_item(i, item.clone()));
        }
        
        assert!(party.is_full());
        assert!(party.find_empty_slot().is_none());
    }

    #[test]
    fn test_trade_dialog_creation() {
        let dialog = TradeDialog::new();
        assert!(!dialog.is_visible());
        assert_eq!(dialog.get_state(), TradeState::Pending);
    }

    #[test]
    fn test_trade_start_end() {
        let mut dialog = TradeDialog::new();
        
        dialog.start_trade("Alice".to_string(), "Bob".to_string());
        assert!(dialog.is_visible());
        assert_eq!(dialog.get_state(), TradeState::Trading);
        assert_eq!(dialog.host.name, "Alice");
        assert_eq!(dialog.guest.name, "Bob");
        
        dialog.end_trade();
        assert!(!dialog.is_visible());
        assert_eq!(dialog.get_state(), TradeState::Pending);
    }

    #[test]
    fn test_trade_add_items() {
        let mut dialog = TradeDialog::new();
        dialog.start_trade("Host".to_string(), "Guest".to_string());
        
        let item = UserItem { unique_id: 2001, ..Default::default() };
        
        assert!(dialog.add_host_item(0, item.clone()));
        assert_eq!(dialog.host.item_count(), 1);
        
        // 不能在锁定后添加
        dialog.set_host_locked(true);
        assert!(!dialog.add_host_item(1, item));
    }

    #[test]
    fn test_trade_locking() {
        let mut dialog = TradeDialog::new();
        dialog.start_trade("A".to_string(), "B".to_string());
        
        assert!(dialog.can_trade());
        assert!(!dialog.is_locked());
        
        dialog.toggle_host_lock();
        assert!(dialog.host.locked);
        assert!(!dialog.guest.locked);
        assert_eq!(dialog.get_state(), TradeState::Trading);
        
        dialog.set_guest_locked(true);
        assert_eq!(dialog.get_state(), TradeState::Locked);
        assert!(dialog.is_locked());
    }

    #[test]
    fn test_trade_cancel() {
        let mut dialog = TradeDialog::new();
        dialog.start_trade("A".to_string(), "B".to_string());
        
        dialog.cancel_trade();
        assert_eq!(dialog.get_state(), TradeState::Cancelled);
        assert!(!dialog.is_visible());
    }

    #[test]
    fn test_trade_summary() {
        let mut dialog = TradeDialog::new();
        dialog.start_trade("Host".to_string(), "Guest".to_string());
        
        let item = UserItem::default();
        dialog.add_host_item(0, item.clone());
        dialog.set_host_gold(500);
        
        dialog.add_guest_item(0, item.clone());
        dialog.add_guest_item(1, item);
        dialog.set_guest_gold(1000);
        
        let summary = dialog.get_trade_summary();
        assert_eq!(summary.host_item_count, 1);
        assert_eq!(summary.host_gold, 500);
        assert_eq!(summary.guest_item_count, 2);
        assert_eq!(summary.guest_gold, 1000);
        assert!(!summary.both_locked);
    }
}
