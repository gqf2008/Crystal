// Friend Dialog - 好友对话框
// 管理好友列表和黑名单

use super::Dialog;

/// 好友状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FriendStatus {
    Online,  // 在线
    Offline, // 离线
    Blocked, // 被屏蔽
}

/// 好友信息
#[derive(Debug, Clone)]
pub struct Friend {
    pub name: String,
    pub status: FriendStatus,
    pub level: u16,
    pub class: String,
    pub memo: String,     // 备注
    pub blocked: bool,    // 是否在黑名单
    pub added_time: i64,  // 添加时间(Unix时间戳)
}

impl Friend {
    /// 创建新好友
    pub fn new(name: String) -> Self {
        Self {
            name,
            status: FriendStatus::Offline,
            level: 1,
            class: String::from("Warrior"),
            memo: String::new(),
            blocked: false,
            added_time: 0,
        }
    }

    /// 检查是否在线
    pub fn is_online(&self) -> bool {
        self.status == FriendStatus::Online
    }

    /// 设置在线状态
    pub fn set_online(&mut self, online: bool) {
        self.status = if online {
            FriendStatus::Online
        } else {
            FriendStatus::Offline
        };
    }

    /// 添加到黑名单
    pub fn block(&mut self) {
        self.blocked = true;
        self.status = FriendStatus::Blocked;
    }

    /// 从黑名单移除
    pub fn unblock(&mut self) {
        self.blocked = false;
        self.status = FriendStatus::Offline;
    }

    /// 设置备注
    pub fn set_memo(&mut self, memo: String) {
        self.memo = memo;
    }
}

/// 好友对话框页面
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FriendTab {
    Friends,   // 好友列表
    Blacklist, // 黑名单
}

/// 好友对话框
pub struct FriendDialog {
    visible: bool,
    current_tab: FriendTab,

    // 好友列表
    pub friends: Vec<Friend>,

    // 分页
    pub page: usize,
    pub rows_per_page: usize, // 每页显示行数 (默认12)
    pub start_index: usize,

    // 选中的好友
    pub selected_friend: Option<usize>,
}

impl FriendDialog {
    /// 创建新的好友对话框
    pub fn new() -> Self {
        Self {
            visible: false,
            current_tab: FriendTab::Friends,
            friends: Vec::new(),
            page: 0,
            rows_per_page: 12,
            start_index: 0,
            selected_friend: None,
        }
    }

    /// 切换标签页
    pub fn set_tab(&mut self, tab: FriendTab) {
        self.current_tab = tab;
        self.page = 0;
        self.start_index = 0;
        self.selected_friend = None;
    }

    /// 获取当前标签页
    pub fn get_tab(&self) -> FriendTab {
        self.current_tab
    }

    /// 添加好友
    pub fn add_friend(&mut self, friend: Friend) {
        if !self.friends.iter().any(|f| f.name == friend.name) {
            self.friends.push(friend);
        }
    }

    /// 移除好友
    pub fn remove_friend(&mut self, name: &str) -> bool {
        if let Some(index) = self.friends.iter().position(|f| f.name == name) {
            self.friends.remove(index);
            return true;
        }
        false
    }

    /// 查找好友
    pub fn find_friend(&self, name: &str) -> Option<&Friend> {
        self.friends.iter().find(|f| f.name == name)
    }

    /// 查找好友(可变)
    pub fn find_friend_mut(&mut self, name: &str) -> Option<&mut Friend> {
        self.friends.iter_mut().find(|f| f.name == name)
    }

    /// 更新好友在线状态
    pub fn set_friend_online(&mut self, name: &str, online: bool) {
        if let Some(friend) = self.find_friend_mut(name) {
            friend.set_online(online);
        }
    }

    /// 添加到黑名单
    pub fn block_friend(&mut self, name: &str) -> bool {
        if let Some(friend) = self.find_friend_mut(name) {
            friend.block();
            return true;
        }
        false
    }

    /// 从黑名单移除
    pub fn unblock_friend(&mut self, name: &str) -> bool {
        if let Some(friend) = self.find_friend_mut(name) {
            friend.unblock();
            return true;
        }
        false
    }

