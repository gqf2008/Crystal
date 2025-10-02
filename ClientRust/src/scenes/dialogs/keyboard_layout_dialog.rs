/// KeyboardLayoutDialog - 键盘设置对话框
/// 
/// 管理游戏键盘绑定设置
/// 
/// # 功能特性
/// - 显示所有键盘绑定
/// - 修改键盘绑定
/// - 重置为默认值
/// - 严格/宽松分配规则
/// - 滚动查看所有绑定

use std::collections::HashMap;

/// 键盘功能选项（简化版）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeybindOption {
    // 移动
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    
    // 技能栏
    Bar1Skill1, Bar1Skill2, Bar1Skill3, Bar1Skill4,
    Bar1Skill5, Bar1Skill6, Bar1Skill7, Bar1Skill8,
    Bar2Skill1, Bar2Skill2, Bar2Skill3, Bar2Skill4,
    Bar2Skill5, Bar2Skill6, Bar2Skill7, Bar2Skill8,
    
    // 对话框
    Inventory,
    Character,
    Skills,
    Guild,
    Trade,
    Exit,
    Logout,
    Help,
    Ranking,
    
    // 功能键
    Pickup,
    Attack,
    Screenshot,
    Minimap,
    Bigmap,
    
    // 其他
    Mount,
    Fishing,
    Creature,
}

impl KeybindOption {
    /// 获取描述文本
    pub fn description(&self) -> &str {
        match self {
            KeybindOption::MoveUp => "Move Up",
            KeybindOption::MoveDown => "Move Down",
            KeybindOption::MoveLeft => "Move Left",
            KeybindOption::MoveRight => "Move Right",
            KeybindOption::Bar1Skill1 => "Skill Bar 1 - Slot 1",
            KeybindOption::Bar1Skill2 => "Skill Bar 1 - Slot 2",
            KeybindOption::Inventory => "Inventory",
            KeybindOption::Character => "Character",
            KeybindOption::Exit => "Exit Game",
            KeybindOption::Logout => "Logout",
            KeybindOption::Help => "Help",
            KeybindOption::Pickup => "Pickup Item",
            KeybindOption::Attack => "Attack",
            KeybindOption::Mount => "Mount",
            KeybindOption::Fishing => "Fishing",
            _ => "Unknown",
        }
    }
    
    /// 获取分组
    pub fn group(&self) -> &str {
        match self {
            KeybindOption::MoveUp | KeybindOption::MoveDown |
            KeybindOption::MoveLeft | KeybindOption::MoveRight => "Movement",
            
            KeybindOption::Bar1Skill1 | KeybindOption::Bar1Skill2 | 
            KeybindOption::Bar1Skill3 | KeybindOption::Bar1Skill4 |
            KeybindOption::Bar1Skill5 | KeybindOption::Bar1Skill6 |
            KeybindOption::Bar1Skill7 | KeybindOption::Bar1Skill8 => "Skill Bar 1",
            
            KeybindOption::Bar2Skill1 | KeybindOption::Bar2Skill2 |
            KeybindOption::Bar2Skill3 | KeybindOption::Bar2Skill4 |
            KeybindOption::Bar2Skill5 | KeybindOption::Bar2Skill6 |
            KeybindOption::Bar2Skill7 | KeybindOption::Bar2Skill8 => "Skill Bar 2",
            
            KeybindOption::Inventory | KeybindOption::Character |
            KeybindOption::Skills | KeybindOption::Guild |
            KeybindOption::Trade => "Dialogs",
            
            KeybindOption::Exit | KeybindOption::Logout |
            KeybindOption::Help | KeybindOption::Ranking => "System",
            
            _ => "Other",
        }
    }
}

/// 键盘绑定数据
#[derive(Debug, Clone)]
pub struct KeyBind {
    pub function: KeybindOption,
    pub key: String,  // 如 "F1", "A", "Ctrl+1"
    pub require_ctrl: bool,
    pub require_shift: bool,
    pub require_alt: bool,
    pub require_tilde: bool,
}

impl KeyBind {
    pub fn new(function: KeybindOption, key: &str) -> Self {
        Self {
            function,
            key: key.to_string(),
            require_ctrl: false,
            require_shift: false,
            require_alt: false,
            require_tilde: false,
        }
    }
    
    /// 获取完整键盘绑定字符串
    pub fn get_full_key(&self) -> String {
        let mut parts = Vec::new();
        
        if self.require_ctrl { parts.push("Ctrl"); }
        if self.require_shift { parts.push("Shift"); }
        if self.require_alt { parts.push("Alt"); }
        if self.require_tilde { parts.push("~"); }
        
        parts.push(&self.key);
        
        parts.join("+")
    }
}

