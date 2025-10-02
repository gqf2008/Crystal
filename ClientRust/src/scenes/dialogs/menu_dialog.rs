/// MenuDialog - 游戏菜单对话框
/// 
/// 提供游戏主菜单功能，包括退出、登出、帮助、键盘设置、排行榜等
/// 
/// # 功能特性
/// - 退出游戏 (Exit)
/// - 登出账号 (Logout)
/// - 帮助文档 (Help)
/// - 键盘设置 (Keyboard Layout)
/// - 排行榜 (Ranking)
/// - 制作系统 (Crafting, 隐藏)
/// - 智能生物 (Intelligent Creature)
/// - 坐骑系统 (Ride/Mount)
/// - 钓鱼系统 (Fishing)
/// - 好友列表 (Friend)
/// - 导师系统 (Mentor)
/// - 关系系统 (Relationship)
/// - 组队系统 (Group)
/// - 公会系统 (Guild)

use std::collections::HashMap;

/// 菜单按钮类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MenuButton {
    Exit,                    // 退出游戏
    Logout,                  // 登出账号
    Help,                    // 帮助
    KeyboardLayout,          // 键盘设置
    Ranking,                 // 排行榜
    Crafting,                // 制作 (隐藏)
    IntelligentCreature,     // 智能生物
    Ride,                    // 坐骑
    Fishing,                 // 钓鱼
    Friend,                  // 好友
    Mentor,                  // 导师
    Relationship,            // 关系
    Group,                   // 组队
    Guild,                   // 公会
}

impl MenuButton {
    /// 获取按钮提示文本
    pub fn hint(&self, keybind: &str) -> String {
        match self {
            MenuButton::Exit => format!("Exit ({})", keybind),
            MenuButton::Logout => format!("Logout ({})", keybind),
            MenuButton::Help => format!("Help ({})", keybind),
            MenuButton::KeyboardLayout => format!("Keyboard ({})", keybind),
            MenuButton::Ranking => format!("Ranking ({})", keybind),
            MenuButton::Crafting => "Crafting".to_string(),
            MenuButton::IntelligentCreature => format!("Creatures ({})", keybind),
            MenuButton::Ride => format!("Mount ({})", keybind),
            MenuButton::Fishing => format!("Fishing ({})", keybind),
            MenuButton::Friend => "Friends".to_string(),
            MenuButton::Mentor => "Mentor".to_string(),
            MenuButton::Relationship => "Relationship".to_string(),
            MenuButton::Group => "Group".to_string(),
            MenuButton::Guild => "Guild".to_string(),
        }
    }
    
    /// 获取按钮位置（基于索引）
    pub fn position(&self) -> (i32, i32) {
        let y_base = 12;
        let y_spacing = 19; // 每个按钮间隔19像素
        
        let index = match self {
            MenuButton::Exit => 0,
            MenuButton::Logout => 1,
            MenuButton::Help => 2,
            MenuButton::KeyboardLayout => 3,
            MenuButton::Ranking => 4,
            MenuButton::Crafting => 5,
            MenuButton::IntelligentCreature => 6,
            MenuButton::Ride => 7,
            MenuButton::Fishing => 8,
            MenuButton::Friend => 9,
            MenuButton::Mentor => 10,
            MenuButton::Relationship => 11,
            MenuButton::Group => 12,
            MenuButton::Guild => 13,
        };
        
        (3, y_base + index * y_spacing)
    }
}

/// 游戏菜单对话框
pub struct MenuDialog {
    /// 是否可见
    pub visible: bool,
    
    /// 对话框位置
    pub position: (i32, i32),
    
    /// 对话框大小 (从 Index 567 推断)
    pub size: (i32, i32),
    
    /// 是否可移动
    pub movable: bool,
    
    /// 是否排序（Z-order）
    pub sort: bool,
    
    /// 按钮启用状态
    pub button_enabled: HashMap<MenuButton, bool>,
    
    /// 按钮悬停状态
    pub button_hover: HashMap<MenuButton, bool>,
    
    /// 按钮按下状态
    pub button_pressed: HashMap<MenuButton, bool>,
    
    /// 按钮索引配置（Index, HoverIndex, PressedIndex）
    pub button_indices: HashMap<MenuButton, (i32, i32, i32)>,
}

