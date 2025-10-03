/// DialogManager - 对话框管理器
/// 
/// 负责所有对话框的生命周期管理、Z-order排序、模态栈管理和输入事件分发
/// 
/// # 功能特性
/// - 对话框注册和查找
/// - 可见性控制（显示/隐藏/切换）
/// - 模态对话框栈管理
/// - Z-order自动排序（点击置顶）
/// - 统一的更新和渲染循环
/// - 输入事件分发（鼠标/键盘）
/// - 批量操作（隐藏全部/显示全部）

use std::collections::HashMap;

/// 对话框 trait - 所有对话框必须实现
pub trait Dialog {
    /// 显示对话框
    fn show(&mut self);
    
    /// 隐藏对话框
    fn hide(&mut self);
    
    /// 切换可见性
    fn toggle(&mut self) {
        if self.is_visible() {
            self.hide();
        } else {
            self.show();
        }
    }
    
    /// 检查是否可见
    fn is_visible(&self) -> bool;
    
    /// 更新逻辑（每帧调用）
    fn update(&mut self, delta_time: f32);
    
    /// 渲染对话框（每帧调用）
    fn draw(&self);
    
    /// 获取对话框名称（用于标识）
    fn name(&self) -> &str;
    
    /// 检查鼠标是否在对话框区域内
    fn contains_point(&self, x: i32, y: i32) -> bool;
    
    /// 鼠标移动事件
    fn on_mouse_move(&mut self, _x: i32, _y: i32) -> bool {
        false // 默认不处理
    }
    
    /// 鼠标点击事件
    fn on_mouse_click(&mut self, _x: i32, _y: i32, _button: MouseButton) -> bool {
        false // 默认不处理
    }
    
    /// 键盘按键事件
    fn on_key_press(&mut self, _key: KeyCode) -> bool {
        false // 默认不处理
    }
    
    /// 是否为模态对话框（阻止其他对话框交互）
    fn is_modal(&self) -> bool {
        false // 默认非模态
    }
    
    /// 获取对话框位置
    fn position(&self) -> (i32, i32);
    
    /// 获取对话框大小
    fn size(&self) -> (i32, i32);
}

/// 鼠标按钮枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// 键盘按键代码（简化版）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    Escape,
    Enter,
    Space,
    Tab,
    Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8, Key9, Key0,
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    Up, Down, Left, Right,
    Unknown,
}

/// 对话框管理器
pub struct DialogManager {
    /// 所有注册的对话框（名称 -> 对话框引用ID）
    dialogs: HashMap<String, usize>,
    
    /// 对话框存储（使用索引访问，避免借用问题）
    dialog_storage: Vec<Box<dyn Dialog>>,
    
    /// 可见对话框列表（Z-order排序，后面的在最上层）
    visible_dialogs: Vec<usize>,
    
    /// 模态对话框栈（最后一个是当前模态对话框）
    modal_stack: Vec<usize>,
    
    /// 当前鼠标悬停的对话框
    hover_dialog: Option<usize>,
    
    /// 是否启用对话框管理
    enabled: bool,
}

impl DialogManager {
    /// 创建新的对话框管理器
    pub fn new() -> Self {
        Self {
            dialogs: HashMap::new(),
            dialog_storage: Vec::new(),
            visible_dialogs: Vec::new(),
            modal_stack: Vec::new(),
            hover_dialog: None,
            enabled: true,
        }
    }
    
    /// 注册对话框
    /// 
    /// # Arguments
    /// * `dialog` - 要注册的对话框
    /// 
    /// # Returns
    /// 对话框的唯一ID
    pub fn register_dialog(&mut self, dialog: Box<dyn Dialog>) -> usize {
        let name = dialog.name().to_string();
        let id = self.dialog_storage.len();
        self.dialog_storage.push(dialog);
        self.dialogs.insert(name, id);
        id
    }
    
    /// 根据名称查找对话框ID
    pub fn find_dialog(&self, name: &str) -> Option<usize> {
        self.dialogs.get(name).copied()
    }
    
