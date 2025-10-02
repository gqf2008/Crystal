/// NoticeDialog - 系统公告对话框
/// 
/// 显示游戏公告、系统消息、活动通知等内容
/// 
/// # 功能特性
/// - 支持长文本内容滚动
/// - 支持链接和颜色标记
/// - 鼠标滚轮滚动
/// - 滚动条拖动
/// - 最多显示19行文本

/// 公告数据
#[derive(Debug, Clone)]
pub struct Notice {
    /// 公告标题
    pub title: String,
    
    /// 公告内容
    pub message: String,
    
    /// 发布时间
    pub date: String,
}

impl Notice {
    pub fn new() -> Self {
        Self {
            title: String::new(),
            message: String::new(),
            date: String::new(),
        }
    }
}

/// 系统公告对话框
pub struct NoticeDialog {
    /// 是否可见
    pub visible: bool,
    
    /// 对话框位置
    pub position: (i32, i32),
    
    /// 对话框大小 (Index 961)
    pub size: (i32, i32),
    
    /// 是否可移动
    pub movable: bool,
    
    /// 是否排序
    pub sort: bool,
    
    /// 当前公告
    pub notice: Notice,
    
    /// 当前文本行
    pub current_lines: Vec<String>,
    
    /// 滚动索引（当前显示第几行开始）
    pub scroll_index: usize,
    
    /// 最大显示行数
    pub maximum_lines: usize,
    
    /// 滚动条位置
    pub scroll_bar_position: (i32, i32),
}

impl NoticeDialog {
    /// 创建新的公告对话框
    pub fn new(screen_width: i32, screen_height: i32) -> Self {
        let size = (320, 480); // 估算大小
        let position = ((screen_width - size.0) / 2, (screen_height - size.1) / 3);
        
        Self {
            visible: false,
            position,
            size,
            movable: true,
            sort: true,
            notice: Notice::new(),
            current_lines: Vec::new(),
            scroll_index: 0,
            maximum_lines: 19,
            scroll_bar_position: (293, 46),
        }
    }
    
    /// 显示对话框
    pub fn show(&mut self) {
        self.visible = true;
    }
    
