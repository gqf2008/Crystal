// NPC Dialog - NPC对话框
// 用于与NPC交互，显示对话文本、选项、任务等

use super::Dialog;
use std::collections::VecDeque;

/// NPC对话类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NPCDialogType {
    Normal,       // 普通对话
    Quest,        // 任务对话
    Shop,         // 商店
    Storage,      // 仓库
    Crafting,     // 制作
    Refining,     // 精炼
    Guild,        // 公会
    Transport,    // 传送
}

/// NPC对话选项
#[derive(Debug, Clone)]
pub struct NPCOption {
    pub text: String,      // 选项文本
    pub action: String,    // 动作标识 (如 "@buy", "@sell", "QUEST_1")
    pub enabled: bool,     // 是否可点击
    pub key: Option<u8>,   // 快捷键 (1-9)
}

impl NPCOption {
    /// 创建新选项
    pub fn new(text: String, action: String) -> Self {
        Self {
            text,
            action,
            enabled: true,
            key: None,
        }
    }

    /// 禁用选项
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// 启用选项
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// 设置快捷键
    pub fn with_key(mut self, key: u8) -> Self {
        self.key = Some(key);
        self
    }
}

/// NPC对话页面
#[derive(Debug, Clone)]
pub struct NPCPage {
    pub title: String,           // 标题/NPC名称
    pub content: Vec<String>,    // 对话内容(多行)
    pub options: Vec<NPCOption>, // 可选选项
    pub can_scroll: bool,        // 内容是否可滚动
    pub scroll_position: usize,  // 滚动位置
    pub max_visible_lines: usize, // 最大可见行数
}

impl NPCPage {
    /// 创建新页面
    pub fn new(title: String) -> Self {
        Self {
            title,
            content: Vec::new(),
            options: Vec::new(),
            can_scroll: false,
            scroll_position: 0,
            max_visible_lines: 8,
        }
    }

    /// 添加内容行
    pub fn add_line(&mut self, line: String) {
        self.content.push(line);
        self.update_scroll_state();
    }

    /// 添加多行内容
    pub fn add_lines(&mut self, lines: Vec<String>) {
        self.content.extend(lines);
        self.update_scroll_state();
    }

    /// 添加选项
    pub fn add_option(&mut self, option: NPCOption) {
        self.options.push(option);
    }

    /// 清空内容
    pub fn clear(&mut self) {
        self.content.clear();
        self.options.clear();
        self.scroll_position = 0;
        self.can_scroll = false;
    }

    /// 更新滚动状态
    fn update_scroll_state(&mut self) {
        self.can_scroll = self.content.len() > self.max_visible_lines;
    }

    /// 向上滚动
    pub fn scroll_up(&mut self) {
        if self.scroll_position > 0 {
            self.scroll_position -= 1;
        }
    }

    /// 向下滚动
    pub fn scroll_down(&mut self) {
        if self.can_scroll {
            let max_scroll = self.content.len().saturating_sub(self.max_visible_lines);
            if self.scroll_position < max_scroll {
                self.scroll_position += 1;
            }
        }
    }

    /// 获取可见内容
    pub fn get_visible_content(&self) -> &[String] {
        let start = self.scroll_position;
        let end = (start + self.max_visible_lines).min(self.content.len());
        &self.content[start..end]
    }

    /// 检查是否可以继续向下滚动
    pub fn can_scroll_down(&self) -> bool {
        if !self.can_scroll {
            return false;
        }
        let max_scroll = self.content.len().saturating_sub(self.max_visible_lines);
        self.scroll_position < max_scroll
    }
}

/// NPC对话框
pub struct NPCDialog {
    visible: bool,
    dialog_type: NPCDialogType,

    // 当前页面
    current_page: Option<NPCPage>,

    // 页面历史 (用于返回上一页)
    page_history: VecDeque<NPCPage>,
    max_history: usize,

    // 当前选中的选项索引
    selected_option: Option<usize>,

    // NPC信息
    pub npc_id: u32,
    pub npc_name: String,
    pub npc_image: u16, // NPC图像索引
}

impl NPCDialog {
    /// 创建新的NPC对话框
    pub fn new() -> Self {
        Self {
            visible: false,
            dialog_type: NPCDialogType::Normal,
            current_page: None,
            page_history: VecDeque::new(),
            max_history: 10,
            selected_option: None,
            npc_id: 0,
            npc_name: String::new(),
            npc_image: 0,
        }
    }