    /// 获取对话框（不可变引用）
    pub fn get_dialog(&self, id: usize) -> Option<&dyn Dialog> {
        self.dialog_storage.get(id).map(|b| b.as_ref())
    }
    
    // 注意：get_dialog_mut 无法直接返回 &mut dyn Dialog 由于生命周期问题
    // 需要在每个使用点直接访问 self.dialog_storage.get_mut(id)
    
    /// 显示对话框
    pub fn show_dialog(&mut self, name: &str) {
        if let Some(&id) = self.dialogs.get(name) {
            // 先检查状态
            let is_modal = self.dialog_storage.get(id)
                .map(|b| b.is_modal())
                .unwrap_or(false);
            
            // 显示对话框
            if let Some(dialog) = self.dialog_storage.get_mut(id) {
                dialog.show();
            }
            
            // 添加到可见列表（如果不存在）
            if !self.visible_dialogs.contains(&id) {
                self.visible_dialogs.push(id);
            }
            
            // 如果是模态对话框，添加到模态栈
            if is_modal && !self.modal_stack.contains(&id) {
                self.modal_stack.push(id);
            }
        }
    }
    
    /// 隐藏对话框
    pub fn hide_dialog(&mut self, name: &str) {
        if let Some(&id) = self.dialogs.get(name) {
            if let Some(dialog) = self.dialog_storage.get_mut(id) {
                dialog.hide();
            }
            
            // 从可见列表移除
            self.visible_dialogs.retain(|&x| x != id);
            
            // 从模态栈移除
            self.modal_stack.retain(|&x| x != id);
        }
    }
    
    /// 切换对话框可见性
    pub fn toggle_dialog(&mut self, name: &str) {
        if let Some(&id) = self.dialogs.get(name) {
            let is_visible = self.get_dialog(id)
                .map(|d| d.is_visible())
                .unwrap_or(false);
            
            if is_visible {
                self.hide_dialog(name);
            } else {
                self.show_dialog(name);
            }
        }
    }
    
    /// 隐藏所有对话框
    pub fn hide_all(&mut self) {
        let dialog_ids: Vec<usize> = self.visible_dialogs.clone();
        for id in dialog_ids {
            if let Some(dialog) = self.dialog_storage.get_mut(id) {
                dialog.hide();
            }
        }
        self.visible_dialogs.clear();
        self.modal_stack.clear();
    }
    
    /// 隐藏所有非必要对话框（保留 MainDialog, ChatDialog 等）
    pub fn hide_all_except(&mut self, keep_names: &[&str]) {
        let keep_ids: Vec<usize> = keep_names.iter()
            .filter_map(|name| self.dialogs.get(*name))
            .copied()
            .collect();
        
        let dialog_ids: Vec<usize> = self.visible_dialogs.clone();
        for id in dialog_ids {
            if !keep_ids.contains(&id) {
                if let Some(dialog) = self.dialog_storage.get_mut(id) {
                    dialog.hide();
                }
                self.visible_dialogs.retain(|&x| x != id);
                self.modal_stack.retain(|&x| x != id);
            }
        }
    }
    
    /// 将对话框置于最前（Z-order最高）
    pub fn bring_to_front(&mut self, name: &str) {
        if let Some(&id) = self.dialogs.get(name) {
            // 从当前位置移除
            self.visible_dialogs.retain(|&x| x != id);
            // 添加到末尾（最上层）
            self.visible_dialogs.push(id);
        }
    }
    
    /// 将对话框置于最后（Z-order最低）
    pub fn send_to_back(&mut self, name: &str) {
        if let Some(&id) = self.dialogs.get(name) {
            // 从当前位置移除
            self.visible_dialogs.retain(|&x| x != id);
            // 添加到开头（最下层）
            self.visible_dialogs.insert(0, id);
        }
    }
    
    /// 检查是否有模态对话框激活
    pub fn is_modal_active(&self) -> bool {
        !self.modal_stack.is_empty()
    }
    
    /// 获取当前模态对话框
    pub fn current_modal_dialog(&self) -> Option<usize> {
        self.modal_stack.last().copied()
    }
    