    /// 隐藏对话框
    pub fn hide(&mut self) {
        self.visible = false;
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
    
    /// 更新公告内容
    pub fn update_notice(&mut self, notice: Notice) {
        self.notice = notice;
        self.scroll_index = 0;
        
        // 解析文本行
        self.current_lines.clear();
        for line in self.notice.message.lines() {
            self.current_lines.push(line.to_string());
        }
        
        self.update_scroll_bar();
    }
    
    /// 获取当前显示的文本行
    pub fn get_visible_lines(&self) -> Vec<String> {
        let end = (self.scroll_index + self.maximum_lines).min(self.current_lines.len());
        self.current_lines[self.scroll_index..end].to_vec()
    }
    
    /// 向上滚动
    pub fn scroll_up(&mut self, lines: usize) {
        if self.scroll_index == 0 {
            return;
        }
        
        self.scroll_index = self.scroll_index.saturating_sub(lines);
        self.update_scroll_bar();
    }
    
    /// 向下滚动
    pub fn scroll_down(&mut self, lines: usize) {
        let max_scroll = self.current_lines.len().saturating_sub(self.maximum_lines);
        if self.scroll_index >= max_scroll {
            return;
        }
        
        self.scroll_index = (self.scroll_index + lines).min(max_scroll);
        self.update_scroll_bar();
    }
    
    /// 鼠标滚轮滚动
    pub fn on_mouse_wheel(&mut self, delta: i32) {
        // delta > 0 向上滚动, delta < 0 向下滚动
        if delta > 0 {
            self.scroll_up(1);
        } else if delta < 0 {
            self.scroll_down(1);
        }
    }
    
    /// 更新滚动条位置
    fn update_scroll_bar(&mut self) {
        if self.current_lines.len() <= self.maximum_lines {
            return;
        }
        
        let max_scroll = self.current_lines.len() - self.maximum_lines;
        let interval = 400 / max_scroll;
        
        let x = 293;
        let y = 46 + (self.scroll_index * interval).min(399);
        
        self.scroll_bar_position = (x, y);
    }
    
    /// 滚动条拖动
    pub fn on_scroll_bar_drag(&mut self, mouse_y: i32) {
        let bar_min_y = 46;
        let bar_max_y = 399;
        
        let y = mouse_y.max(bar_min_y).min(bar_max_y);
        
        let location = y - bar_min_y;
        let max_scroll = self.current_lines.len().saturating_sub(self.maximum_lines);
        
        if max_scroll == 0 {
            return;
        }
        
        let interval = (bar_max_y - bar_min_y) / max_scroll as i32;
        let index = (location / interval) as usize;
        
        self.scroll_index = index.min(max_scroll);
        self.scroll_bar_position = (293, y);
    }
    
    /// 鼠标点击事件
    pub fn on_mouse_click(&mut self, x: i32, y: i32) -> bool {
        if !self.visible {
            return false;
        }
        
        // 关闭按钮 (289, 3, 24x24)
        let close_x = self.position.0 + 289;
        let close_y = self.position.1 + 3;
        if x >= close_x && x < close_x + 24 && y >= close_y && y < close_y + 24 {
            self.hide();
            return true;
        }
        
        // OK按钮 (120, 436)
        let ok_x = self.position.0 + 120;
        let ok_y = self.position.1 + 436;
        if x >= ok_x && x < ok_x + 60 && y >= ok_y && y < ok_y + 30 {
            self.hide();
            return true;
        }
        
        // 向上按钮 (293, 33, 16x14)
        let up_x = self.position.0 + 293;
        let up_y = self.position.1 + 33;
        if x >= up_x && x < up_x + 16 && y >= up_y && y < up_y + 14 {
            self.scroll_up(1);
            return true;
        }
        
        // 向下按钮 (293, 418, 16x14)
        let down_x = self.position.0 + 293;
        let down_y = self.position.1 + 418;
        if x >= down_x && x < down_x + 16 && y >= down_y && y < down_y + 14 {
            self.scroll_down(1);
            return true;
        }
        
        false
    }
    
    /// 检查是否需要显示滚动条
    pub fn needs_scrollbar(&self) -> bool {
        self.current_lines.len() > self.maximum_lines
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_notice_dialog_creation() {
        let dialog = NoticeDialog::new(1024, 768);
        
        assert!(!dialog.visible);
        assert_eq!(dialog.scroll_index, 0);
        assert_eq!(dialog.maximum_lines, 19);
    }
    
    #[test]
    fn test_show_hide() {
        let mut dialog = NoticeDialog::new(1024, 768);
        
        dialog.show();
        assert!(dialog.is_visible());
        
        dialog.hide();
        assert!(!dialog.is_visible());
    }
    
    #[test]
    fn test_update_notice() {
        let mut dialog = NoticeDialog::new(1024, 768);
        
        let notice = Notice {
            title: "Test Notice".to_string(),
            message: "Line 1\nLine 2\nLine 3".to_string(),
            date: "2025-10-02".to_string(),
        };
        
        dialog.update_notice(notice);
        
        assert_eq!(dialog.current_lines.len(), 3);
        assert_eq!(dialog.current_lines[0], "Line 1");
        assert_eq!(dialog.current_lines[1], "Line 2");
        assert_eq!(dialog.current_lines[2], "Line 3");
    }
    
    #[test]
    fn test_scrolling() {
        let mut dialog = NoticeDialog::new(1024, 768);
        dialog.maximum_lines = 5;
        
        // 创建25行文本
        let mut message = String::new();
        for i in 0..25 {
            message.push_str(&format!("Line {}\n", i + 1));
        }
        
        dialog.update_notice(Notice {
            title: "Test".to_string(),
            message,
            date: "2025-10-02".to_string(),
        });
        
        assert_eq!(dialog.current_lines.len(), 26); // 25行 + 1个空行
        assert_eq!(dialog.scroll_index, 0);
        
        // 向下滚动
        dialog.scroll_down(5);
        assert_eq!(dialog.scroll_index, 5);
        
        // 向上滚动
        dialog.scroll_up(2);
        assert_eq!(dialog.scroll_index, 3);
        
        // 滚动到边界
        dialog.scroll_up(10);
        assert_eq!(dialog.scroll_index, 0);
        
        dialog.scroll_down(100);
        assert_eq!(dialog.scroll_index, 21); // 26 - 5 = 21
    }
    
    #[test]
    fn test_visible_lines() {
        let mut dialog = NoticeDialog::new(1024, 768);
        dialog.maximum_lines = 3;
        
        dialog.update_notice(Notice {
            title: "Test".to_string(),
            message: "A\nB\nC\nD\nE".to_string(),
            date: "2025-10-02".to_string(),
        });
        
        // 初始显示前3行
        let visible = dialog.get_visible_lines();
        assert_eq!(visible.len(), 3);
        assert_eq!(visible[0], "A");
        assert_eq!(visible[2], "C");
        
        // 滚动后显示不同行
        dialog.scroll_down(2);
        let visible = dialog.get_visible_lines();
        assert_eq!(visible[0], "C");
        assert_eq!(visible[2], "E");
    }
    
    #[test]
    fn test_mouse_wheel() {
        let mut dialog = NoticeDialog::new(1024, 768);
        dialog.maximum_lines = 5;
        
        let mut message = String::new();
        for i in 0..25 {
            message.push_str(&format!("Line {}\n", i + 1));
        }
        dialog.update_notice(Notice {
            title: "Test".to_string(),
            message,
            date: "2025-10-02".to_string(),
        });
        
        // 向下滚轮（delta < 0）
        dialog.on_mouse_wheel(-1);
        assert_eq!(dialog.scroll_index, 1);
        
        // 向上滚轮（delta > 0）
        dialog.on_mouse_wheel(1);
        assert_eq!(dialog.scroll_index, 0);
    }
    
    #[test]
    fn test_needs_scrollbar() {
        let mut dialog = NoticeDialog::new(1024, 768);
        dialog.maximum_lines = 19;
        
        // 少于19行，不需要滚动条
        dialog.update_notice(Notice {
            title: "Short".to_string(),
            message: "A\nB\nC".to_string(),
            date: "2025-10-02".to_string(),
        });
        assert!(!dialog.needs_scrollbar());
        
        // 超过19行，需要滚动条
        let mut long_message = String::new();
        for i in 0..30 {
            long_message.push_str(&format!("Line {}\n", i + 1));
        }
        dialog.update_notice(Notice {
            title: "Long".to_string(),
            message: long_message,
            date: "2025-10-02".to_string(),
        });
        assert!(dialog.needs_scrollbar());
    }
    
    #[test]
    fn test_scroll_bar_position() {
        let mut dialog = NoticeDialog::new(1024, 768);
        dialog.maximum_lines = 5;
        
        let mut message = String::new();
        for i in 0..25 {
            message.push_str(&format!("Line {}\n", i + 1));
        }
        dialog.update_notice(Notice {
            title: "Test".to_string(),
            message,
            date: "2025-10-02".to_string(),
        });
        
        // 初始位置
        assert_eq!(dialog.scroll_bar_position, (293, 46));
        
        // 滚动后位置变化
        dialog.scroll_down(10);
        assert!(dialog.scroll_bar_position.1 > 46);
    }
}
