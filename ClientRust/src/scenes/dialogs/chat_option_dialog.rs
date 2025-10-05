//! Chat Option Dialog
//!
//! Chat filtering and transparency options dialog.
//! Corresponds to Client/MirScenes/Dialogs/ChatOptionDialog.cs

/// 聊天选项标签页
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatOptionTab {
    Filters = 0,   // 过滤器
    Chat = 1,      // 聊天设置
}

/// 聊天选项对话框 - 管理聊天过滤器和透明度设置
#[derive(Debug, Clone)]
pub struct ChatOptionDialog {
    /// 是否可见
    pub visible: bool,
    /// 窗口位置
    pub location: (i32, i32),
    /// 窗口大小
    pub size: (i32, i32),

    /// 当前标签页
    pub current_tab: ChatOptionTab,

    /// 聊天过滤器设置
    pub filter_normal_chat: bool,
    pub filter_whisper_chat: bool,
    pub filter_shout_chat: bool,
    pub filter_system_chat: bool,
    pub filter_lover_chat: bool,
    pub filter_mentor_chat: bool,
    pub filter_group_chat: bool,
    pub filter_guild_chat: bool,

    /// 所有过滤器是否关闭
    pub all_filters_off: bool,

    /// 聊天透明度设置
    pub transparent_chat: bool,
}

impl Default for ChatOptionDialog {
    fn default() -> Self {
        Self {
            visible: false,
            location: (400 - 112, 300 - 90), // Centered on 800x600, size 224x180
            size: (224, 180),
            current_tab: ChatOptionTab::Filters,
            filter_normal_chat: false,
            filter_whisper_chat: false,
            filter_shout_chat: false,
            filter_system_chat: false,
            filter_lover_chat: false,
            filter_group_chat: false,
            filter_guild_chat: false,
            filter_mentor_chat: false,
            all_filters_off: true,
            transparent_chat: false,
        }
    }
}

impl ChatOptionDialog {
    /// 创建新的聊天选项对话框
    pub fn new() -> Self {
        let mut dialog = Self::default();
        dialog.check_all_filters();
        dialog.update_transparency();
        dialog
    }

    /// 显示对话框
    pub fn show(&mut self) {
        if self.visible {
            return;
        }
        self.visible = true;
    }

    /// 隐藏对话框
    pub fn hide(&mut self) {
        if !self.visible {
            return;
        }
        self.visible = false;
    }

    /// 切换显示状态
    pub fn toggle(&mut self) {
        if !self.visible {
            self.show();
        } else {
            self.hide();
        }
    }

    /// 切换标签页
    pub fn switch_tab(&mut self, tab: ChatOptionTab) {
        self.current_tab = tab;
        // Note: UI visibility updates would happen here in the actual implementation
    }

    /// 切换所有过滤器
    pub fn toggle_all_filters(&mut self) {
        if self.all_filters_off {
            // 开启所有过滤器
            self.filter_normal_chat = true;
            self.filter_whisper_chat = true;
            self.filter_shout_chat = true;
            self.filter_system_chat = true;
            self.filter_lover_chat = true;
            self.filter_mentor_chat = true;
            self.filter_group_chat = true;
            self.filter_guild_chat = true;
        } else {
            // 关闭所有过滤器
            self.filter_normal_chat = false;
            self.filter_whisper_chat = false;
            self.filter_shout_chat = false;
            self.filter_system_chat = false;
            self.filter_lover_chat = false;
            self.filter_mentor_chat = false;
            self.filter_group_chat = false;
            self.filter_guild_chat = false;
        }

        self.all_filters_off = !self.all_filters_off;
        self.update_chat_display();
    }

    /// 切换普通聊天过滤
    pub fn toggle_normal_chat_filter(&mut self) {
        self.filter_normal_chat = !self.filter_normal_chat;
        self.check_all_filters();
    }