    /// 更新所有可见对话框
    pub fn update_all(&mut self, delta_time: f32) {
        if !self.enabled {
            return;
        }
        
        // 更新所有可见对话框（按Z-order顺序）
        for &id in &self.visible_dialogs.clone() {
            if let Some(dialog) = self.dialog_storage.get_mut(id) {
                dialog.update(delta_time);
            }
        }
    }
    
    /// 渲染所有可见对话框
    pub fn draw_all(&self) {
        if !self.enabled {
            return;
        }
        
        // 按Z-order顺序渲染（从底层到顶层）
        for &id in &self.visible_dialogs {
            if let Some(dialog) = self.get_dialog(id) {
                if dialog.is_visible() {
                    dialog.draw();
                }
            }
        }
    }
    
    /// 处理鼠标移动事件
    pub fn on_mouse_move(&mut self, x: i32, y: i32) -> bool {
        if !self.enabled {
            return false;
        }
        
        // 如果有模态对话框，只处理模态对话框
        if let Some(modal_id) = self.current_modal_dialog() {
            if let Some(dialog) = self.dialog_storage.get_mut(modal_id) {
                return dialog.on_mouse_move(x, y);
            }
            return false;
        }
        
        // 从顶层到底层检查鼠标悬停
        for &id in self.visible_dialogs.iter().rev() {
            if let Some(dialog) = self.get_dialog(id) {
                if dialog.contains_point(x, y) {
                    self.hover_dialog = Some(id);
                    
                    // 通知对话框鼠标移动（需要可变引用）
                    if let Some(dialog_mut) = self.dialog_storage.get_mut(id) {
                        if dialog_mut.on_mouse_move(x, y) {
                            return true; // 事件被处理
                        }
                    }
                    return false; // 鼠标在对话框内，阻止事件继续传播
                }
            }
        }
        
        self.hover_dialog = None;
        false
    }
    
    /// 处理鼠标点击事件
    pub fn on_mouse_click(&mut self, x: i32, y: i32, button: MouseButton) -> bool {
        if !self.enabled {
            return false;
        }
        
        // 如果有模态对话框，只处理模态对话框
        if let Some(modal_id) = self.current_modal_dialog() {
            if let Some(dialog) = self.dialog_storage.get_mut(modal_id) {
                return dialog.on_mouse_click(x, y, button);
            }
            return false;
        }
        
        // 从顶层到底层检查点击
        for &id in self.visible_dialogs.iter().rev() {
            if let Some(dialog) = self.get_dialog(id) {
                if dialog.contains_point(x, y) {
                    // 点击时自动置顶
                    let name = dialog.name().to_string();
                    self.bring_to_front(&name);
                    
                    // 通知对话框点击事件
                    if let Some(dialog_mut) = self.dialog_storage.get_mut(id) {
                        dialog_mut.on_mouse_click(x, y, button);
                    }
                    return true; // 事件被处理
                }
            }
        }
        
        false
    }
    
    /// 处理键盘按键事件
    pub fn on_key_press(&mut self, key: KeyCode) -> bool {
        if !self.enabled {
            return false;
        }
        
        // ESC 关闭最顶层对话框（或模态对话框）
        if key == KeyCode::Escape {
            if let Some(modal_id) = self.current_modal_dialog() {
                if let Some(dialog) = self.dialog_storage.get_mut(modal_id) {
                    dialog.hide();
                }
                self.modal_stack.pop();
                self.visible_dialogs.retain(|&x| x != modal_id);
                return true;
            }
            
            // 关闭最顶层对话框
            if let Some(&top_id) = self.visible_dialogs.last() {
                if let Some(dialog) = self.dialog_storage.get_mut(top_id) {
                    let name = dialog.name().to_string();
                    self.hide_dialog(&name);
                    return true;
                }
            }
        }
        
        // 如果有模态对话框，只传递给模态对话框
        if let Some(modal_id) = self.current_modal_dialog() {
            if let Some(dialog) = self.dialog_storage.get_mut(modal_id) {
                return dialog.on_key_press(key);
            }
            return false;
        }
        
        // 传递给最顶层对话框
        if let Some(&top_id) = self.visible_dialogs.last() {
            if let Some(dialog) = self.dialog_storage.get_mut(top_id) {
                return dialog.on_key_press(key);
            }
        }
        
        false
    }
    