    /// 设置好友备注
    pub fn set_friend_memo(&mut self, name: &str, memo: String) -> bool {
        if let Some(friend) = self.find_friend_mut(name) {
            friend.set_memo(memo);
            return true;
        }
        false
    }

    /// 获取当前标签页的好友列表
    pub fn get_filtered_friends(&self) -> Vec<&Friend> {
        match self.current_tab {
            FriendTab::Friends => self.friends.iter().filter(|f| !f.blocked).collect(),
            FriendTab::Blacklist => self.friends.iter().filter(|f| f.blocked).collect(),
        }
    }

    /// 获取在线好友数量
    pub fn online_count(&self) -> usize {
        self.friends.iter().filter(|f| f.is_online() && !f.blocked).count()
    }

    /// 获取黑名单数量
    pub fn blacklist_count(&self) -> usize {
        self.friends.iter().filter(|f| f.blocked).count()
    }

    /// 获取当前页面的好友
    pub fn get_page_friends(&self) -> Vec<&Friend> {
        let filtered = self.get_filtered_friends();
        let start = self.start_index;
        let end = (start + self.rows_per_page).min(filtered.len());
        filtered[start..end].to_vec()
    }

    /// 总页数
    pub fn total_pages(&self) -> usize {
        let count = self.get_filtered_friends().len();
        if count == 0 {
            1
        } else {
            (count + self.rows_per_page - 1) / self.rows_per_page
        }
    }

    /// 下一页
    pub fn next_page(&mut self) {
        let total = self.total_pages();
        if self.page < total - 1 {
            self.page += 1;
            self.start_index = self.page * self.rows_per_page;
            self.selected_friend = None;
        }
    }

    /// 上一页
    pub fn previous_page(&mut self) {
        if self.page > 0 {
            self.page -= 1;
            self.start_index = self.page * self.rows_per_page;
            self.selected_friend = None;
        }
    }

    /// 选中好友
    pub fn select_friend(&mut self, index: usize) {
        let page_friends = self.get_page_friends();
        if index < page_friends.len() {
            self.selected_friend = Some(self.start_index + index);
        }
    }

    /// 取消选中
    pub fn deselect(&mut self) {
        self.selected_friend = None;
    }

    /// 获取选中的好友
    pub fn get_selected_friend(&self) -> Option<&Friend> {
        self.selected_friend.and_then(|idx| self.friends.get(idx))
    }

    /// 清空好友列表
    pub fn clear(&mut self) {
        self.friends.clear();
        self.page = 0;
        self.start_index = 0;
        self.selected_friend = None;
    }
}