impl MenuDialog {
    /// 创建新的菜单对话框
    /// 
    /// # Arguments
    /// * `screen_width` - 屏幕宽度（用于定位）
    /// * `main_dialog_y` - MainDialog 的 Y 坐标
    pub fn new(screen_width: i32, main_dialog_y: i32) -> Self {
        let size = (80, 280); // 估算大小
        let position = (screen_width - size.0, main_dialog_y - size.1 + 15);
        
        // 初始化按钮索引配置
        let mut button_indices = HashMap::new();
        button_indices.insert(MenuButton::Exit, (633, 634, 635));
        button_indices.insert(MenuButton::Logout, (636, 637, 638));
        button_indices.insert(MenuButton::Help, (1970, 1971, 1972));
        button_indices.insert(MenuButton::KeyboardLayout, (1973, 1974, 1975));
        button_indices.insert(MenuButton::Ranking, (2000, 2001, 2002));
        button_indices.insert(MenuButton::Crafting, (2000, 2001, 2002)); // 同上
        button_indices.insert(MenuButton::IntelligentCreature, (431, 432, 433)); // Prguse2
        button_indices.insert(MenuButton::Ride, (1976, 1977, 1978));
        button_indices.insert(MenuButton::Fishing, (1979, 1980, 1981));
        button_indices.insert(MenuButton::Friend, (0, 0, 0)); // TODO: 补充索引
        button_indices.insert(MenuButton::Mentor, (0, 0, 0));
        button_indices.insert(MenuButton::Relationship, (0, 0, 0));
        button_indices.insert(MenuButton::Group, (0, 0, 0));
        button_indices.insert(MenuButton::Guild, (0, 0, 0));
        
        // 初始化启用状态
        let mut button_enabled = HashMap::new();
        for &button in &[
            MenuButton::Exit,
            MenuButton::Logout,
            MenuButton::Help,
            MenuButton::KeyboardLayout,
            MenuButton::Ranking,
            MenuButton::IntelligentCreature,
            MenuButton::Ride,
            MenuButton::Fishing,
            MenuButton::Friend,
            MenuButton::Mentor,
            MenuButton::Relationship,
            MenuButton::Group,
            MenuButton::Guild,
        ] {
            button_enabled.insert(button, true);
        }
        button_enabled.insert(MenuButton::Crafting, false); // 制作按钮默认隐藏
        
        Self {
            visible: false,
            position,
            size,
            movable: true,
            sort: true,
            button_enabled,
            button_hover: HashMap::new(),
            button_pressed: HashMap::new(),
            button_indices,
        }
    }
    
    /// 显示对话框
    pub fn show(&mut self) {
        self.visible = true;
    }
    