    /// 打开对话框
    pub fn open(&mut self, npc_id: u32, npc_name: String, npc_image: u16, dialog_type: NPCDialogType) {
        self.npc_id = npc_id;
        self.npc_name = npc_name;
        self.npc_image = npc_image;
        self.dialog_type = dialog_type;
        self.current_page = None;
        self.page_history.clear();
        self.selected_option = None;
        self.visible = true;
    }

    /// 设置当前页面
    pub fn set_page(&mut self, page: NPCPage) {
        // 保存当前页到历史
        if let Some(current) = self.current_page.take() {
            self.page_history.push_back(current);
            if self.page_history.len() > self.max_history {
                self.page_history.pop_front();
            }
        }
        self.current_page = Some(page);
        self.selected_option = None;
    }

    /// 获取当前页面
    pub fn get_page(&self) -> Option<&NPCPage> {
        self.current_page.as_ref()
    }

    /// 获取当前页面(可变)
    pub fn get_page_mut(&mut self) -> Option<&mut NPCPage> {
        self.current_page.as_mut()
    }

    /// 返回上一页
    pub fn go_back(&mut self) -> bool {
        if let Some(previous) = self.page_history.pop_back() {
            self.current_page = Some(previous);
            self.selected_option = None;
            true
        } else {
            false
        }
    }

    /// 清空历史
    pub fn clear_history(&mut self) {
        self.page_history.clear();
    }

    /// 选择选项
    pub fn select_option(&mut self, index: usize) -> Option<String> {
        if let Some(page) = &self.current_page {
            if index < page.options.len() && page.options[index].enabled {
                self.selected_option = Some(index);
                return Some(page.options[index].action.clone());
            }
        }
        None
    }

    /// 按快捷键选择选项
    pub fn select_option_by_key(&mut self, key: u8) -> Option<String> {
        if let Some(page) = &self.current_page {
            if let Some(index) = page.options.iter().position(|opt| opt.key == Some(key)) {
                return self.select_option(index);
            }
        }
        None
    }

    /// 获取当前选中的选项索引
    pub fn get_selected_option(&self) -> Option<usize> {
        self.selected_option
    }

    /// 向上滚动
    pub fn scroll_up(&mut self) {
        if let Some(page) = &mut self.current_page {
            page.scroll_up();
        }
    }

    /// 向下滚动
    pub fn scroll_down(&mut self) {
        if let Some(page) = &mut self.current_page {
            page.scroll_down();
        }
    }

    /// 滚动到顶部
    pub fn scroll_to_top(&mut self) {
        if let Some(page) = &mut self.current_page {
            page.scroll_position = 0;
        }
    }

    /// 滚动到底部
    pub fn scroll_to_bottom(&mut self) {
        if let Some(page) = &mut self.current_page {
            if page.can_scroll {
                page.scroll_position = page.content.len().saturating_sub(page.max_visible_lines);
            }
        }
    }

    /// 获取对话类型
    pub fn get_dialog_type(&self) -> NPCDialogType {
        self.dialog_type
    }

    /// 检查是否可以返回上一页
    pub fn can_go_back(&self) -> bool {
        !self.page_history.is_empty()
    }
}