    /// 启用/禁用对话框管理器
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    
    /// 检查是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    /// 获取可见对话框数量
    pub fn visible_count(&self) -> usize {
        self.visible_dialogs.len()
    }
    
    /// 获取所有已注册对话框名称
    pub fn dialog_names(&self) -> Vec<String> {
        self.dialogs.keys().cloned().collect()
    }
}

impl Default for DialogManager {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;
    
    // 模拟对话框
    struct MockDialog {
        name: String,
        visible: bool,
        modal: bool,
        position: (i32, i32),
        size: (i32, i32),
        click_count: usize,
    }
    
    impl MockDialog {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                visible: false,
                modal: false,
                position: (100, 100),
                size: (200, 150),
                click_count: 0,
            }
        }
        
        fn new_modal(name: &str) -> Self {
            Self {
                name: name.to_string(),
                visible: false,
                modal: true,
                position: (150, 150),
                size: (300, 200),
                click_count: 0,
            }
        }
    }
    
    impl Dialog for MockDialog {
        fn show(&mut self) {
            self.visible = true;
        }
        
        fn hide(&mut self) {
            self.visible = false;
        }
        
        fn is_visible(&self) -> bool {
            self.visible
        }
        
        fn update(&mut self, _delta_time: f32) {}
        
        fn draw(&self) {}
        
        fn name(&self) -> &str {
            &self.name
        }
        
        fn contains_point(&self, x: i32, y: i32) -> bool {
            x >= self.position.0 && x < self.position.0 + self.size.0 &&
            y >= self.position.1 && y < self.position.1 + self.size.1
        }
        
        fn on_mouse_click(&mut self, _x: i32, _y: i32, _button: MouseButton) -> bool {
            self.click_count += 1;
            true
        }
        
        fn is_modal(&self) -> bool {
            self.modal
        }
        
        fn position(&self) -> (i32, i32) {
            self.position
        }
        
        fn size(&self) -> (i32, i32) {
            self.size
        }
    }
    
    #[test]
    fn test_register_and_find() {
        let mut manager = DialogManager::new();
        let dialog = Box::new(MockDialog::new("TestDialog"));
        
        let id = manager.register_dialog(dialog);
        assert_eq!(manager.find_dialog("TestDialog"), Some(id));
        assert_eq!(manager.find_dialog("NonExistent"), None);
    }
    
    #[test]
    fn test_show_hide() {
        let mut manager = DialogManager::new();
        manager.register_dialog(Box::new(MockDialog::new("Dialog1")));
        
        manager.show_dialog("Dialog1");
        assert_eq!(manager.visible_count(), 1);
        
        manager.hide_dialog("Dialog1");
        assert_eq!(manager.visible_count(), 0);
    }
    
    #[test]
    fn test_toggle() {
        let mut manager = DialogManager::new();
        manager.register_dialog(Box::new(MockDialog::new("Dialog1")));
        
        manager.toggle_dialog("Dialog1");
        assert_eq!(manager.visible_count(), 1);
        
        manager.toggle_dialog("Dialog1");
        assert_eq!(manager.visible_count(), 0);
    }
    
    #[test]
    fn test_z_order() {
        let mut manager = DialogManager::new();
        manager.register_dialog(Box::new(MockDialog::new("Dialog1")));
        manager.register_dialog(Box::new(MockDialog::new("Dialog2")));
        manager.register_dialog(Box::new(MockDialog::new("Dialog3")));
        
        manager.show_dialog("Dialog1");
        manager.show_dialog("Dialog2");
        manager.show_dialog("Dialog3");
        
        // Dialog3 应该在最上层
        assert_eq!(manager.visible_dialogs.last(), manager.find_dialog("Dialog3").as_ref());
        
        // 将 Dialog1 置顶
        manager.bring_to_front("Dialog1");
        assert_eq!(manager.visible_dialogs.last(), manager.find_dialog("Dialog1").as_ref());
        
        // 将 Dialog1 置底
        manager.send_to_back("Dialog1");
        assert_eq!(manager.visible_dialogs.first(), manager.find_dialog("Dialog1").as_ref());
    }
    
    #[test]
    fn test_modal_dialog() {
        let mut manager = DialogManager::new();
        manager.register_dialog(Box::new(MockDialog::new("Normal")));
        manager.register_dialog(Box::new(MockDialog::new_modal("Modal")));
        
        manager.show_dialog("Normal");
        assert!(!manager.is_modal_active());
        
        manager.show_dialog("Modal");
        assert!(manager.is_modal_active());
        assert_eq!(manager.current_modal_dialog(), manager.find_dialog("Modal"));
        
        manager.hide_dialog("Modal");
        assert!(!manager.is_modal_active());
    }
    
    #[test]
    fn test_hide_all() {
        let mut manager = DialogManager::new();
        manager.register_dialog(Box::new(MockDialog::new("Dialog1")));
        manager.register_dialog(Box::new(MockDialog::new("Dialog2")));
        manager.register_dialog(Box::new(MockDialog::new("Dialog3")));
        
        manager.show_dialog("Dialog1");
        manager.show_dialog("Dialog2");
        manager.show_dialog("Dialog3");
        assert_eq!(manager.visible_count(), 3);
        
        manager.hide_all();
        assert_eq!(manager.visible_count(), 0);
    }
    
    #[test]
    fn test_hide_all_except() {
        let mut manager = DialogManager::new();
        manager.register_dialog(Box::new(MockDialog::new("Main")));
        manager.register_dialog(Box::new(MockDialog::new("Chat")));
        manager.register_dialog(Box::new(MockDialog::new("Inventory")));
        manager.register_dialog(Box::new(MockDialog::new("Trade")));
        
        manager.show_dialog("Main");
        manager.show_dialog("Chat");
        manager.show_dialog("Inventory");
        manager.show_dialog("Trade");
        assert_eq!(manager.visible_count(), 4);
        
        manager.hide_all_except(&["Main", "Chat"]);
        assert_eq!(manager.visible_count(), 2);
    }
    
    #[test]
    fn test_mouse_click_z_order() {
        let mut manager = DialogManager::new();
        manager.register_dialog(Box::new(MockDialog::new("Dialog1")));
        manager.register_dialog(Box::new(MockDialog::new("Dialog2")));
        
        manager.show_dialog("Dialog1");
        manager.show_dialog("Dialog2");
        
        // 点击 Dialog1 (100-300, 100-250)
        manager.on_mouse_click(150, 150, MouseButton::Left);
        
        // Dialog1 应该被置顶
        assert_eq!(manager.visible_dialogs.last(), manager.find_dialog("Dialog1").as_ref());
    }
    
    #[test]
    fn test_esc_key_closes_dialog() {
        let mut manager = DialogManager::new();
        manager.register_dialog(Box::new(MockDialog::new("Dialog1")));
        manager.register_dialog(Box::new(MockDialog::new("Dialog2")));
        
        manager.show_dialog("Dialog1");
        manager.show_dialog("Dialog2");
        assert_eq!(manager.visible_count(), 2);
        
        // ESC 关闭最顶层对话框
        manager.on_key_press(KeyCode::Escape);
        assert_eq!(manager.visible_count(), 1);
        
        manager.on_key_press(KeyCode::Escape);
        assert_eq!(manager.visible_count(), 0);
    }
    
    #[test]
    fn test_enable_disable() {
        let mut manager = DialogManager::new();
        manager.register_dialog(Box::new(MockDialog::new("Dialog1")));
        
        manager.show_dialog("Dialog1");
        assert_eq!(manager.visible_count(), 1);
        
        manager.set_enabled(false);
        assert!(!manager.is_enabled());
        
        // 禁用时不处理输入
        assert!(!manager.on_mouse_click(150, 150, MouseButton::Left));
        assert!(!manager.on_key_press(KeyCode::Escape));
    }
}

