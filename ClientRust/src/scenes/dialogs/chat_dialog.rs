// ChatDialog - Chat window
// Mirrors Client/MirScenes/Dialogs/MainDialogs.cs ChatDialog class

use super::Dialog;
use std::collections::VecDeque;
use mir2_shared::ChatType;

/// Chat message
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub sender: String,
    pub text: String,
    pub chat_type: ChatType,
    pub color: (u8, u8, u8), // RGB
    pub timestamp: i64,
}

/// Chat history item
#[derive(Debug, Clone)]
pub struct ChatHistory {
    pub text: String,
    pub back_colour: u32, // ARGB
    pub fore_colour: u32, // ARGB
    pub chat_type: ChatType,
}

/// Chat item link
#[derive(Debug, Clone)]
pub struct ChatItem {
    pub item_name: String,
    pub item_id: u64,
}

/// Chat dialog - handles all chat functionality
#[derive(Debug)]
pub struct ChatDialog {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,

    /// Full unfiltered chat history
    pub full_history: Vec<ChatHistory>,
    /// Filtered history for display
    pub history: Vec<ChatHistory>,
    /// Current chat lines (labels)
    pub chat_lines: Vec<String>, // Simplified for now

    /// Linked items in current message
    pub linked_items: Vec<ChatItem>,
    /// Linked item buttons
    pub linked_item_buttons: Vec<String>, // Simplified

    /// Scroll controls
    pub start_index: usize,
    pub line_count: usize,
    pub window_size: usize,

    /// Chat input
    pub chat_prefix: String,
    pub last_pm: String,
    pub input_text: String,

    /// Transparency
    pub transparent: bool,

    /// Message storage (for compatibility)
    pub messages: VecDeque<ChatMessage>,
    pub max_messages: usize,
    pub chat_filter: Vec<ChatType>,
}

impl ChatDialog {
    pub fn new() -> Self {
        Self {
            visible: true,
            x: 0,
            y: 500,
            width: 400,
            height: 200,
            full_history: Vec::new(),
            history: Vec::new(),
            chat_lines: Vec::new(),
            linked_items: Vec::new(),
            linked_item_buttons: Vec::new(),
            start_index: 0,
            line_count: 4,
            window_size: 0,
            chat_prefix: String::new(),
            last_pm: String::new(),
            input_text: String::new(),
            transparent: false,
            messages: VecDeque::new(),
            max_messages: 100,
            chat_filter: vec![
                ChatType::Normal,
                ChatType::WhisperIn,
                ChatType::WhisperOut,
                ChatType::Shout,
                ChatType::System,
                ChatType::Group,
                ChatType::Guild,
                ChatType::Announcement,
            ],
        }
    }
    
    /// Add message to chat (legacy method for compatibility)
    pub fn add_message(&mut self, sender: String, text: String, chat_type: ChatType) {
        let color = Self::get_chat_color(chat_type);
        let message = ChatMessage {
            sender,
            text: text.clone(),
            chat_type,
            color,
            timestamp: get_current_time(),
        };

        self.messages.push_back(message);

        // Limit message count
        while self.messages.len() > self.max_messages {
            self.messages.pop_front();
        }

        // Also add to full history
        self.receive_chat(&text, chat_type);
    }

    /// Receive chat message (main method)
    pub fn receive_chat(&mut self, text: &str, chat_type: ChatType) {
        let (fore_colour, back_colour) = self.get_chat_colors(chat_type);

        // Split text into lines
        let chat_lines = self.split_chat_text(text);

        // Adjust scroll position if at bottom
        if self.start_index == self.history.len().saturating_sub(self.line_count) {
            self.start_index += chat_lines.len();
        }

        // Add to full history
        for line in chat_lines {
            self.full_history.push(ChatHistory {
                text: line,
                back_colour,
                fore_colour,
                chat_type,
            });
        }

        self.update();
    }

