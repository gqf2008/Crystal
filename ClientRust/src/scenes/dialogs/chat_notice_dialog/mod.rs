// ChatNoticeDialog - 聊天通知对话框
// Mirrors Client/MirScenes/Dialogs/ChatNoticeDialog.cs (70 lines)

use std::time::{Duration, Instant};

/// 聊天通知类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatNoticeType {
    Normal = 0,
    Special = 1,
}

/// 聊天通知对话框 - 屏幕顶部的通知消息
#[derive(Debug, Clone)]
pub struct ChatNoticeDialog {
    /// 是否可见
    pub visible: bool,

    /// 显示位置
    pub location: (i32, i32),

    /// 通知类型
    pub notice_type: ChatNoticeType,

    /// 显示文本（第一行，小字）
    pub text1: String,

    /// 显示文本（第二行，大字）
    pub text2: String,

    /// 显示时长（毫秒）
    view_time: u64,

    /// 开始显示的时间
    start_time: Option<Instant>,

    /// 不透明度
    pub opacity: f32,
}

impl ChatNoticeDialog {
    /// 创建新的聊天通知对话框
    pub fn new(screen_width: i32, screen_height: i32) -> Self {
        // C# 位置: ScreenWidth / 2 - 330, ScreenHeight / 6 - 20
        // Size.Width = 660, Size.Height = 40
        Self {
            visible: false,
            location: (screen_width / 2 - 330, screen_height / 6 - 20),
            notice_type: ChatNoticeType::Normal,
            text1: String::new(),
            text2: String::new(),
            view_time: 10000, // 10 seconds
            start_time: None,
            opacity: 0.7,
        }
    }

    /// 显示通知
    pub fn show_notice(&mut self, text: String, notice_type: ChatNoticeType) {
        self.notice_type = notice_type;

        // 根据类型设置文本位置
        match notice_type {
            ChatNoticeType::Normal => {
                self.text1 = String::new();
                self.text2 = text;
            }
            ChatNoticeType::Special => {
                self.text1 = text.clone();
                self.text2 = text;
            }
        }

        self.visible = true;
        self.start_time = Some(Instant::now());
    }

    /// 显示普通通知
    pub fn show(&mut self, text: String) {
        self.show_notice(text, ChatNoticeType::Normal);
    }

    /// 显示特殊通知
    pub fn show_special(&mut self, text: String) {
        self.show_notice(text, ChatNoticeType::Special);
    }

    /// 隐藏通知
    pub fn hide(&mut self) {
        self.visible = false;
        self.text1.clear();
        self.text2.clear();
        self.start_time = None;
    }

    /// 更新状态（检查是否超时）
    pub fn update(&mut self) {
        if !self.visible {
            return;
        }

        if let Some(start) = self.start_time {
            let elapsed = start.elapsed();
            if elapsed > Duration::from_millis(self.view_time) {
                self.hide();
            }
        }
    }

    /// 设置显示时长（毫秒）
    pub fn set_view_time(&mut self, milliseconds: u64) {
        self.view_time = milliseconds;
    }

    /// 获取剩余显示时间（毫秒）
    pub fn remaining_time(&self) -> Option<u64> {
        if !self.visible {
            return None;
        }

        if let Some(start) = self.start_time {
            let elapsed = start.elapsed().as_millis() as u64;
            if elapsed < self.view_time {
                return Some(self.view_time - elapsed);
            }
        }

        None
    }

    /// 获取图像索引（用于渲染）
    pub fn get_image_index(&self) -> i32 {
        match self.notice_type {
            ChatNoticeType::Normal => 1361,
            ChatNoticeType::Special => 1363,
        }
    }
}

impl Default for ChatNoticeDialog {
    fn default() -> Self {
        Self::new(800, 600)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_chat_notice_creation() {
        let dialog = ChatNoticeDialog::new(800, 600);
        assert!(!dialog.visible);
        assert_eq!(dialog.view_time, 10000);
        assert_eq!(dialog.opacity, 0.7);
    }

    #[test]
    fn test_show_notice() {
        let mut dialog = ChatNoticeDialog::new(800, 600);

        dialog.show("Test message".to_string());

        assert!(dialog.visible);
        assert_eq!(dialog.text2, "Test message");
        assert!(dialog.start_time.is_some());
    }

    #[test]
    fn test_hide_notice() {
        let mut dialog = ChatNoticeDialog::new(800, 600);

        dialog.show("Test".to_string());
        dialog.hide();

        assert!(!dialog.visible);
        assert!(dialog.text1.is_empty());
        assert!(dialog.text2.is_empty());
        assert!(dialog.start_time.is_none());
    }

    #[test]
    fn test_timeout() {
        let mut dialog = ChatNoticeDialog::new(800, 600);
        dialog.set_view_time(100); // 100ms for testing

        dialog.show("Test".to_string());
        assert!(dialog.visible);

        // Wait longer than view time
        thread::sleep(Duration::from_millis(150));
        dialog.update();

        assert!(!dialog.visible);
    }

    #[test]
    fn test_remaining_time() {
        let mut dialog = ChatNoticeDialog::new(800, 600);
        dialog.set_view_time(1000);

        dialog.show("Test".to_string());

        let remaining = dialog.remaining_time();
        assert!(remaining.is_some());
        assert!(remaining.unwrap() <= 1000);
    }

    #[test]
    fn test_notice_types() {
        let mut dialog = ChatNoticeDialog::new(800, 600);

        dialog.show_notice("Normal".to_string(), ChatNoticeType::Normal);
        assert_eq!(dialog.get_image_index(), 1361);

        dialog.show_notice("Special".to_string(), ChatNoticeType::Special);
        assert_eq!(dialog.get_image_index(), 1363);
    }

    #[test]
    fn test_special_notice_text() {
        let mut dialog = ChatNoticeDialog::new(800, 600);

        dialog.show_special("Special message".to_string());

        // Special notices show on both lines
        assert_eq!(dialog.text1, "Special message");
        assert_eq!(dialog.text2, "Special message");
    }
}