impl Default for NPCDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl Dialog for NPCDialog {
    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
        self.current_page = None;
        self.page_history.clear();
        self.selected_option = None;
    }

    fn update(&mut self, _delta_time: f32) {
        // 更新逻辑 (如动画等)
    }

    fn draw(&self) {
        if !self.visible {
            return;
        }
        // TODO: 实际渲染逻辑
        // 绘制对话框背景、NPC头像、对话内容、选项按钮等
    }

    fn is_visible(&self) -> bool {
        self.visible
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_npc_option_creation() {
        let option = NPCOption::new("购买物品".to_string(), "@buy".to_string());
        assert_eq!(option.text, "购买物品");
        assert!(option.enabled);
        assert!(option.key.is_none());
    }

    #[test]
    fn test_npc_option_state() {
        let mut option = NPCOption::new("测试".to_string(), "@test".to_string());
        assert!(option.enabled);
        
        option.disable();
        assert!(!option.enabled);
        
        option.enable();
        assert!(option.enabled);
    }

    #[test]
    fn test_npc_page_creation() {
        let page = NPCPage::new("测试NPC".to_string());
        assert_eq!(page.title, "测试NPC");
        assert_eq!(page.content.len(), 0);
        assert!(!page.can_scroll);
    }

    #[test]
    fn test_npc_page_content() {
        let mut page = NPCPage::new("商人".to_string());
        page.add_line("欢迎光临！".to_string());
        page.add_line("需要什么帮助吗？".to_string());
        
        assert_eq!(page.content.len(), 2);
        assert_eq!(page.get_visible_content().len(), 2);
    }

    #[test]
    fn test_npc_page_scrolling() {
        let mut page = NPCPage::new("长对话".to_string());
        page.max_visible_lines = 3;
        
        // 添加5行内容
        for i in 1..=5 {
            page.add_line(format!("第{}行", i));
        }
        
        assert!(page.can_scroll);
        assert_eq!(page.get_visible_content().len(), 3);
        
        page.scroll_down();
        assert_eq!(page.scroll_position, 1);
        
        page.scroll_up();
        assert_eq!(page.scroll_position, 0);
    }

    #[test]
    fn test_npc_dialog_creation() {
        let dialog = NPCDialog::new();
        assert!(!dialog.is_visible());
        assert!(dialog.get_page().is_none());
        assert!(!dialog.can_go_back());
    }

    #[test]
    fn test_npc_dialog_open() {
        let mut dialog = NPCDialog::new();
        dialog.open(1001, "铁匠".to_string(), 42, NPCDialogType::Shop);
        
        assert!(dialog.is_visible());
        assert_eq!(dialog.npc_id, 1001);
        assert_eq!(dialog.npc_name, "铁匠");
        assert_eq!(dialog.get_dialog_type(), NPCDialogType::Shop);
    }

    #[test]
    fn test_npc_dialog_page_navigation() {
        let mut dialog = NPCDialog::new();
        dialog.open(1001, "商人".to_string(), 42, NPCDialogType::Normal);
        
        let mut page1 = NPCPage::new("页面1".to_string());
        page1.add_line("第一页内容".to_string());
        dialog.set_page(page1);
        
        assert!(dialog.get_page().is_some());
        assert!(!dialog.can_go_back());
        
        let mut page2 = NPCPage::new("页面2".to_string());
        page2.add_line("第二页内容".to_string());
        dialog.set_page(page2);
        
        assert!(dialog.can_go_back());
        
        let success = dialog.go_back();
        assert!(success);
        assert_eq!(dialog.get_page().unwrap().title, "页面1");
    }

    #[test]
    fn test_npc_dialog_option_selection() {
        let mut dialog = NPCDialog::new();
        dialog.open(1001, "商人".to_string(), 42, NPCDialogType::Normal);
        
        let mut page = NPCPage::new("商店".to_string());
        page.add_option(NPCOption::new("购买".to_string(), "@buy".to_string()).with_key(1));
        page.add_option(NPCOption::new("出售".to_string(), "@sell".to_string()).with_key(2));
        dialog.set_page(page);
        
        // 按索引选择
        let action = dialog.select_option(0);
        assert_eq!(action, Some("@buy".to_string()));
        assert_eq!(dialog.get_selected_option(), Some(0));
        
        // 按快捷键选择
        let action = dialog.select_option_by_key(2);
        assert_eq!(action, Some("@sell".to_string()));
        assert_eq!(dialog.get_selected_option(), Some(1));
    }

    #[test]
    fn test_npc_dialog_scrolling() {
        let mut dialog = NPCDialog::new();
        dialog.open(1001, "NPC".to_string(), 42, NPCDialogType::Normal);
        
        let mut page = NPCPage::new("长对话".to_string());
        page.max_visible_lines = 3;
        for i in 1..=10 {
            page.add_line(format!("行{}", i));
        }
        dialog.set_page(page);
        
        dialog.scroll_down();
        assert_eq!(dialog.get_page().unwrap().scroll_position, 1);
        
        dialog.scroll_to_bottom();
        assert_eq!(dialog.get_page().unwrap().scroll_position, 7); // 10 - 3
        
        dialog.scroll_to_top();
        assert_eq!(dialog.get_page().unwrap().scroll_position, 0);
    }
}