impl Default for FriendDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl Dialog for FriendDialog {
    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
        self.deselect();
    }

    fn update(&mut self, _delta_time: f32) {
        // 更新逻辑 (如在线状态刷新等)
    }

    fn draw(&self) {
        if !self.visible {
            return;
        }
        // TODO: 实际渲染逻辑
        // 绘制好友列表、标签页、操作按钮等
    }

    fn is_visible(&self) -> bool {
        self.visible
    }
    
    fn name(&self) -> &str { "FriendDialog" }
    fn contains_point(&self, x: i32, y: i32) -> bool { x >= 0 && x < 400 && y >= 0 && y < 500 }
    fn position(&self) -> (i32, i32) { (0, 0) }
    fn size(&self) -> (i32, i32) { (400, 500) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_friend_creation() {
        let friend = Friend::new("Player1".to_string());
        assert_eq!(friend.name, "Player1");
        assert_eq!(friend.status, FriendStatus::Offline);
        assert!(!friend.blocked);
    }

    #[test]
    fn test_friend_status() {
        let mut friend = Friend::new("Test".to_string());
        
        assert!(!friend.is_online());
        friend.set_online(true);
        assert!(friend.is_online());
        assert_eq!(friend.status, FriendStatus::Online);
    }

    #[test]
    fn test_friend_block() {
        let mut friend = Friend::new("Test".to_string());
        
        friend.block();
        assert!(friend.blocked);
        assert_eq!(friend.status, FriendStatus::Blocked);
        
        friend.unblock();
        assert!(!friend.blocked);
        assert_eq!(friend.status, FriendStatus::Offline);
    }

    #[test]
    fn test_friend_memo() {
        let mut friend = Friend::new("Test".to_string());
        friend.set_memo("Best friend".to_string());
        assert_eq!(friend.memo, "Best friend");
    }

    #[test]
    fn test_friend_dialog_creation() {
        let dialog = FriendDialog::new();
        assert!(!dialog.is_visible());
        assert_eq!(dialog.get_tab(), FriendTab::Friends);
        assert_eq!(dialog.rows_per_page, 12);
    }

    #[test]
    fn test_add_remove_friend() {
        let mut dialog = FriendDialog::new();
        
        let friend = Friend::new("Alice".to_string());
        dialog.add_friend(friend);
        assert_eq!(dialog.friends.len(), 1);
        
        assert!(dialog.remove_friend("Alice"));
        assert_eq!(dialog.friends.len(), 0);
        
        assert!(!dialog.remove_friend("Bob"));
    }

    #[test]
    fn test_find_friend() {
        let mut dialog = FriendDialog::new();
        
        let friend = Friend::new("Bob".to_string());
        dialog.add_friend(friend);
        
        assert!(dialog.find_friend("Bob").is_some());
        assert!(dialog.find_friend("Alice").is_none());
    }

    #[test]
    fn test_friend_online_status() {
        let mut dialog = FriendDialog::new();
        
        let friend = Friend::new("Charlie".to_string());
        dialog.add_friend(friend);
        
        assert_eq!(dialog.online_count(), 0);
        
        dialog.set_friend_online("Charlie", true);
        assert_eq!(dialog.online_count(), 1);
    }

    #[test]
    fn test_block_unblock() {
        let mut dialog = FriendDialog::new();
        
        let friend = Friend::new("Dave".to_string());
        dialog.add_friend(friend);
        
        assert_eq!(dialog.blacklist_count(), 0);
        
        dialog.block_friend("Dave");
        assert_eq!(dialog.blacklist_count(), 1);
        
        dialog.unblock_friend("Dave");
        assert_eq!(dialog.blacklist_count(), 0);
    }

    #[test]
    fn test_filtered_friends() {
        let mut dialog = FriendDialog::new();
        
        let mut friend1 = Friend::new("Friend1".to_string());
        friend1.block();
        let friend2 = Friend::new("Friend2".to_string());
        
        dialog.add_friend(friend1);
        dialog.add_friend(friend2);
        
        // 好友列表标签
        dialog.set_tab(FriendTab::Friends);
        assert_eq!(dialog.get_filtered_friends().len(), 1);
        
        // 黑名单标签
        dialog.set_tab(FriendTab::Blacklist);
        assert_eq!(dialog.get_filtered_friends().len(), 1);
    }

    #[test]
    fn test_pagination() {
        let mut dialog = FriendDialog::new();
        dialog.rows_per_page = 5;
        
        for i in 0..15 {
            dialog.add_friend(Friend::new(format!("Friend{}", i)));
        }
        
        assert_eq!(dialog.total_pages(), 3);
        assert_eq!(dialog.get_page_friends().len(), 5);
        
        dialog.next_page();
        assert_eq!(dialog.page, 1);
        assert_eq!(dialog.get_page_friends().len(), 5);
        
        dialog.next_page();
        assert_eq!(dialog.page, 2);
        assert_eq!(dialog.get_page_friends().len(), 5);
        
        dialog.previous_page();
        assert_eq!(dialog.page, 1);
    }

    #[test]
    fn test_select_friend() {
        let mut dialog = FriendDialog::new();
        
        dialog.add_friend(Friend::new("Eve".to_string()));
        dialog.add_friend(Friend::new("Frank".to_string()));
        
        dialog.select_friend(0);
        assert!(dialog.get_selected_friend().is_some());
        assert_eq!(dialog.get_selected_friend().unwrap().name, "Eve");
        
        dialog.deselect();
        assert!(dialog.get_selected_friend().is_none());
    }

    #[test]
    fn test_set_memo() {
        let mut dialog = FriendDialog::new();
        
        dialog.add_friend(Friend::new("George".to_string()));
        
        assert!(dialog.set_friend_memo("George", "Best friend".to_string()));
        
        let friend = dialog.find_friend("George").unwrap();
        assert_eq!(friend.memo, "Best friend");
    }
}
