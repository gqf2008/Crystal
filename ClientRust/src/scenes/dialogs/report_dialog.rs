/// ReportDialog - 举报对话框
/// 
/// 用于提交Bug报告或举报玩家
/// 
/// # 功能特性
/// - 选择举报类型（Bug/玩家）
/// - 多行文本输入框
/// - 发送举报

/// 举报类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportType {
    None,          // 未选择
    SubmitBug,     // 提交Bug
    ReportPlayer,  // 举报玩家
}

impl ReportType {
    /// 获取类型文本
    pub fn text(&self) -> &str {
        match self {
            ReportType::None => "Select Report Type.",
            ReportType::SubmitBug => "Submit Bug",
            ReportType::ReportPlayer => "Report Player",
        }
    }
    
    /// 从索引获取类型
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => ReportType::None,
            1 => ReportType::SubmitBug,
            2 => ReportType::ReportPlayer,
            _ => ReportType::None,
        }
    }
    
    /// 获取所有类型
    pub fn all() -> Vec<ReportType> {
        vec![
            ReportType::None,
            ReportType::SubmitBug,
            ReportType::ReportPlayer,
        ]
    }
}

/// 举报对话框
pub struct ReportDialog {
    /// 是否可见
    pub visible: bool,
    
    /// 对话框位置（居中）
    pub position: (i32, i32),
    
    /// 对话框大小 (Index 1633)
    pub size: (i32, i32),
    
    /// 是否可移动
    pub movable: bool,
    
    /// 是否排序
    pub sort: bool,
    
    /// 举报类型
    pub report_type: ReportType,
    
    /// 举报内容
    pub message: String,
    
    /// 类型下拉框是否展开
    pub dropdown_expanded: bool,
    
    /// 文本框光标位置
    pub cursor_position: usize,
    
    /// 是否正在编辑
    pub editing: bool,
}

impl ReportDialog {
    /// 创建新的举报对话框
    pub fn new(screen_width: i32, screen_height: i32) -> Self {
        let size = (360, 260);
        let position = ((screen_width - size.0) / 2, (screen_height - size.1) / 2);
        
        Self {
            visible: false,
            position,
            size,
            movable: true,
            sort: true,
            report_type: ReportType::None,
            message: String::new(),
            dropdown_expanded: false,
            cursor_position: 0,
            editing: false,
        }
    }
    
    /// 显示对话框
    pub fn show(&mut self) {
        self.visible = true;
        self.report_type = ReportType::None;
        self.message.clear();
        self.cursor_position = 0;
        self.editing = false;
        self.dropdown_expanded = false;
    }
    
