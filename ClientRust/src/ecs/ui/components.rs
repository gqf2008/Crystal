// ============================================================================
// UI 组件 - 用于 ECS 系统的UI元素
// ============================================================================

use ggez::graphics::Color;

/// 角色状态组件 - 存储角色的基础属性
#[derive(Debug, Clone)]
pub struct CharacterStatus {
    /// 角色名称
    pub name: String,
    
    /// 当前等级
    pub level: u16,
    
    /// 当前生命值
    pub health: i32,
    
    /// 最大生命值
    pub max_health: i32,
    
    /// 当前魔法值
    pub mana: i32,
    
    /// 最大魔法值
    pub max_mana: i32,
    
    /// 当前经验值
    pub experience: i64,
    
    /// 升级所需经验
    pub max_experience: i64,
}

impl Default for CharacterStatus {
    fn default() -> Self {
        Self {
            name: "勇士".to_string(),
            level: 1,
            health: 100,
            max_health: 100,
            mana: 50,
            max_mana: 50,
            experience: 0,
            max_experience: 100,
        }
    }
}

impl CharacterStatus {
    /// 获取生命值百分比 (0.0 - 1.0)
    pub fn health_percent(&self) -> f32 {
        if self.max_health == 0 {
            0.0
        } else {
            (self.health as f32 / self.max_health as f32).clamp(0.0, 1.0)
        }
    }
    
    /// 获取魔法值百分比 (0.0 - 1.0)
    pub fn mana_percent(&self) -> f32 {
        if self.max_mana == 0 {
            0.0
        } else {
            (self.mana as f32 / self.max_mana as f32).clamp(0.0, 1.0)
        }
    }
    
    /// 获取经验值百分比 (0.0 - 1.0)
    pub fn exp_percent(&self) -> f32 {
        if self.max_experience == 0 {
            0.0
        } else {
            (self.experience as f32 / self.max_experience as f64 as f32).clamp(0.0, 1.0)
        }
    }
}

/// 血条组件
#[derive(Debug, Clone)]
pub struct HealthBar {
    /// 屏幕位置 (左上角)
    pub x: f32,
    pub y: f32,
    
    /// 血条尺寸
    pub width: f32,
    pub height: f32,
    
    /// 是否显示
    pub visible: bool,
    
    /// 是否显示数值
    pub show_text: bool,
}

impl Default for HealthBar {
    fn default() -> Self {
        Self {
            x: 20.0,
            y: 20.0,
            width: 200.0,
            height: 20.0,
            visible: true,
            show_text: true,
        }
    }
}

impl HealthBar {
    /// 获取血条颜色（根据血量百分比）
    pub fn get_color(percent: f32) -> Color {
        if percent > 0.6 {
            Color::from_rgb(0, 200, 0) // 绿色
        } else if percent > 0.3 {
            Color::from_rgb(255, 165, 0) // 橙色
        } else {
            Color::from_rgb(200, 0, 0) // 红色
        }
    }
}

/// 魔法条组件
#[derive(Debug, Clone)]
pub struct ManaBar {
    /// 屏幕位置 (左上角)
    pub x: f32,
    pub y: f32,
    
    /// 魔法条尺寸
    pub width: f32,
    pub height: f32,
    
    /// 是否显示
    pub visible: bool,
    
    /// 是否显示数值
    pub show_text: bool,
}

impl Default for ManaBar {
    fn default() -> Self {
        Self {
            x: 20.0,
            y: 45.0,
            width: 200.0,
            height: 20.0,
            visible: true,
            show_text: true,
        }
    }
}

impl ManaBar {
    /// 获取魔法条颜色
    pub fn get_color(_percent: f32) -> Color {
        Color::from_rgb(0, 100, 255) // 蓝色
    }
}

/// 经验条组件
#[derive(Debug, Clone)]
pub struct ExpBar {
    /// 屏幕位置 (左下角)
    pub x: f32,
    pub y: f32,
    
    /// 经验条尺寸
    pub width: f32,
    pub height: f32,
    
    /// 是否显示
    pub visible: bool,
    
    /// 是否显示百分比
    pub show_percent: bool,
}

impl ExpBar {
    /// 创建经验条（通常在屏幕底部）
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        Self {
            x: 0.0,
            y: screen_height - 15.0,
            width: screen_width,
            height: 15.0,
            visible: true,
            show_percent: true,
        }
    }
    
    /// 获取经验条颜色
    pub fn get_color(_percent: f32) -> Color {
        Color::from_rgb(255, 215, 0) // 金色
    }
}

/// 技能栏组件
#[derive(Debug, Clone)]
pub struct SkillBar {
    /// 屏幕位置 (左上角)
    pub x: f32,
    pub y: f32,
    
    /// 技能槽尺寸
    pub slot_size: f32,
    
    /// 技能槽间距
    pub slot_spacing: f32,
    
    /// 是否显示
    pub visible: bool,
    
    /// 技能列表 (8个槽位，对应 F1-F8)
    pub skills: Vec<Option<Skill>>,
}

impl Default for SkillBar {
    fn default() -> Self {
        Self {
            x: 20.0,
            y: 70.0,
            slot_size: 40.0,
            slot_spacing: 5.0,
            visible: true,
            skills: vec![None; 8],
        }
    }
}

/// 技能信息
#[derive(Debug, Clone)]
pub struct Skill {
    /// 技能ID
    pub id: u32,
    
    /// 技能名称
    pub name: String,
    
