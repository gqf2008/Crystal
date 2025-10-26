// ============================================================================
// 行会对话框 - GuildDialog
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::Canvas;

/// 行会成员信息
#[derive(Debug, Clone)]
pub struct GuildMember {
    /// 角色名
    pub name: String,
    
    /// 等级
    pub level: u16,
    
    /// 职位
    pub rank: String,
    
    /// 是否在线
    pub online: bool,
    
    /// 贡献度
    pub contribution: u32,
}

/// 行会标签页
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuildTab {
    Notice,   // 公告
    Members,  // 成员
    Storage,  // 仓库
    Ranks,    // 排行
}

/// 行会对话框
pub struct GuildDialog {
    /// 背景图像索引 (Prguse2)
    background_index: u16,
    
    /// 对话框位置 (屏幕坐标)
    position: (f32, f32),
    
    /// 对话框尺寸
    size: (f32, f32),
    
    /// 当前标签页
    current_tab: GuildTab,
    
    /// 行会名称
    pub guild_name: String,
    
    /// 行会等级
    pub guild_level: u16,
    
    /// 行会公告
    pub notice: String,
    
    /// 成员列表
    members: Vec<GuildMember>,
}

impl GuildDialog {
    /// 创建新的行会对话框
    pub fn new() -> Self {
        Self {
            background_index: 1930, // 行会对话框背景 (需要从 C# 客户端确认)
            position: (200.0, 100.0),
            size: (400.0, 500.0),
            current_tab: GuildTab::Notice,
            guild_name: String::new(),
            guild_level: 0,
            notice: String::new(),
            members: Vec::new(),
        }
    }
    
    /// 切换标签页
    pub fn switch_tab(&mut self, tab: GuildTab) {
        self.current_tab = tab;
        tracing::info!("📑 切换到行会标签: {:?}", tab);
    }
    
    /// 设置行会信息
    pub fn set_guild_info(&mut self, name: String, level: u16, notice: String) {
        self.guild_name = name;
        self.guild_level = level;
        self.notice = notice;
        tracing::info!("🏛️ 行会信息更新: {} (Level {})", self.guild_name, self.guild_level);
    }
    
    /// 添加成员
    pub fn add_member(&mut self, member: GuildMember) {
        tracing::info!("👥 添加行会成员: {} ({})", member.name, member.rank);
        self.members.push(member);
    }
    
    /// 移除成员
    pub fn remove_member(&mut self, name: &str) {
        self.members.retain(|m| m.name != name);
        tracing::info!("🗑️ 移除行会成员: {}", name);
    }
    
    /// 更新成员在线状态
    pub fn update_member_status(&mut self, name: &str, online: bool) {
        if let Some(member) = self.members.iter_mut().find(|m| m.name == name) {
            member.online = online;
        }
    }
    
    /// 清空行会信息
    pub fn clear(&mut self) {
        self.guild_name.clear();
        self.guild_level = 0;
        self.notice.clear();
        self.members.clear();
        tracing::info!("🔄 清空行会信息");
    }
    
    /// 是否加入了行会
    pub fn is_in_guild(&self) -> bool {
        !self.guild_name.is_empty()
    }
    
    /// 获取在线成员数
    pub fn online_members(&self) -> usize {
        self.members.iter().filter(|m| m.online).count()
    }
    
    /// 获取总成员数
    pub fn total_members(&self) -> usize {
        self.members.len()
    }
    
    /// 绘制行会对话框
    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        // TODO: 实现实际的行会对话框渲染
        // 1. 绘制背景
        // 2. 绘制行会名称、等级、图标
        // 3. 绘制标签页按钮
        // 4. 根据标签页绘制不同内容:
        //    - 公告: 显示行会公告文本
        //    - 成员: 显示成员列表 (在线/离线、职位、贡献)
        //    - 仓库: 显示行会仓库物品
        //    - 排行: 显示行会排行榜
        // 5. 绘制操作按钮
        
        Ok(())
    }
    
    /// 处理鼠标点击
    pub fn handle_click(&mut self, x: f32, y: f32) -> bool {
        // 检查点击是否在对话框范围内
        if x >= self.position.0 && x <= self.position.0 + self.size.0
            && y >= self.position.1 && y <= self.position.1 + self.size.1
        {
            // TODO: 处理标签切换、成员选择、按钮点击等
            return true;
        }
        false
    }
}

/// 行会对话框组件
pub struct GuildDialogComponent {
    pub dialog: GuildDialog,
    pub is_open: bool,
}

impl GuildDialogComponent {
    pub fn new() -> Self {
        Self {
            dialog: GuildDialog::new(),
            is_open: false,
        }
    }
}