    /// 隐藏对话框
    pub fn hide(&mut self) {
        self.visible = false;
        self.editing = false;
        self.dropdown_expanded = false;
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
    
    /// 设置举报类型
    pub fn set_report_type(&mut self, report_type: ReportType) {
        self.report_type = report_type;
        self.dropdown_expanded = false;
    }
    
    /// 切换下拉框展开状态
    pub fn toggle_dropdown(&mut self) {
        self.dropdown_expanded = !self.dropdown_expanded;
    }
    
    /// 添加文本到消息
    pub fn add_text(&mut self, text: &str) {
        if self.editing && self.message.len() + text.len() <= 500 {
            self.message.insert_str(self.cursor_position, text);
            self.cursor_position += text.len();
        }
    }
    
    /// 删除字符（Backspace）
    pub fn delete_char(&mut self) {
        if self.editing && self.cursor_position > 0 {
            self.message.remove(self.cursor_position - 1);
            self.cursor_position -= 1;
        }
    }
    
    /// 清空消息
    pub fn clear_message(&mut self) {
        self.message.clear();
        self.cursor_position = 0;
    }
    
    /// 移动光标
    pub fn move_cursor(&mut self, offset: i32) {
        if offset > 0 {
            self.cursor_position = (self.cursor_position + offset as usize).min(self.message.len());
        } else if offset < 0 {
            self.cursor_position = self.cursor_position.saturating_sub((-offset) as usize);
        }
    }
    
    /// 检查是否可以发送
    pub fn can_send(&self) -> bool {
        self.report_type != ReportType::None && !self.message.trim().is_empty()
    }
    
    /// 发送举报
    /// 
    /// # Returns
    /// (举报类型, 举报内容)
    pub fn send_report(&mut self) -> Option<(ReportType, String)> {
        if !self.can_send() {
            return None;
        }
        
        let report_type = self.report_type;
        let message = self.message.clone();
        
        // 清空并关闭
        self.hide();
        
        Some((report_type, message))
    }
    
    /// 鼠标点击事件
    pub fn on_mouse_click(&mut self, x: i32, y: i32) -> Option<ReportAction> {
        if !self.visible {
            return None;
        }
        
        // 关闭按钮 (336, 3)
        let close_x = self.position.0 + 336;
        let close_y = self.position.1 + 3;
        if x >= close_x && x < close_x + 20 && y >= close_y && y < close_y + 20 {
            self.hide();
            return Some(ReportAction::Close);
        }
        
        // 类型下拉框 (12, 35, 170x14)
        let dropdown_x = self.position.0 + 12;
        let dropdown_y = self.position.1 + 35;
        if x >= dropdown_x && x < dropdown_x + 170 && y >= dropdown_y && y < dropdown_y + 14 {
            self.toggle_dropdown();
            return Some(ReportAction::ToggleDropdown);
        }
        
        // 如果下拉框展开，检查选项点击
        if self.dropdown_expanded {
            let option_y = dropdown_y + 14;
            for (i, report_type) in ReportType::all().iter().enumerate() {
                let item_y = option_y + (i * 14) as i32;
                if x >= dropdown_x && x < dropdown_x + 170 && 
                   y >= item_y && y < item_y + 14 {
                    self.set_report_type(*report_type);
                    return Some(ReportAction::SelectType(*report_type));
                }
            }
        }
        
        // 文本框 (12, 57, 330x150)
        let text_x = self.position.0 + 12;
        let text_y = self.position.1 + 57;
        if x >= text_x && x < text_x + 330 && y >= text_y && y < text_y + 150 {
            self.editing = true;
            return Some(ReportAction::StartEditing);
        } else {
            self.editing = false;
        }
        
        // 发送按钮 (260, 219)
        let send_x = self.position.0 + 260;
        let send_y = self.position.1 + 219;
        if x >= send_x && x < send_x + 60 && y >= send_y && y < send_y + 30 {
            if self.can_send() {
                return Some(ReportAction::Send);
            }
        }
        
        None
    }
    
    /// 获取文本框显示文本（处理多行）
    pub fn get_display_text(&self) -> Vec<String> {
        let max_width = 40; // 每行最多40字符
        let mut lines = Vec::new();
        let mut current_line = String::new();
        
        for ch in self.message.chars() {
            if ch == '\n' {
                lines.push(current_line.clone());
                current_line.clear();
            } else {
                current_line.push(ch);
                if current_line.len() >= max_width {
                    lines.push(current_line.clone());
                    current_line.clear();
                }
            }
        }
        
        if !current_line.is_empty() {
            lines.push(current_line);
        }
        
        lines
    }
}

/// 举报对话框动作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportAction {
    Close,
    ToggleDropdown,
    SelectType(ReportType),
    StartEditing,
    Send,
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_report_dialog_creation() {
        let dialog = ReportDialog::new(1024, 768);
        
        assert!(!dialog.visible);
        assert_eq!(dialog.report_type, ReportType::None);
        assert!(dialog.message.is_empty());
    }
    
    #[test]
    fn test_report_type() {
        assert_eq!(ReportType::None.text(), "Select Report Type.");
        assert_eq!(ReportType::SubmitBug.text(), "Submit Bug");
        assert_eq!(ReportType::ReportPlayer.text(), "Report Player");
        
        assert_eq!(ReportType::from_index(0), ReportType::None);
        assert_eq!(ReportType::from_index(1), ReportType::SubmitBug);
        assert_eq!(ReportType::from_index(2), ReportType::ReportPlayer);
    }
    