    /// Get colors for chat type (returns ARGB)
    fn get_chat_colors(&self, chat_type: ChatType) -> (u32, u32) {
        match chat_type {
            ChatType::Hint => (0xFF006400, 0xFFFFFFFF), // DarkGreen on White
            ChatType::Announcement => (0xFFFFFFFF, 0xFF0000FF), // White on Blue
            ChatType::LineMessage => (0xFFFFFFFF, 0xFF0000FF), // White on Blue
            ChatType::Shout => (0xFF000000, 0xFFFFFF00), // Black on Yellow
            ChatType::Shout2 => (0xFFFFFFFF, 0xFF008000), // White on Green
            ChatType::Shout3 => (0xFFFFFFFF, 0xFF800080), // White on Purple
            ChatType::System => (0xFFFFFFFF, 0xFFFF0000), // White on Red
            ChatType::System2 => (0xFFFFFFFF, 0xFF8B0000), // White on DarkRed
            ChatType::Group => (0xFFA52A2A, 0xFFFFFFFF), // Brown on White
            ChatType::WhisperOut => (0xFF6495ED, 0xFFFFFFFF), // CornflowerBlue on White
            ChatType::WhisperIn => (0xFF00008B, 0xFFFFFFFF), // DarkBlue on White
            ChatType::Guild => (0xFF008000, 0xFFFFFFFF), // Green on White
            ChatType::LevelUp => (0xFF0000FF, 0xFFE1B9FA), // Blue on Light Purple
            ChatType::Relationship => (0xFFFF69B4, 0x00FFFFFF), // HotPink on Transparent
            ChatType::Mentor => (0xFF800080, 0xFFFFFFFF), // Purple on White
            _ => (0xFF000000, 0xFFFFFFFF), // Black on White (Normal)
        }
    }

    /// Get RGB color for chat type (legacy method)
    fn get_chat_color(chat_type: ChatType) -> (u8, u8, u8) {
        match chat_type {
            ChatType::Normal => (255, 255, 255),      // White
            ChatType::WhisperIn | ChatType::WhisperOut => (255, 100, 255), // Pink
            ChatType::Shout | ChatType::Shout2 | ChatType::Shout3 => (255, 255, 0), // Yellow
            ChatType::System | ChatType::System2 => (255, 100, 100), // Red
            ChatType::Hint => (255, 200, 100),        // Light Orange
            ChatType::Announcement => (255, 200, 0),  // Orange
            ChatType::Group => (100, 255, 100),       // Green
            ChatType::Guild => (100, 200, 255),       // Cyan
            ChatType::Trainer => (200, 150, 255),     // Purple
            ChatType::LevelUp => (255, 215, 0),       // Gold
            ChatType::Relationship => (255, 105, 180), // Hot Pink
            ChatType::Mentor => (147, 112, 219),      // Medium Purple
            ChatType::LineMessage => (150, 150, 150), // Gray
        }
    }

    /// Split chat text into lines based on width
    fn split_chat_text(&self, text: &str) -> Vec<String> {
        let chat_width = 614; // Default width, TODO: Make configurable
        let mut lines = Vec::new();
        let mut index = 0;

        while index < text.len() {
            let mut line_end = text.len();
            let mut found_break = false;

            // Find where to break the line
            for i in (index + 1)..=text.len() {
                let substring = &text[index..i];
                // TODO: Measure text width properly
                if substring.len() > 50 { // Temporary approximation
                    line_end = i - 1;
                    found_break = true;
                    break;
                }
            }

            if found_break || line_end == text.len() {
                lines.push(text[index..line_end].to_string());
                index = line_end;
            } else {
                lines.push(text[index..].to_string());
                break;
            }
        }

        lines
    }

    /// Update chat display
    pub fn update(&mut self) {
        // Clear current history
        self.history.clear();

        // Apply filters and build display history
        for history_item in &self.full_history {
            if !self.should_filter_chat(history_item.chat_type) {
                self.history.push(history_item.clone());
            }
        }

        // Clear existing chat lines
        self.chat_lines.clear();
        self.linked_item_buttons.clear();

        // Adjust start index
        if self.start_index >= self.history.len() {
            self.start_index = self.history.len().saturating_sub(1);
        }
        if self.start_index > self.history.len() {
            self.start_index = 0;
        }

        // Create chat lines
        self.create_chat_lines();
    }