    /// 隐藏对话框
    pub fn hide(&mut self) {
        self.visible = false;
        self.button_hover.clear();
        self.button_pressed.clear();
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
    
    /// 更新对话框位置（跟随 MainDialog）
    pub fn update_position(&mut self, screen_width: i32, main_dialog_y: i32) {
        self.position = (screen_width - self.size.0, main_dialog_y - self.size.1 + 15);
    }
    
    /// 启用/禁用按钮
    pub fn set_button_enabled(&mut self, button: MenuButton, enabled: bool) {
        self.button_enabled.insert(button, enabled);
    }
    
    /// 检查按钮是否启用
    pub fn is_button_enabled(&self, button: MenuButton) -> bool {
        self.button_enabled.get(&button).copied().unwrap_or(false)
    }
    
    /// 检查按钮是否悬停
    pub fn is_button_hover(&self, button: MenuButton) -> bool {
        self.button_hover.get(&button).copied().unwrap_or(false)
    }
    
    /// 检查按钮是否按下
    pub fn is_button_pressed(&self, button: MenuButton) -> bool {
        self.button_pressed.get(&button).copied().unwrap_or(false)
    }
    
    /// 鼠标移动事件
    pub fn on_mouse_move(&mut self, x: i32, y: i32) {
        // 清除所有悬停状态
        self.button_hover.clear();
        
        if !self.visible {
            return;
        }
        
        // 检查鼠标是否在按钮上
        for &button in &[
            MenuButton::Exit,
            MenuButton::Logout,
            MenuButton::Help,
            MenuButton::KeyboardLayout,
            MenuButton::Ranking,
            MenuButton::Crafting,
            MenuButton::IntelligentCreature,
            MenuButton::Ride,
            MenuButton::Fishing,
            MenuButton::Friend,
            MenuButton::Mentor,
            MenuButton::Relationship,
            MenuButton::Group,
            MenuButton::Guild,
        ] {
            if !self.is_button_enabled(button) {
                continue;
            }
            
            let (btn_x, btn_y) = button.position();
            let btn_x = self.position.0 + btn_x;
            let btn_y = self.position.1 + btn_y;
            
            // 按钮大小约 70x17
            if x >= btn_x && x < btn_x + 70 && y >= btn_y && y < btn_y + 17 {
                self.button_hover.insert(button, true);
                break;
            }
        }
    }
    
    /// 鼠标点击事件
    /// 
    /// # Returns
    /// 点击的按钮（如果有）
    pub fn on_mouse_click(&mut self, x: i32, y: i32) -> Option<MenuButton> {
        if !self.visible {
            return None;
        }
        
        for &button in &[
            MenuButton::Exit,
            MenuButton::Logout,
            MenuButton::Help,
            MenuButton::KeyboardLayout,
            MenuButton::Ranking,
            MenuButton::Crafting,
            MenuButton::IntelligentCreature,
            MenuButton::Ride,
            MenuButton::Fishing,
            MenuButton::Friend,
            MenuButton::Mentor,
            MenuButton::Relationship,
            MenuButton::Group,
            MenuButton::Guild,
        ] {
            if !self.is_button_enabled(button) {
                continue;
            }
            
            let (btn_x, btn_y) = button.position();
            let btn_x = self.position.0 + btn_x;
            let btn_y = self.position.1 + btn_y;
            
            if x >= btn_x && x < btn_x + 70 && y >= btn_y && y < btn_y + 17 {
                return Some(button);
            }
        }
        
        None
    }
    
    /// 获取按钮显示索引（考虑悬停和按下状态）
    pub fn get_button_index(&self, button: MenuButton) -> i32 {
        let (normal_idx, hover_idx, pressed_idx) = 
            self.button_indices.get(&button).copied().unwrap_or((0, 0, 0));
        
        if self.is_button_pressed(button) {
            pressed_idx
        } else if self.is_button_hover(button) {
            hover_idx
        } else {
            normal_idx
        }
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_menu_dialog_creation() {
        let dialog = MenuDialog::new(1024, 600);
        
        assert!(!dialog.visible);
        assert!(dialog.movable);
        assert!(dialog.sort);
    }
    
    #[test]
    fn test_show_hide() {
        let mut dialog = MenuDialog::new(1024, 600);
        
        dialog.show();
        assert!(dialog.is_visible());
        
        dialog.hide();
        assert!(!dialog.is_visible());
    }
    
    #[test]
    fn test_toggle() {
        let mut dialog = MenuDialog::new(1024, 600);
        
        dialog.toggle();
        assert!(dialog.is_visible());
        
        dialog.toggle();
        assert!(!dialog.is_visible());
    }
    
    #[test]
    fn test_button_enabled() {
        let mut dialog = MenuDialog::new(1024, 600);
        
        assert!(dialog.is_button_enabled(MenuButton::Exit));
        assert!(dialog.is_button_enabled(MenuButton::Help));
        assert!(!dialog.is_button_enabled(MenuButton::Crafting)); // 默认禁用
        
        dialog.set_button_enabled(MenuButton::Crafting, true);
        assert!(dialog.is_button_enabled(MenuButton::Crafting));
    }
    
    #[test]
    fn test_button_positions() {
        let positions = vec![
            (MenuButton::Exit, (3, 12)),
            (MenuButton::Logout, (3, 31)),
            (MenuButton::Help, (3, 50)),
            (MenuButton::KeyboardLayout, (3, 69)),
            (MenuButton::Ranking, (3, 88)),
        ];
        
        for (button, expected) in positions {
            assert_eq!(button.position(), expected);
        }
    }
    
    #[test]
    fn test_mouse_hover() {
        let mut dialog = MenuDialog::new(1024, 600);
        dialog.show();
        
        // 模拟鼠标移动到 Exit 按钮
        let (btn_x, btn_y) = MenuButton::Exit.position();
        let abs_x = dialog.position.0 + btn_x + 10;
        let abs_y = dialog.position.1 + btn_y + 8;
        
        dialog.on_mouse_move(abs_x, abs_y);
        assert!(dialog.is_button_hover(MenuButton::Exit));
    }
    
    #[test]
    fn test_mouse_click() {
        let mut dialog = MenuDialog::new(1024, 600);
        dialog.show();
        
        // 点击 Exit 按钮
        let (btn_x, btn_y) = MenuButton::Exit.position();
        let abs_x = dialog.position.0 + btn_x + 10;
        let abs_y = dialog.position.1 + btn_y + 8;
        
        let clicked = dialog.on_mouse_click(abs_x, abs_y);
        assert_eq!(clicked, Some(MenuButton::Exit));
    }
    
    #[test]
    fn test_update_position() {
        let mut dialog = MenuDialog::new(1024, 600);
        let old_pos = dialog.position;
        
        dialog.update_position(1280, 700);
        assert_ne!(dialog.position, old_pos);
        assert_eq!(dialog.position.0, 1280 - dialog.size.0);
    }
    
    #[test]
    fn test_button_indices() {
        let dialog = MenuDialog::new(1024, 600);
        
        // Exit 按钮索引：633(normal), 634(hover), 635(pressed)
        let indices = dialog.button_indices.get(&MenuButton::Exit).unwrap();
        assert_eq!(indices, &(633, 634, 635));
    }
    
    #[test]
    fn test_button_hints() {
        assert_eq!(MenuButton::Exit.hint("F12"), "Exit (F12)");
        assert_eq!(MenuButton::Help.hint("H"), "Help (H)");
        assert_eq!(MenuButton::Fishing.hint("N"), "Fishing (N)");
    }
}