    #[test]
    fn test_show_hide() {
        let mut dialog = ReportDialog::new(1024, 768);
        
        dialog.show();
        assert!(dialog.is_visible());
        assert_eq!(dialog.report_type, ReportType::None);
        
        dialog.hide();
        assert!(!dialog.is_visible());
    }
    
    #[test]
    fn test_set_report_type() {
        let mut dialog = ReportDialog::new(1024, 768);
        
        dialog.set_report_type(ReportType::SubmitBug);
        assert_eq!(dialog.report_type, ReportType::SubmitBug);
        
        dialog.set_report_type(ReportType::ReportPlayer);
        assert_eq!(dialog.report_type, ReportType::ReportPlayer);
    }
    
    #[test]
    fn test_message_editing() {
        let mut dialog = ReportDialog::new(1024, 768);
        dialog.editing = true;
        
        dialog.add_text("Hello");
        assert_eq!(dialog.message, "Hello");
        assert_eq!(dialog.cursor_position, 5);
        
        dialog.add_text(" World");
        assert_eq!(dialog.message, "Hello World");
        assert_eq!(dialog.cursor_position, 11);
        
        dialog.delete_char();
        assert_eq!(dialog.message, "Hello Worl");
        assert_eq!(dialog.cursor_position, 10);
    }
    
    #[test]
    fn test_cursor_movement() {
        let mut dialog = ReportDialog::new(1024, 768);
        dialog.editing = true;
        dialog.add_text("Test");
        
        assert_eq!(dialog.cursor_position, 4);
        
        dialog.move_cursor(-2);
        assert_eq!(dialog.cursor_position, 2);
        
        dialog.move_cursor(1);
        assert_eq!(dialog.cursor_position, 3);
        
        dialog.move_cursor(10); // 超出范围
        assert_eq!(dialog.cursor_position, 4); // 应该限制在最大值
    }
    
    #[test]
    fn test_can_send() {
        let mut dialog = ReportDialog::new(1024, 768);
        
        // 未选择类型，不能发送
        assert!(!dialog.can_send());
        
        dialog.set_report_type(ReportType::SubmitBug);
        // 选择了类型但没有内容，不能发送
        assert!(!dialog.can_send());
        
        dialog.editing = true;
        dialog.add_text("Bug report content");
        // 选择了类型且有内容，可以发送
        assert!(dialog.can_send());
    }
    
    #[test]
    fn test_send_report() {
        let mut dialog = ReportDialog::new(1024, 768);
        dialog.show();
        dialog.set_report_type(ReportType::ReportPlayer);
        dialog.editing = true;
        dialog.add_text("Cheating player");
        
        let result = dialog.send_report();
        assert!(result.is_some());
        
        let (report_type, message) = result.unwrap();
        assert_eq!(report_type, ReportType::ReportPlayer);
        assert_eq!(message, "Cheating player");
        
        // 发送后应该关闭
        assert!(!dialog.visible);
    }
    
    #[test]
    fn test_clear_message() {
        let mut dialog = ReportDialog::new(1024, 768);
        dialog.editing = true;
        dialog.add_text("Test message");
        
        assert!(!dialog.message.is_empty());
        
        dialog.clear_message();
        assert!(dialog.message.is_empty());
        assert_eq!(dialog.cursor_position, 0);
    }
    
    #[test]
    fn test_dropdown_toggle() {
        let mut dialog = ReportDialog::new(1024, 768);
        
        assert!(!dialog.dropdown_expanded);
        
        dialog.toggle_dropdown();
        assert!(dialog.dropdown_expanded);
        
        dialog.toggle_dropdown();
        assert!(!dialog.dropdown_expanded);
    }
    
    #[test]
    fn test_display_text_wrapping() {
        let mut dialog = ReportDialog::new(1024, 768);
        dialog.editing = true;
        
        // 添加长文本
        let long_text = "a".repeat(100);
        dialog.add_text(&long_text);
        
        let lines = dialog.get_display_text();
        assert!(lines.len() > 1); // 应该被分成多行
    }
}
