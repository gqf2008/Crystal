// ============================================================================
// 组队对话框 - GroupDialog
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::Canvas;

/// 队员信息
#[derive(Debug, Clone)]
pub struct GroupMember {
    /// 角色名
    pub name: String,
    
    /// 等级
    pub level: u16,
    
    /// 职业
    pub class: String,
    
    /// 当前HP
    pub hp: i32,
    
    /// 最大HP
    pub max_hp: i32,
    
    /// 当前MP
    pub mp: i32,
    
    /// 最大MP
    pub max_mp: i32,
    
    /// 是否是队长
    pub is_leader: bool,
}

/// 组队对话框
pub struct GroupDialog {
    /// 背景图像索引 (Prguse2)
    background_index: u16,
    
    /// 对话框位置 (屏幕坐标)
    position: (f32, f32),
    
    /// 对话框尺寸
    size: (f32, f32),
    
    /// 队伍成员列表 (最多6人)
    members: Vec<GroupMember>,
    
    /// 是否允许组队请求
    pub allow_group: bool,
}

impl GroupDialog {
    /// 创建新的组队对话框
    pub fn new() -> Self {
        Self {
            background_index: 1928, // 组队对话框背景 (需要从 C# 客户端确认)
            position: (10.0, 200.0), // 左侧
            size: (200.0, 300.0),
            members: Vec::new(),
            allow_group: true,
        }
    }
    
    /// 添加队员
    pub fn add_member(&mut self, member: GroupMember) -> bool {
        if self.members.len() >= 6 {
            tracing::warn!("❌ 队伍已满，无法添加队员: {}", member.name);
            return false;
        }
        
        tracing::info!("👥 添加队员: {} (Level {})", member.name, member.level);
        self.members.push(member);
        true
    }
    
    /// 移除队员
    pub fn remove_member(&mut self, name: &str) {
        self.members.retain(|m| m.name != name);
        tracing::info!("🗑️ 移除队员: {}", name);
    }
    
    /// 更新队员状态
    pub fn update_member(&mut self, name: &str, hp: i32, mp: i32) {
        if let Some(member) = self.members.iter_mut().find(|m| m.name == name) {
            member.hp = hp;
            member.mp = mp;
        }
    }
    
    /// 设置队长
    pub fn set_leader(&mut self, name: &str) {
        for member in &mut self.members {
            member.is_leader = member.name == name;
        }
        tracing::info!("👑 新队长: {}", name);
    }
    
    /// 清空队伍
    pub fn clear(&mut self) {
        self.members.clear();
        tracing::info!("🔄 解散队伍");
    }
    
    /// 是否在队伍中
    pub fn is_in_group(&self) -> bool {
        !self.members.is_empty()
    }
    
    /// 获取队伍人数
    pub fn member_count(&self) -> usize {
        self.members.len()
    }
    
    /// 获取队长
    pub fn get_leader(&self) -> Option<&GroupMember> {
        self.members.iter().find(|m| m.is_leader)
    }
    
    /// 绘制组队对话框
    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        // TODO: 实现实际的组队对话框渲染
        // 1. 绘制背景
        // 2. 绘制队员列表 (头像、名称、等级、职业)
        // 3. 绘制HP/MP条
        // 4. 绘制队长图标
        // 5. 绘制操作按钮 (离队、踢人、转让队长等)
        
        Ok(())
    }
    
    /// 处理鼠标点击
    pub fn handle_click(&mut self, x: f32, y: f32) -> bool {
        // 检查点击是否在对话框范围内
        if x >= self.position.0 && x <= self.position.0 + self.size.0
            && y >= self.position.1 && y <= self.position.1 + self.size.1
        {
            // TODO: 处理队员选择、按钮点击等
            return true;
        }
        false
    }
}

/// 组队对话框组件
pub struct GroupDialogComponent {
    pub dialog: GroupDialog,
    pub is_open: bool,
}

impl GroupDialogComponent {
    pub fn new() -> Self {
        Self {
            dialog: GroupDialog::new(),
            is_open: false,
        }
    }
}