    /// 切换私聊过滤
    pub fn toggle_whisper_chat_filter(&mut self) {
        self.filter_whisper_chat = !self.filter_whisper_chat;
        self.check_all_filters();
    }

    /// 切换喊话过滤
    pub fn toggle_shout_chat_filter(&mut self) {
        self.filter_shout_chat = !self.filter_shout_chat;
        self.check_all_filters();
    }

    /// 切换系统消息过滤
    pub fn toggle_system_chat_filter(&mut self) {
        self.filter_system_chat = !self.filter_system_chat;
        self.check_all_filters();
    }

    /// 切换恋人聊天过滤
    pub fn toggle_lover_chat_filter(&mut self) {
        self.filter_lover_chat = !self.filter_lover_chat;
        self.check_all_filters();
    }

    /// 切换导师聊天过滤
    pub fn toggle_mentor_chat_filter(&mut self) {
        self.filter_mentor_chat = !self.filter_mentor_chat;
        self.check_all_filters();
    }

    /// 切换队伍聊天过滤
    pub fn toggle_group_chat_filter(&mut self) {
        self.filter_group_chat = !self.filter_group_chat;
        self.check_all_filters();
    }

    /// 切换公会聊天过滤
    pub fn toggle_guild_chat_filter(&mut self) {
        self.filter_guild_chat = !self.filter_guild_chat;
        self.check_all_filters();
    }

    /// 切换聊天透明度
    pub fn toggle_transparency(&mut self) {
        self.transparent_chat = !self.transparent_chat;
        self.update_transparency();
    }

    /// 检查所有过滤器状态
    fn check_all_filters(&mut self) {
        if !self.filter_normal_chat && !self.filter_whisper_chat
            && !self.filter_shout_chat && !self.filter_system_chat
            && !self.filter_lover_chat && !self.filter_mentor_chat
            && !self.filter_group_chat && !self.filter_guild_chat {
            self.all_filters_off = true;
        } else {
            self.all_filters_off = false;
        }

        self.update_chat_display();
    }

    /// 更新聊天显示
    fn update_chat_display(&mut self) {
        // TODO: Update the actual chat dialog display
        // GameScene.Scene.ChatDialog.Update();
        println!("Updating chat display with current filter settings");
    }

    /// 更新透明度设置
    fn update_transparency(&mut self) {
        // TODO: Update chat dialog transparency
        if self.transparent_chat {
            // GameScene.Scene.ChatDialog.ForeColour = Color.FromArgb(15, 0, 0);
            // GameScene.Scene.ChatDialog.BackColour = Color.FromArgb(15, 0, 0);
            // GameScene.Scene.ChatDialog.Opacity = 0.8f;
            println!("Setting chat to transparent mode");
        } else {
            // GameScene.Scene.ChatDialog.ForeColour = Color.White;
            // GameScene.Scene.ChatDialog.BackColour = Color.White;
            // GameScene.Scene.ChatDialog.Opacity = 1;
            println!("Setting chat to normal mode");
        }
    }

    /// 获取过滤器按钮状态
    pub fn get_filter_button_index(&self, filter_type: &str) -> i32 {
        match filter_type {
            "all" => if self.all_filters_off { 2087 } else { 2086 },
            "normal" => if self.filter_normal_chat { 2070 } else { 2071 },
            "whisper" => if self.filter_whisper_chat { 2074 } else { 2075 },
            "shout" => if self.filter_shout_chat { 2072 } else { 2073 },
            "system" => if self.filter_system_chat { 2084 } else { 2085 },
            "lover" => if self.filter_lover_chat { 2076 } else { 2077 },
            "mentor" => if self.filter_mentor_chat { 2078 } else { 2079 },
            "group" => if self.filter_group_chat { 2080 } else { 2081 },
            "guild" => if self.filter_guild_chat { 2082 } else { 2083 },
            _ => 0,
        }
    }

