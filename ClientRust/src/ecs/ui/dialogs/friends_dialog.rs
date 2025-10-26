// ============================================================================
// 好友对话框 - FriendsDialog
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::Canvas;

/// 好友信息
#[derive(Debug, Clone)]
pub struct FriendInfo {
    /// 角色名
    pub name: String,
    
    /// 等级
    pub level: u16,
    
    /// 职业
    pub class: String,
    
    /// 是否在线
    pub online: bool,
    
    /// 备注
    pub memo: String,
}

/// 好友标签页
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FriendsTab {
    Friends,   // 好友列表
    Blacklist, // 黑名单
    Mentor,    // 师徒
}

/// 好友对话框
pub struct FriendsDialog {
    /// 是否可见
    pub visible: bool,
    /// 背景图像索引 (Prguse2)
    background_index: u16,
    
    /// 对话框位置 (屏幕坐标)
    position: (f32, f32),
    
    /// 对话框尺寸
    size: (f32, f32),
    
    /// 当前标签页
    current_tab: FriendsTab,
    
    /// 好友列表
    friends: Vec<FriendInfo>,
    
    /// 黑名单
    blacklist: Vec<String>,
    
    /// 选中的好友索引
    selected_friend: Option<usize>,
}

impl FriendsDialog {
    /// 创建新的好友对话框
    pub fn new() -> Self {
          
        Self {
            visible: false,
            background_index: 1920, // 好友对话框背景 (需要从 C# 客户端确认)
            position: (100.0, 100.0),
            size: (300.0, 400.0),
            current_tab: FriendsTab::Friends,
            friends: Vec::new(),
            blacklist: Vec::new(),
            selected_friend: None,
        }
    }
    
    /// 切换标签页
    pub fn switch_tab(&mut self, tab: FriendsTab) {
        self.current_tab = tab;
        self.selected_friend = None;
        tracing::info!("📑 切换到好友标签: {:?}", tab);
    }
    
    /// 添加好友
    pub fn add_friend(&mut self, friend: FriendInfo) {
        tracing::info!("👥 添加好友: {} (Level {})", friend.name, friend.level);
        self.friends.push(friend);
    }
    
    /// 移除好友
    pub fn remove_friend(&mut self, name: &str) {
        self.friends.retain(|f| f.name != name);
        tracing::info!("🗑️ 移除好友: {}", name);
    }
    
    /// 更新好友在线状态
    pub fn update_friend_status(&mut self, name: &str, online: bool) {
        if let Some(friend) = self.friends.iter_mut().find(|f| f.name == name) {
            friend.online = online;
            tracing::info!("🔄 更新好友状态: {} -> {}", name, if online { "在线" } else { "离线" });
        }
    }
    
    /// 添加到黑名单
    pub fn add_to_blacklist(&mut self, name: String) {
        if !self.blacklist.contains(&name) {
            tracing::info!("🚫 添加到黑名单: {}", name);
            self.blacklist.push(name);
        }
    }
    
    /// 从黑名单移除
    pub fn remove_from_blacklist(&mut self, name: &str) {
        self.blacklist.retain(|n| n != name);
        tracing::info!("✅ 从黑名单移除: {}", name);
    }
    
    /// 获取在线好友数量
    pub fn online_count(&self) -> usize {
        self.friends.iter().filter(|f| f.online).count()
    }
    
    /// 绘制好友对话框
    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        // TODO: 实现实际的好友对话框渲染
        // 1. 绘制背景
        // 2. 绘制标签页按钮
        // 3. 根据标签页绘制列表:
        //    - 好友: 显示在线/离线状态，等级，职业
        //    - 黑名单: 显示名称
        //    - 师徒: 显示师傅/徒弟信息
        // 4. 绘制操作按钮 (私聊、组队、删除等)
        
        Ok(())
    }
    
    /// 处理鼠标点击
    pub fn handle_click(&mut self, x: f32, y: f32) -> bool {
        // 检查点击是否在对话框范围内
        if x >= self.position.0 && x <= self.position.0 + self.size.0
            && y >= self.position.1 && y <= self.position.1 + self.size.1
        {
            // TODO: 处理标签切换、好友选择、按钮点击等
            return true;
        }
        false
    }
    
    /// 检查是否打开
    pub fn is_open(&self) -> bool {
        self.visible
    }
    
    /// 设置打开/关闭
    pub fn set_open(&mut self, open: bool) {
        self.visible = open;
    }
}