    /// 技能图标索引
    pub icon: u32,
    
    /// 冷却时间（秒）
    pub cooldown: f32,
    
    /// 当前冷却剩余时间
    pub remaining_cooldown: f32,
}

impl Skill {
    /// 是否在冷却中
    pub fn is_cooling_down(&self) -> bool {
        self.remaining_cooldown > 0.0
    }
    
    /// 获取冷却百分比 (0.0 - 1.0)
    pub fn cooldown_percent(&self) -> f32 {
        if self.cooldown == 0.0 {
            0.0
        } else {
            (self.remaining_cooldown / self.cooldown).clamp(0.0, 1.0)
        }
    }
}

/// 聊天窗口组件
#[derive(Debug, Clone)]
pub struct ChatWindow {
    /// 屏幕位置
    pub x: f32,
    pub y: f32,
    
    /// 窗口尺寸
    pub width: f32,
    pub height: f32,
    
    /// 是否显示
    pub visible: bool,
    
    /// 聊天消息列表（最多保存100条）
    pub messages: Vec<ChatMessage>,
    
    /// 最大消息数量
    pub max_messages: usize,
}

impl ChatWindow {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        Self {
            x: 20.0,
            y: screen_height - 200.0,
            width: 400.0,
            height: 150.0,
            visible: true,
            messages: Vec::new(),
            max_messages: 100,
        }
    }
    
    /// 添加聊天消息
    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
        
        // 限制消息数量
        if self.messages.len() > self.max_messages {
            self.messages.remove(0);
        }
    }
}

/// 聊天消息
#[derive(Debug, Clone)]
pub struct ChatMessage {
    /// 发送者名称
    pub sender: String,
    
    /// 消息内容
    pub content: String,
    
    /// 消息类型（用于显示不同颜色）
    pub msg_type: ChatMessageType,
    
    /// 时间戳
    pub timestamp: std::time::Instant,
}

/// 聊天消息类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatMessageType {
    /// 普通聊天
    Normal,
    
    /// 系统消息
    System,
    
    /// 组队消息
    Party,
    
    /// 公会消息
    Guild,
    
    /// 私聊
    Whisper,
    
    /// 喊话
    Shout,
}

impl ChatMessageType {
    /// 获取消息类型对应的颜色
    pub fn get_color(&self) -> Color {
        match self {
            ChatMessageType::Normal => Color::WHITE,
            ChatMessageType::System => Color::from_rgb(255, 215, 0), // 金色
            ChatMessageType::Party => Color::from_rgb(0, 255, 0), // 绿色
            ChatMessageType::Guild => Color::from_rgb(0, 191, 255), // 深天蓝
            ChatMessageType::Whisper => Color::from_rgb(255, 105, 180), // 粉红
            ChatMessageType::Shout => Color::from_rgb(255, 0, 0), // 红色
        }
    }
}

// ============================================================================
// UI 对话框 ECS 组件封装
// ============================================================================
//
// 将 UI 对话框封装为 ECS 组件，使其能够存储在 World 中
// 符合 ECS 的数据驱动设计原则
//
// ============================================================================

use super::{
    MainDialog, InventoryDialog, CharacterDialog, 
    SkillBarDialog, ChatDialog
};

/// 主对话框组件
pub struct MainDialogComp {
    pub dialog: MainDialog,
}

impl MainDialogComp {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        Self {
            dialog: MainDialog::new(screen_width, screen_height),
        }
    }
}

/// 背包对话框组件
pub struct InventoryDialogComp {
    pub dialog: InventoryDialog,
    pub is_open: bool,
}

impl InventoryDialogComp {
    pub fn new() -> Self {
        Self {
            dialog: InventoryDialog::new(),
            is_open: false,
        }
    }
}

/// 角色对话框组件
pub struct CharacterDialogComp {
    pub dialog: CharacterDialog,
    pub is_open: bool,
}

impl CharacterDialogComp {
    pub fn new() -> Self {
        Self {
            dialog: CharacterDialog::new(),
            is_open: false,
        }
    }
}

/// 技能栏组件
pub struct SkillBarComp {
    pub dialog: SkillBarDialog,
    pub bar_index: u8,
}

impl SkillBarComp {
    pub fn new(bar_index: u8) -> Self {
        Self {
            dialog: SkillBarDialog::new(bar_index),
            bar_index,
        }
    }
}

/// 聊天对话框组件
pub struct ChatDialogComp {
    pub dialog: ChatDialog,
}

impl ChatDialogComp {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            dialog: ChatDialog::new(x, y),
        }
    }
    
    /// 检查输入框是否激活
    pub fn is_input_active(&self) -> bool {
        self.dialog.is_input_active()
    }
    
    /// 取消输入
    pub fn deactivate_input(&mut self) {
        self.dialog.deactivate_input();
    }
}

/// 技能学习对话框组件
pub struct MagicLearningDialogComp {
    pub dialog: super::MagicLearningDialog,
    pub is_open: bool,
}

impl MagicLearningDialogComp {
    pub fn new() -> Self {
        Self {
            dialog: super::MagicLearningDialog::new(),
            is_open: false,
        }
    }
    
    /// 检查是否打开
    pub fn is_open(&self) -> bool {
        self.is_open
    }
    
    /// 切换显示/隐藏
    pub fn toggle(&mut self) {
        self.is_open = !self.is_open;
        self.dialog.set_visible(self.is_open);
    }
}