    /// 获取透明度按钮状态
    pub fn get_transparency_button_indices(&self) -> (i32, i32, i32, i32) {
        if self.transparent_chat {
            (474, 475, 470, 470) // On button active, Off button inactive
        } else {
            (473, 473, 471, 472) // On button inactive, Off button active
        }
    }

    /// 获取对话框背景索引
    pub fn get_background_index(&self) -> i32 {
        match self.current_tab {
            ChatOptionTab::Filters => 466,
            ChatOptionTab::Chat => 467,
        }
    }

    /// 获取标签页按钮索引
    pub fn get_tab_button_indices(&self) -> (i32, i32, i32, i32) {
        match self.current_tab {
            ChatOptionTab::Filters => (463, 462, 464, 465), // Filter active, Chat inactive
            ChatOptionTab::Chat => (462, 463, 465, 464),    // Filter inactive, Chat active
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_option_dialog_creation() {
        let dialog = ChatOptionDialog::new();
        assert!(!dialog.visible);
        assert_eq!(dialog.current_tab, ChatOptionTab::Filters);
        assert!(dialog.all_filters_off);
        assert!(!dialog.transparent_chat);
    }

    #[test]
    fn test_show_hide() {
        let mut dialog = ChatOptionDialog::new();
        dialog.show();
        assert!(dialog.visible);
        dialog.hide();
        assert!(!dialog.visible);
    }

    #[test]
    fn test_toggle() {
        let mut dialog = ChatOptionDialog::new();
        dialog.toggle();
        assert!(dialog.visible);
        dialog.toggle();
        assert!(!dialog.visible);
    }

    #[test]
    fn test_switch_tab() {
        let mut dialog = ChatOptionDialog::new();
        assert_eq!(dialog.current_tab, ChatOptionTab::Filters);

        dialog.switch_tab(ChatOptionTab::Chat);
        assert_eq!(dialog.current_tab, ChatOptionTab::Chat);

        dialog.switch_tab(ChatOptionTab::Filters);
        assert_eq!(dialog.current_tab, ChatOptionTab::Filters);
    }

    #[test]
    fn test_toggle_all_filters() {
        let mut dialog = ChatOptionDialog::new();
        assert!(dialog.all_filters_off);

        dialog.toggle_all_filters();
        assert!(!dialog.all_filters_off);
        assert!(dialog.filter_normal_chat);
        assert!(dialog.filter_whisper_chat);
        assert!(dialog.filter_shout_chat);
        assert!(dialog.filter_system_chat);
        assert!(dialog.filter_lover_chat);
        assert!(dialog.filter_mentor_chat);
        assert!(dialog.filter_group_chat);
        assert!(dialog.filter_guild_chat);

        dialog.toggle_all_filters();
        assert!(dialog.all_filters_off);
        assert!(!dialog.filter_normal_chat);
        assert!(!dialog.filter_whisper_chat);
        assert!(!dialog.filter_shout_chat);
        assert!(!dialog.filter_system_chat);
        assert!(!dialog.filter_lover_chat);
        assert!(!dialog.filter_mentor_chat);
        assert!(!dialog.filter_group_chat);
        assert!(!dialog.filter_guild_chat);
    }

    #[test]
    fn test_individual_filter_toggles() {
        let mut dialog = ChatOptionDialog::new();

        dialog.toggle_normal_chat_filter();
        assert!(dialog.filter_normal_chat);
        assert!(!dialog.all_filters_off);

        dialog.toggle_whisper_chat_filter();
        assert!(dialog.filter_whisper_chat);

        // Turn off normal chat
        dialog.toggle_normal_chat_filter();
        assert!(!dialog.filter_normal_chat);
        assert!(!dialog.all_filters_off); // Still has whisper filter on

        // Turn off whisper chat
        dialog.toggle_whisper_chat_filter();
        assert!(!dialog.filter_whisper_chat);
        assert!(dialog.all_filters_off); // All filters off now
    }

    #[test]
    fn test_toggle_transparency() {
        let mut dialog = ChatOptionDialog::new();
        assert!(!dialog.transparent_chat);

        dialog.toggle_transparency();
        assert!(dialog.transparent_chat);

        dialog.toggle_transparency();
        assert!(!dialog.transparent_chat);
    }

    #[test]
    fn test_get_filter_button_index() {
        let mut dialog = ChatOptionDialog::new();

        // All filters off
        assert_eq!(dialog.get_filter_button_index("all"), 2087);

        // Turn on normal chat filter
        dialog.toggle_normal_chat_filter();
        assert_eq!(dialog.get_filter_button_index("normal"), 2070);
        assert_eq!(dialog.get_filter_button_index("all"), 2086); // All not off anymore

        // Test other filters
        dialog.toggle_whisper_chat_filter();
        assert_eq!(dialog.get_filter_button_index("whisper"), 2074);

        dialog.toggle_shout_chat_filter();
        assert_eq!(dialog.get_filter_button_index("shout"), 2072);

        dialog.toggle_system_chat_filter();
        assert_eq!(dialog.get_filter_button_index("system"), 2084);
    }

    #[test]
    fn test_get_transparency_button_indices() {
        let mut dialog = ChatOptionDialog::new();

        // Normal mode
        let (on_idx, on_hover, off_idx, off_hover) = dialog.get_transparency_button_indices();
        assert_eq!((on_idx, on_hover, off_idx, off_hover), (473, 473, 471, 472));

        // Transparent mode
        dialog.toggle_transparency();
        let (on_idx, on_hover, off_idx, off_hover) = dialog.get_transparency_button_indices();
        assert_eq!((on_idx, on_hover, off_idx, off_hover), (474, 475, 470, 470));
    }

    #[test]
    fn test_get_background_index() {
        let mut dialog = ChatOptionDialog::new();

        assert_eq!(dialog.get_background_index(), 466); // Filters tab

        dialog.switch_tab(ChatOptionTab::Chat);
        assert_eq!(dialog.get_background_index(), 467); // Chat tab
    }

    #[test]
    fn test_get_tab_button_indices() {
        let mut dialog = ChatOptionDialog::new();

        // Filters tab active
        let (filter_idx, filter_pressed, chat_idx, chat_pressed) = dialog.get_tab_button_indices();
        assert_eq!((filter_idx, filter_pressed, chat_idx, chat_pressed), (463, 462, 464, 465));

        // Chat tab active
        dialog.switch_tab(ChatOptionTab::Chat);
        let (filter_idx, filter_pressed, chat_idx, chat_pressed) = dialog.get_tab_button_indices();
        assert_eq!((filter_idx, filter_pressed, chat_idx, chat_pressed), (462, 463, 465, 464));
    }

    #[test]
    fn test_all_filter_types() {
        let mut dialog = ChatOptionDialog::new();

        // Test all filter types
        let filter_types = vec![
            ("lover", "filter_lover_chat"),
            ("mentor", "filter_mentor_chat"),
            ("group", "filter_group_chat"),
            ("guild", "filter_guild_chat"),
        ];

        for (filter_name, _field_name) in filter_types {
            // Turn on filter
            match filter_name {
                "lover" => dialog.toggle_lover_chat_filter(),
                "mentor" => dialog.toggle_mentor_chat_filter(),
                "group" => dialog.toggle_group_chat_filter(),
                "guild" => dialog.toggle_guild_chat_filter(),
                _ => {}
            }
            assert!(!dialog.all_filters_off);

            // Turn off filter
            match filter_name {
                "lover" => dialog.toggle_lover_chat_filter(),
                "mentor" => dialog.toggle_mentor_chat_filter(),
                "group" => dialog.toggle_group_chat_filter(),
                "guild" => dialog.toggle_guild_chat_filter(),
                _ => {}
            }
        }

        // All should be off now
        assert!(dialog.all_filters_off);
    }
}