    /// Check if chat type should be filtered
    fn should_filter_chat(&self, chat_type: ChatType) -> bool {
        // TODO: Use actual settings instead of hardcoded filters
        match chat_type {
            ChatType::Normal | ChatType::LineMessage => false, // Settings::filter_normal_chat(),
            ChatType::WhisperIn | ChatType::WhisperOut => false, // Settings::filter_whisper_chat(),
            ChatType::Shout | ChatType::Shout2 | ChatType::Shout3 => false, // Settings::filter_shout_chat(),
            ChatType::System | ChatType::System2 => false, // Settings::filter_system_chat(),
            ChatType::Group => false, // Settings::filter_group_chat(),
            ChatType::Guild => false, // Settings::filter_guild_chat(),
            _ => false,
        }
    }

    /// Create chat line labels
    fn create_chat_lines(&mut self) {
        let mut y = 1;

        for i in self.start_index..self.history.len().min(self.start_index + self.line_count) {
            if i >= self.history.len() {
                break;
            }

            let history_item = &self.history[i];
            self.chat_lines.push(history_item.text.clone());

            y += 14; // Line height
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
    
    fn name(&self) -> &str {
        "ChatDialog"
    }
    
    fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.width &&
        y >= self.y && y < self.y + self.height
    }
    
    fn position(&self) -> (i32, i32) {
        (self.x, self.y)
    }
    
    fn size(&self) -> (i32, i32) {
        (self.width, self.height)
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
        assert_eq!(dialog.line_count, 4);
        assert!(dialog.chat_prefix.is_empty());
        assert!(!dialog.transparent);
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
        assert_eq!(dialog.full_history.len(), 1);
    }

    #[test]
    fn test_receive_chat() {
        let mut dialog = ChatDialog::new();
        let initial_count = dialog.full_history.len();

        dialog.receive_chat("Test message", ChatType::Normal);
        assert_eq!(dialog.full_history.len(), initial_count + 1);
        assert_eq!(dialog.full_history[0].text, "Test message");
    }

    #[test]
    fn test_chat_colors() {
        let dialog = ChatDialog::new();

        let (fore, back) = dialog.get_chat_colors(ChatType::System);
        assert_eq!(fore, 0xFFFFFFFF); // White
        assert_eq!(back, 0xFFFF0000); // Red

        let (fore, back) = dialog.get_chat_colors(ChatType::Normal);
        assert_eq!(fore, 0xFF000000); // Black
        assert_eq!(back, 0xFFFFFFFF); // White
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

    #[test]
    fn test_update() {
        let mut dialog = ChatDialog::new();

        dialog.receive_chat("Message 1", ChatType::Normal);
        dialog.receive_chat("Message 2", ChatType::System);
        dialog.update();

        assert!(!dialog.history.is_empty());
        assert!(!dialog.chat_lines.is_empty());
    }

    #[test]
    fn test_scroll_bounds() {
        let mut dialog = ChatDialog::new();

        // Test scroll bounds with empty history
        assert_eq!(dialog.start_index, 0);

        // Add some messages
        for i in 0..10 {
            dialog.receive_chat(&format!("Message {}", i), ChatType::Normal);
        }

        // Test scrolling
        dialog.start_index = 5;
        dialog.update();
        assert_eq!(dialog.start_index, 5);

        // Test out of bounds
        dialog.start_index = 100;
        dialog.update();
        assert_eq!(dialog.start_index, dialog.history.len().saturating_sub(1));
    }

    #[test]
    fn test_split_chat_text() {
        let dialog = ChatDialog::new();

        let lines = dialog.split_chat_text("Short message");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "Short message");

        // Test with longer text (would be split in real implementation)
        let long_text = "This is a very long message that should be split into multiple lines when displayed in the chat window.";
        let lines = dialog.split_chat_text(long_text);
        assert!(!lines.is_empty());
    }
}