/// 键盘设置对话框
pub struct KeyboardLayoutDialog {
    /// 是否可见
    pub visible: bool,
    
    /// 对话框位置（居中）
    pub position: (i32, i32),
    
    /// 对话框大小 (Index 119)
    pub size: (i32, i32),
    
    /// 是否可移动
    pub movable: bool,
    
    /// 是否排序
    pub sort: bool,
    
    /// 所有键盘绑定
    pub key_bindings: Vec<KeyBind>,
    
    /// 默认键盘绑定（用于重置）
    pub default_bindings: Vec<KeyBind>,
    
    /// 滚动位置
    pub top_line: usize,
    
    /// 每页显示行数
    pub line_count: usize,
    
    /// 是否严格模式
    pub enforce_mode: bool,
    
    /// 正在等待输入的绑定
    pub waiting_for_bind: Option<usize>,
}

impl KeyboardLayoutDialog {
    /// 创建新的键盘设置对话框
    pub fn new(screen_width: i32, screen_height: i32) -> Self {
        let size = (520, 450);
        let position = ((screen_width - size.0) / 2, (screen_height - size.1) / 2);
        
        // 初始化默认键盘绑定
        let mut key_bindings = Vec::new();
        key_bindings.push(KeyBind::new(KeybindOption::MoveUp, "W"));
        key_bindings.push(KeyBind::new(KeybindOption::MoveDown, "S"));
        key_bindings.push(KeyBind::new(KeybindOption::MoveLeft, "A"));
        key_bindings.push(KeyBind::new(KeybindOption::MoveRight, "D"));
        key_bindings.push(KeyBind::new(KeybindOption::Inventory, "I"));
        key_bindings.push(KeyBind::new(KeybindOption::Character, "C"));
        key_bindings.push(KeyBind::new(KeybindOption::Exit, "F12"));
        key_bindings.push(KeyBind::new(KeybindOption::Logout, "F11"));
        key_bindings.push(KeyBind::new(KeybindOption::Help, "H"));
        
        // 技能栏绑定
        for i in 1..=8 {
            let mut bind = KeyBind::new(
                match i {
                    1 => KeybindOption::Bar1Skill1,
                    2 => KeybindOption::Bar1Skill2,
                    3 => KeybindOption::Bar1Skill3,
                    4 => KeybindOption::Bar1Skill4,
                    5 => KeybindOption::Bar1Skill5,
                    6 => KeybindOption::Bar1Skill6,
                    7 => KeybindOption::Bar1Skill7,
                    8 => KeybindOption::Bar1Skill8,
                    _ => KeybindOption::Bar1Skill1,
                },
                &i.to_string()
            );
            bind.require_tilde = true;
            key_bindings.push(bind);
        }
        
        let default_bindings = key_bindings.clone();
        
        Self {
            visible: false,
            position,
            size,
            movable: true,
            sort: true,
            key_bindings,
            default_bindings,
            top_line: 0,
            line_count: 16,
            enforce_mode: true,
            waiting_for_bind: None,
        }
    }
    
    /// 显示对话框
    pub fn show(&mut self) {
        self.visible = true;
    }
    
    /// 隐藏对话框
    pub fn hide(&mut self) {
        self.visible = false;
        self.waiting_for_bind = None;
    }
    
    /// 切换可见性
    pub fn toggle(&mut self) {
        if self.visible {
            self.hide();
        } else {
            self.show();
        }
    }
    
    /// 检查是否可见
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    
    /// 重置所有键盘绑定为默认值
    pub fn reset_to_default(&mut self) {
        self.key_bindings = self.default_bindings.clone();
    }
    
    /// 向上滚动
    pub fn scroll_up(&mut self) {
        if self.top_line > 0 {
            self.top_line -= 1;
        }
    }
    
    /// 向下滚动
    pub fn scroll_down(&mut self) {
        let max_top = self.key_bindings.len().saturating_sub(self.line_count);
        if self.top_line < max_top {
            self.top_line += 1;
        }
    }
    
    /// 获取当前显示的绑定列表
    pub fn get_visible_bindings(&self) -> Vec<&KeyBind> {
        let end = (self.top_line + self.line_count).min(self.key_bindings.len());
        self.key_bindings[self.top_line..end].iter().collect()
    }
    
