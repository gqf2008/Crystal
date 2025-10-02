// ChatDialog - Chat window
// Mirrors Client/MirScenes/Dialogs/ChatDialog.cs

use super::Dialog;
use std::collections::VecDeque;

/// Chat message type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatType {
    Normal,      // 普通聊天
    Whisper,     // 私聊
    Shout,       // 喊话
    System,      // 系统消息
    Group,       // 组队
    Guild,       // 公会
    Announcement, // 公告
}

/// Chat message
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub sender: String,
    pub text: String,
    pub chat_type: ChatType,
    pub color: (u8, u8, u8), // RGB
    pub timestamp: i64,
}

/// Chat dialog
#[derive(Debug)]
pub struct ChatDialog {
    pub visible: bool,
    pub messages: VecDeque<ChatMessage>,
    pub max_messages: usize,
    pub input_text: String,
    pub chat_filter: Vec<ChatType>, // Show only these types
}

impl ChatDialog {
    pub fn new() -> Self {
        Self {
            visible: true,
            messages: VecDeque::new(),
            max_messages: 100,
            input_text: String::new(),
            chat_filter: vec![
                ChatType::Normal,
                ChatType::Whisper,
                ChatType::Shout,
                ChatType::System,
                ChatType::Group,
                ChatType::Guild,
                ChatType::Announcement,
            ],
        }
    }
    
    /// Add message to chat
    pub fn add_message(&mut self, sender: String, text: String, chat_type: ChatType) {
        let color = Self::get_chat_color(chat_type);
        let message = ChatMessage {
            sender,
            text,
            chat_type,
            color,
            timestamp: get_current_time(),
        };
        
        self.messages.push_back(message);
        
        // Limit message count
        while self.messages.len() > self.max_messages {
            self.messages.pop_front();
        }
    }
    
    /// Get color for chat type
    fn get_chat_color(chat_type: ChatType) -> (u8, u8, u8) {
        match chat_type {
            ChatType::Normal => (255, 255, 255),      // White
            ChatType::Whisper => (255, 100, 255),     // Pink
            ChatType::Shout => (255, 255, 0),         // Yellow
            ChatType::System => (255, 100, 100),      // Red
            ChatType::Group => (100, 255, 100),       // Green
            ChatType::Guild => (100, 200, 255),       // Cyan
            ChatType::Announcement => (255, 200, 0),  // Orange
        }
    }
    
    /// Send chat message
    pub fn send_message(&mut self) {
        if self.input_text.is_empty() {
            return;
        }
        
        // TODO: Parse command (starts with /)
        // TODO: Parse whisper (@player)
        // TODO: Send to server
        
        println!("Sending chat: {}", self.input_text);
        
        self.input_text.clear();
    }
    
    /// Toggle chat filter
    pub fn toggle_filter(&mut self, chat_type: ChatType) {
        if self.chat_filter.contains(&chat_type) {
            self.chat_filter.retain(|&t| t != chat_type);
        } else {
            self.chat_filter.push(chat_type);
        }
    }
    
    /// Check if chat type is filtered
    pub fn is_filtered(&self, chat_type: ChatType) -> bool {
        self.chat_filter.contains(&chat_type)
    }
    
    /// Get visible messages (filtered)
    pub fn get_visible_messages(&self) -> Vec<&ChatMessage> {
        self.messages
            .iter()
            .filter(|msg| self.is_filtered(msg.chat_type))
            .collect()
    }
}

impl Default for ChatDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl Dialog for ChatDialog {
    fn show(&mut self) {
        self.visible = true;
    }
    
    fn hide(&mut self) {
        self.visible = false;
    }
    
    fn update(&mut self, _delta_time: f32) {
        // TODO: Scroll chat
        // TODO: Update input cursor
    }
    
    fn draw(&self) {
        if !self.visible {
            return;
        }
        
        // TODO: Draw chat background
        // TODO: Draw messages
        // TODO: Draw input box
        // TODO: Draw filter buttons
    }
    
    fn is_visible(&self) -> bool {
        self.visible
    }
}

/// Get current time in milliseconds
fn get_current_time() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_dialog_creation() {
        let dialog = ChatDialog::new();
        assert!(dialog.visible);
        assert_eq!(dialog.messages.len(), 0);
    }

    #[test]
    fn test_add_message() {
        let mut dialog = ChatDialog::new();
        
        dialog.add_message(
            "Player1".to_string(),
            "Hello!".to_string(),
            ChatType::Normal,
        );
        
        assert_eq!(dialog.messages.len(), 1);
        assert_eq!(dialog.messages[0].sender, "Player1");
        assert_eq!(dialog.messages[0].text, "Hello!");
    }

    #[test]
    fn test_message_limit() {
        let mut dialog = ChatDialog::new();
        dialog.max_messages = 3;
        
        dialog.add_message("P1".to_string(), "Msg1".to_string(), ChatType::Normal);
        dialog.add_message("P2".to_string(), "Msg2".to_string(), ChatType::Normal);
        dialog.add_message("P3".to_string(), "Msg3".to_string(), ChatType::Normal);
        dialog.add_message("P4".to_string(), "Msg4".to_string(), ChatType::Normal);
        
        assert_eq!(dialog.messages.len(), 3);
        assert_eq!(dialog.messages[0].text, "Msg2"); // First message dropped
    }

    #[test]
    fn test_chat_filter() {
        let mut dialog = ChatDialog::new();
        
        assert!(dialog.is_filtered(ChatType::Normal));
        
        dialog.toggle_filter(ChatType::Normal);
        assert!(!dialog.is_filtered(ChatType::Normal));
        
        dialog.toggle_filter(ChatType::Normal);
        assert!(dialog.is_filtered(ChatType::Normal));
    }
}
