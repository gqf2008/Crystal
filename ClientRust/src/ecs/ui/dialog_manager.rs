// ============================================================================
// Dialog Manager - 统一管理所有游戏对话框
// ============================================================================

use ggez::{Context, GameResult, input::keyboard::KeyCode};
use ggez::graphics::Canvas;
use std::collections::HashMap;

/// 对话框类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DialogType {
    Inventory,      // 背包 (I)
    Character,      // 角色 (C)
    Skills,         // 技能 (S)
    Quest,          // 任务 (Q)
    Options,        // 选项 (O)
    Menu,           // 菜单
    GameShop,       // 商城
    Chat,           // 聊天
    MiniMap,        // 小地图
    Trade,          // 交易
    MagicLearning,  // 技能学习
    Group,          // 组队
    Guild,          // 公会
    Friends,        // 好友
    Ranking,        // 排行榜
}

/// 对话框管理器 - 统一管理所有对话框的显示/隐藏、层级、快捷键等
/// 
/// 设计理念：
/// - 每个对话框有独立的显示状态
/// - 支持快捷键切换（如 I 键打开/关闭背包）
/// - 支持对话框层级管理（后打开的在上层）
/// - 支持模态对话框（打开时禁用其他交互）
pub struct DialogManager {
    /// 对话框显示状态 (DialogType -> 是否显示)
    visible: HashMap<DialogType, bool>,
    
    /// 对话框层级顺序（栈结构，最后元素在最上层）
    z_order: Vec<DialogType>,
    
    /// 快捷键映射 (KeyCode -> DialogType)
    hotkeys: HashMap<KeyCode, DialogType>,
    
    /// 当前模态对话框（如果有）
    modal_dialog: Option<DialogType>,
    
    /// 是否锁定所有对话框（如死亡时）
    locked: bool,
}

impl DialogManager {
    /// 创建新的对话框管理器
    pub fn new() -> Self {
        let mut hotkeys = HashMap::new();
        
        // 设置默认快捷键
        hotkeys.insert(KeyCode::KeyI, DialogType::Inventory);
        hotkeys.insert(KeyCode::KeyC, DialogType::Character);
        hotkeys.insert(KeyCode::KeyS, DialogType::Skills);
        hotkeys.insert(KeyCode::KeyQ, DialogType::Quest);
        hotkeys.insert(KeyCode::KeyO, DialogType::Options);
        hotkeys.insert(KeyCode::KeyM, DialogType::MiniMap);
        hotkeys.insert(KeyCode::KeyG, DialogType::Guild);
        hotkeys.insert(KeyCode::KeyP, DialogType::Group);
        hotkeys.insert(KeyCode::KeyF, DialogType::Friends);
        
        // 聊天窗口默认显示
        let mut visible = HashMap::new();
        visible.insert(DialogType::Chat, true);
        
        Self {
            visible,
            z_order: vec![DialogType::Chat],  // 聊天窗口初始在底层
            hotkeys,
            modal_dialog: None,
            locked: false,
        }
    }
    
    /// 检查对话框是否可见
    pub fn is_visible(&self, dialog: DialogType) -> bool {
        self.visible.get(&dialog).copied().unwrap_or(false)
    }
    
    /// 显示对话框
    pub fn show(&mut self, dialog: DialogType) {
        if self.locked && dialog != DialogType::Menu {
            return;  // 锁定时只能打开菜单
        }
        
        // 如果已经显示，提升到顶层
        if self.is_visible(dialog) {
            self.bring_to_front(dialog);
            return;
        }
        
        // 标记为可见
        self.visible.insert(dialog, true);
        
        // 添加到层级栈顶
        self.z_order.retain(|&d| d != dialog);
        self.z_order.push(dialog);
        
        tracing::info!("📂 显示对话框: {:?}", dialog);
    }
    
    /// 隐藏对话框
    pub fn hide(&mut self, dialog: DialogType) {
        self.visible.insert(dialog, false);
        self.z_order.retain(|&d| d != dialog);
        
        tracing::info!("📂 隐藏对话框: {:?}", dialog);
    }
    
    /// 切换对话框显示/隐藏
    pub fn toggle(&mut self, dialog: DialogType) {
        if self.is_visible(dialog) {
            self.hide(dialog);
        } else {
            self.show(dialog);
        }
    }
    
    /// 提升对话框到最上层
    fn bring_to_front(&mut self, dialog: DialogType) {
        self.z_order.retain(|&d| d != dialog);
        self.z_order.push(dialog);
    }
    
    /// 关闭所有对话框（除了聊天窗口）
    pub fn hide_all(&mut self) {
        for (dialog_type, visible) in self.visible.iter_mut() {
            if *dialog_type != DialogType::Chat {
                *visible = false;
            }
        }
        
        // 只保留聊天窗口在层级列表中
        self.z_order.retain(|&d| d == DialogType::Chat);
        
        tracing::info!("📂 关闭所有对话框");
    }
    