    /// 切换严格模式
    pub fn toggle_enforce_mode(&mut self) {
        self.enforce_mode = !self.enforce_mode;
    }
    
    /// 获取严格模式文本
    pub fn get_enforce_text(&self) -> &str {
        if self.enforce_mode {
            "Assign Rule: Strict"
        } else {
            "Assign Rule: Relaxed"
        }
    }
    
    /// 开始等待键盘输入（为指定绑定重新分配按键）
    pub fn start_waiting_for_key(&mut self, binding_index: usize) {
        if binding_index < self.key_bindings.len() {
            self.waiting_for_bind = Some(binding_index);
        }
    }
    
    /// 分配新按键
    pub fn assign_key(&mut self, new_key: String) {
        if let Some(index) = self.waiting_for_bind {
            if index < self.key_bindings.len() {
                self.key_bindings[index].key = new_key;
            }
            self.waiting_for_bind = None;
        }
    }
    
    /// 取消等待输入
    pub fn cancel_waiting(&mut self) {
        self.waiting_for_bind = None;
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_keyboard_dialog_creation() {
        let dialog = KeyboardLayoutDialog::new(1024, 768);
        
        assert!(!dialog.visible);
        assert!(dialog.enforce_mode);
        assert_eq!(dialog.top_line, 0);
        assert!(dialog.key_bindings.len() > 0);
    }
    
    #[test]
    fn test_keybind_descriptions() {
        assert_eq!(KeybindOption::MoveUp.description(), "Move Up");
        assert_eq!(KeybindOption::Exit.description(), "Exit Game");
        assert_eq!(KeybindOption::Inventory.description(), "Inventory");
    }
    
    #[test]
    fn test_keybind_groups() {
        assert_eq!(KeybindOption::MoveUp.group(), "Movement");
        assert_eq!(KeybindOption::Bar1Skill1.group(), "Skill Bar 1");
        assert_eq!(KeybindOption::Inventory.group(), "Dialogs");
    }
    
    #[test]
    fn test_full_key_string() {
        let mut bind = KeyBind::new(KeybindOption::Bar1Skill1, "1");
        bind.require_tilde = true;
        assert_eq!(bind.get_full_key(), "~+1");
        
        bind.require_ctrl = true;
        assert_eq!(bind.get_full_key(), "Ctrl+~+1");
    }
    
    #[test]
    fn test_reset_to_default() {
        let mut dialog = KeyboardLayoutDialog::new(1024, 768);
        
        // 修改一个绑定
        dialog.key_bindings[0].key = "X".to_string();
        assert_eq!(dialog.key_bindings[0].key, "X");
        
        // 重置
        dialog.reset_to_default();
        assert_eq!(dialog.key_bindings[0].key, "W"); // 应该恢复为W
    }
    
    #[test]
    fn test_scrolling() {
        let mut dialog = KeyboardLayoutDialog::new(1024, 768);
        dialog.line_count = 5;
        
        // 添加更多绑定以便测试滚动
        for i in 0..20 {
            dialog.key_bindings.push(KeyBind::new(KeybindOption::Pickup, &format!("K{}", i)));
        }
        
        assert_eq!(dialog.top_line, 0);
        
        dialog.scroll_down();
        assert_eq!(dialog.top_line, 1);
        
        dialog.scroll_up();
        assert_eq!(dialog.top_line, 0);
    }
    
    #[test]
    fn test_visible_bindings() {
        let dialog = KeyboardLayoutDialog::new(1024, 768);
        
        let visible = dialog.get_visible_bindings();
        assert!(visible.len() <= dialog.line_count);
    }
    
    #[test]
    fn test_waiting_for_key() {
        let mut dialog = KeyboardLayoutDialog::new(1024, 768);
        
        assert!(dialog.waiting_for_bind.is_none());
        
        dialog.start_waiting_for_key(0);
        assert_eq!(dialog.waiting_for_bind, Some(0));
        
        dialog.assign_key("X".to_string());
        assert!(dialog.waiting_for_bind.is_none());
        assert_eq!(dialog.key_bindings[0].key, "X");
    }
    
    #[test]
    fn test_enforce_mode() {
        let mut dialog = KeyboardLayoutDialog::new(1024, 768);
        
        assert!(dialog.enforce_mode);
        assert_eq!(dialog.get_enforce_text(), "Assign Rule: Strict");
        
        dialog.toggle_enforce_mode();
        assert!(!dialog.enforce_mode);
        assert_eq!(dialog.get_enforce_text(), "Assign Rule: Relaxed");
    }
}