    /// 处理快捷键
    pub fn handle_hotkey(&mut self, keycode: KeyCode) -> bool {
        if let Some(&dialog) = self.hotkeys.get(&keycode) {
            self.toggle(dialog);
            return true;
        }
        false
    }
    
    /// 获取对话框渲染顺序（从底层到顶层）
    pub fn get_render_order(&self) -> Vec<DialogType> {
        self.z_order.iter()
            .filter(|&&d| self.is_visible(d))
            .copied()
            .collect()
    }
    
    /// 获取最顶层的对话框
    pub fn get_top_dialog(&self) -> Option<DialogType> {
        self.z_order.iter()
            .rev()
            .find(|&&d| self.is_visible(d))
            .copied()
    }
    
    /// 检查鼠标是否点击在任何对话框上
    pub fn is_mouse_over_any_dialog(&self, mouse_x: f32, mouse_y: f32, 
                                      dialog_bounds: &HashMap<DialogType, (f32, f32, f32, f32)>) -> bool {
        for &dialog in self.z_order.iter().rev() {
            if !self.is_visible(dialog) {
                continue;
            }
            
            if let Some(&(x, y, w, h)) = dialog_bounds.get(&dialog) {
                if mouse_x >= x && mouse_x <= x + w && 
                   mouse_y >= y && mouse_y <= y + h {
                    return true;
                }
            }
        }
        false
    }
    
    /// 设置模态对话框（打开后禁用其他交互）
    pub fn set_modal(&mut self, dialog: Option<DialogType>) {
        self.modal_dialog = dialog;
        if let Some(d) = dialog {
            tracing::info!("🔒 设置模态对话框: {:?}", d);
        }
    }
    
    /// 检查是否有模态对话框
    pub fn has_modal(&self) -> bool {
        self.modal_dialog.is_some()
    }
    
    /// 锁定所有对话框（如死亡时）
    pub fn lock(&mut self) {
        self.locked = true;
        tracing::info!("🔒 锁定所有对话框");
    }
    
    /// 解锁对话框
    pub fn unlock(&mut self) {
        self.locked = false;
        tracing::info!("🔓 解锁对话框");
    }
    
    /// 检查是否锁定
    pub fn is_locked(&self) -> bool {
        self.locked
    }
    
    /// 更新快捷键绑定
    pub fn set_hotkey(&mut self, keycode: KeyCode, dialog: DialogType) {
        self.hotkeys.insert(keycode, dialog);
        tracing::info!("🔧 设置快捷键: {:?} -> {:?}", keycode, dialog);
    }
    
    /// 移除快捷键绑定
    pub fn remove_hotkey(&mut self, keycode: KeyCode) {
        self.hotkeys.remove(&keycode);
    }
    
    /// 获取对话框的快捷键
    pub fn get_hotkey(&self, dialog: DialogType) -> Option<KeyCode> {
        self.hotkeys.iter()
            .find(|(_, &d)| d == dialog)
            .map(|(&k, _)| k)
    }
}

impl Default for DialogManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dialog_visibility() {
        let mut manager = DialogManager::new();
        
        // 初始状态
        assert!(!manager.is_visible(DialogType::Inventory));
        
        // 显示对话框
        manager.show(DialogType::Inventory);
        assert!(manager.is_visible(DialogType::Inventory));
        
        // 切换
        manager.toggle(DialogType::Inventory);
        assert!(!manager.is_visible(DialogType::Inventory));
    }
    
    #[test]
    fn test_z_order() {
        let mut manager = DialogManager::new();
        
        manager.show(DialogType::Inventory);
        manager.show(DialogType::Character);
        manager.show(DialogType::Skills);
        
        let order = manager.get_render_order();
        assert_eq!(order.len(), 4);  // Chat + 3 dialogs
        assert_eq!(order.last(), Some(&DialogType::Skills));  // Skills on top
    }
    
    #[test]
    fn test_hotkeys() {
        let mut manager = DialogManager::new();
        
        // 使用快捷键打开背包
        assert!(manager.handle_hotkey(KeyCode::KeyI));
        assert!(manager.is_visible(DialogType::Inventory));
        
        // 再次按快捷键关闭
        assert!(manager.handle_hotkey(KeyCode::KeyI));
        assert!(!manager.is_visible(DialogType::Inventory));
    }
    
    #[test]
    fn test_lock() {
        let mut manager = DialogManager::new();
        
        manager.lock();
        manager.show(DialogType::Inventory);  // 锁定时无法打开
        assert!(!manager.is_visible(DialogType::Inventory));
        
        manager.show(DialogType::Menu);  // 菜单可以打开
        assert!(manager.is_visible(DialogType::Menu));
    }
}
