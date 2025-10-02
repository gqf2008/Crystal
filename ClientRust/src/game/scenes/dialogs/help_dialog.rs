// HelpDialog - In-game help system
// Rust implementation of Client/MirScenes/Dialogs/HelpDialog.cs

use crate::game::scenes::dialogs::Dialog;

/// Help page type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpPageType {
    Image,    // Image-based page
    Text,     // Text-based page
    Shortcut, // Keyboard shortcut page
}

/// Help page content
#[derive(Debug, Clone)]
pub struct HelpPage {
    pub title: String,
    pub page_type: HelpPageType,
    pub image_index: i32, // -1 for no image
    pub content: String,
    pub shortcuts: Vec<(String, String)>, // (Key, Description) pairs
}

impl HelpPage {
    pub fn new_image(title: String, image_index: i32) -> Self {
        Self {
            title,
            page_type: HelpPageType::Image,
            image_index,
            content: String::new(),
            shortcuts: Vec::new(),
        }
    }

    pub fn new_text(title: String, content: String) -> Self {
        Self {
            title,
            page_type: HelpPageType::Text,
            image_index: -1,
            content,
            shortcuts: Vec::new(),
        }
    }

    pub fn new_shortcut(title: String, shortcuts: Vec<(String, String)>) -> Self {
        Self {
            title,
            page_type: HelpPageType::Shortcut,
            image_index: -1,
            content: String::new(),
            shortcuts,
        }
    }

    pub fn has_image(&self) -> bool {
        self.image_index >= 0
    }
}

/// Help Dialog - Display game help and tutorials
pub struct HelpDialog {
    visible: bool,
    pub pages: Vec<HelpPage>,
    pub current_page_number: usize,
}

impl HelpDialog {
    pub fn new() -> Self {
        let mut dialog = Self {
            visible: false,
            pages: Vec::new(),
            current_page_number: 0,
        };
        dialog.load_default_pages();
        dialog
    }

    fn load_default_pages(&mut self) {
        // Shortcut pages
        self.add_page(HelpPage::new_shortcut(
            "Shortcut Information".to_string(),
            vec![
                ("F1".to_string(), "Toggle Character Window".to_string()),
                ("F2".to_string(), "Toggle Inventory".to_string()),
                ("F3".to_string(), "Toggle Skills".to_string()),
                ("F4".to_string(), "Toggle Quests".to_string()),
                ("F5".to_string(), "Toggle Options".to_string()),
                ("F6".to_string(), "Toggle Guild".to_string()),
                ("F7".to_string(), "Toggle Trade".to_string()),
                ("F8".to_string(), "Toggle Friends".to_string()),
                ("F9".to_string(), "Toggle Help".to_string()),
                ("F10".to_string(), "Toggle Mini Map".to_string()),
                ("F11".to_string(), "Toggle Big Map".to_string()),
                ("F12".to_string(), "Screenshot".to_string()),
            ],
        ));

        self.add_page(HelpPage::new_shortcut(
            "Combat Shortcuts".to_string(),
            vec![
                ("1-8".to_string(), "Use Skill in Slot".to_string()),
                ("Tab".to_string(), "Target Next Enemy".to_string()),
                ("Ctrl+1-8".to_string(), "Assign Skill to Slot".to_string()),
                ("Alt+Click".to_string(), "Force Attack".to_string()),
                ("Shift+Click".to_string(), "Pick Up Item".to_string()),
            ],
        ));

        self.add_page(HelpPage::new_shortcut(
            "Chat Shortcuts".to_string(),
            vec![
                ("Enter".to_string(), "Open Chat".to_string()),
                ("/".to_string(), "Normal Chat".to_string()),
                ("!".to_string(), "Shout Chat".to_string()),
                ("~".to_string(), "Global Chat".to_string()),
                ("@".to_string(), "Whisper".to_string()),
                ("#".to_string(), "Group Chat".to_string()),
                ("$".to_string(), "Guild Chat".to_string()),
            ],
        ));

        // Image-based help pages
        self.add_page(HelpPage::new_image("Movements".to_string(), 0));
        self.add_page(HelpPage::new_image("Attacking".to_string(), 1));
        self.add_page(HelpPage::new_image("Collecting Items".to_string(), 2));
        self.add_page(HelpPage::new_image("Health".to_string(), 3));
        self.add_page(HelpPage::new_image("Skills".to_string(), 4));
        self.add_page(HelpPage::new_image("Mana".to_string(), 6));
        self.add_page(HelpPage::new_image("Chatting".to_string(), 7));
        self.add_page(HelpPage::new_image("Groups".to_string(), 8));
        self.add_page(HelpPage::new_image("Durability".to_string(), 9));
        self.add_page(HelpPage::new_image("Purchasing".to_string(), 10));
        self.add_page(HelpPage::new_image("Selling".to_string(), 11));
        self.add_page(HelpPage::new_image("Repairing".to_string(), 12));
        self.add_page(HelpPage::new_image("Trading".to_string(), 13));
        self.add_page(HelpPage::new_image("Inspecting".to_string(), 14));
        self.add_page(HelpPage::new_image("Statistics".to_string(), 15));
        self.add_page(HelpPage::new_image("Quests".to_string(), 21));
        self.add_page(HelpPage::new_image("Mounts".to_string(), 25));
        self.add_page(HelpPage::new_image("Fishing".to_string(), 27));
        self.add_page(HelpPage::new_image("Gems and Orbs".to_string(), 28));
        self.add_page(HelpPage::new_image("Heroes".to_string(), 29));
        self.add_page(HelpPage::new_image("Guild Buffs".to_string(), 34));
        self.add_page(HelpPage::new_image("Awakening".to_string(), 37));

        // Text-based pages
        self.add_page(HelpPage::new_text(
            "Welcome".to_string(),
            "Welcome to the game! This help system contains information about game mechanics, \
             controls, and features. Use the Previous and Next buttons to navigate through pages."
                .to_string(),
        ));

        self.add_page(HelpPage::new_text(
            "Basic Controls".to_string(),
            "Mouse: Click to move and interact\n\
             WASD/Arrow Keys: Alternative movement\n\
             Left Click: Select/Move\n\
             Right Click: Attack/Interact\n\
             Mouse Wheel: Zoom in/out"
                .to_string(),
        ));
    }

    pub fn add_page(&mut self, page: HelpPage) {
        self.pages.push(page);
    }

    pub fn get_current_page(&self) -> Option<&HelpPage> {
        self.pages.get(self.current_page_number)
    }

    pub fn next_page(&mut self) {
        if self.pages.is_empty() {
            return;
        }
        self.current_page_number = (self.current_page_number + 1) % self.pages.len();
    }

    pub fn previous_page(&mut self) {
        if self.pages.is_empty() {
            return;
        }
        if self.current_page_number == 0 {
            self.current_page_number = self.pages.len() - 1;
        } else {
            self.current_page_number -= 1;
        }
    }

    pub fn goto_page(&mut self, page_number: usize) -> bool {
        if page_number < self.pages.len() {
            self.current_page_number = page_number;
            true
        } else {
            false
        }
    }

    pub fn display_page_by_title(&mut self, title: &str) -> bool {
        if let Some(index) = self.pages.iter().position(|p| p.title == title) {
            self.current_page_number = index;
            true
        } else {
            false
        }
    }

    pub fn find_page_by_keyword(&self, keyword: &str) -> Vec<usize> {
        let keyword_lower = keyword.to_lowercase();
        self.pages
            .iter()
            .enumerate()
            .filter(|(_, page)| {
                page.title.to_lowercase().contains(&keyword_lower)
                    || page.content.to_lowercase().contains(&keyword_lower)
                    || page
                        .shortcuts
                        .iter()
                        .any(|(k, d)| k.to_lowercase().contains(&keyword_lower) || d.to_lowercase().contains(&keyword_lower))
            })
            .map(|(idx, _)| idx)
            .collect()
    }

    pub fn get_page_label(&self) -> String {
        if self.pages.is_empty() {
            "0 / 0".to_string()
        } else {
            format!("{} / {}", self.current_page_number + 1, self.pages.len())
        }
    }

    pub fn total_pages(&self) -> usize {
        self.pages.len()
    }

    pub fn has_next_page(&self) -> bool {
        !self.pages.is_empty()
    }

    pub fn has_previous_page(&self) -> bool {
        !self.pages.is_empty()
    }

    pub fn clear_pages(&mut self) {
        self.pages.clear();
        self.current_page_number = 0;
    }

    pub fn reset_to_first_page(&mut self) {
        self.current_page_number = 0;
    }
}

impl Dialog for HelpDialog {
    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn update(&mut self, _delta_time: f32) {
        // Update logic
    }

    fn draw(&self) {
        if !self.visible {
            return;
        }
        // Draw logic would render current page
    }

    fn is_visible(&self) -> bool {
        self.visible
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_page_creation() {
        let page = HelpPage::new_image("Test".to_string(), 5);
        assert_eq!(page.title, "Test");
        assert_eq!(page.page_type, HelpPageType::Image);
        assert_eq!(page.image_index, 5);
        assert!(page.has_image());
    }

    #[test]
    fn test_text_page() {
        let page = HelpPage::new_text("Info".to_string(), "Content".to_string());
        assert_eq!(page.page_type, HelpPageType::Text);
        assert_eq!(page.content, "Content");
        assert!(!page.has_image());
    }

    #[test]
    fn test_shortcut_page() {
        let shortcuts = vec![
            ("F1".to_string(), "Help".to_string()),
            ("F2".to_string(), "Inventory".to_string()),
        ];
        let page = HelpPage::new_shortcut("Shortcuts".to_string(), shortcuts);
        assert_eq!(page.page_type, HelpPageType::Shortcut);
        assert_eq!(page.shortcuts.len(), 2);
    }

    #[test]
    fn test_help_dialog_creation() {
        let dialog = HelpDialog::new();
        assert!(!dialog.is_visible());
        assert!(dialog.total_pages() > 0); // Default pages loaded
    }

    #[test]
    fn test_add_page() {
        let mut dialog = HelpDialog::new();
        let initial_count = dialog.total_pages();
        
        dialog.add_page(HelpPage::new_text("Custom".to_string(), "Test".to_string()));
        assert_eq!(dialog.total_pages(), initial_count + 1);
    }

    #[test]
    fn test_navigation() {
        let mut dialog = HelpDialog::new();
        dialog.clear_pages();
        dialog.add_page(HelpPage::new_text("Page 1".to_string(), "".to_string()));
        dialog.add_page(HelpPage::new_text("Page 2".to_string(), "".to_string()));
        dialog.add_page(HelpPage::new_text("Page 3".to_string(), "".to_string()));

        assert_eq!(dialog.current_page_number, 0);
        
        dialog.next_page();
        assert_eq!(dialog.current_page_number, 1);
        
        dialog.next_page();
        assert_eq!(dialog.current_page_number, 2);
        
        // Wrap around
        dialog.next_page();
        assert_eq!(dialog.current_page_number, 0);
        
        dialog.previous_page();
        assert_eq!(dialog.current_page_number, 2);
    }

    #[test]
    fn test_goto_page() {
        let mut dialog = HelpDialog::new();
        dialog.clear_pages();
        for i in 0..5 {
            dialog.add_page(HelpPage::new_text(format!("Page {}", i), "".to_string()));
        }

        assert!(dialog.goto_page(3));
        assert_eq!(dialog.current_page_number, 3);
        
        assert!(!dialog.goto_page(10));
    }

    #[test]
    fn test_display_page_by_title() {
        let mut dialog = HelpDialog::new();
        dialog.clear_pages();
        dialog.add_page(HelpPage::new_text("Movements".to_string(), "".to_string()));
        dialog.add_page(HelpPage::new_text("Combat".to_string(), "".to_string()));

        assert!(dialog.display_page_by_title("Combat"));
        assert_eq!(dialog.current_page_number, 1);
        
        assert!(!dialog.display_page_by_title("NonExistent"));
    }

    #[test]
    fn test_find_page_by_keyword() {
        let mut dialog = HelpDialog::new();
        dialog.clear_pages();
        dialog.add_page(HelpPage::new_text(
            "Combat".to_string(),
            "Attack enemies".to_string(),
        ));
        dialog.add_page(HelpPage::new_text(
            "Trading".to_string(),
            "Exchange items".to_string(),
        ));

        let results = dialog.find_page_by_keyword("attack");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], 0);
    }

    #[test]
    fn test_get_current_page() {
        let mut dialog = HelpDialog::new();
        dialog.clear_pages();
        dialog.add_page(HelpPage::new_text("Test".to_string(), "Content".to_string()));

        let page = dialog.get_current_page();
        assert!(page.is_some());
        assert_eq!(page.unwrap().title, "Test");
    }

    #[test]
    fn test_page_label() {
        let mut dialog = HelpDialog::new();
        dialog.clear_pages();
        assert_eq!(dialog.get_page_label(), "0 / 0");

        dialog.add_page(HelpPage::new_text("Page 1".to_string(), "".to_string()));
        dialog.add_page(HelpPage::new_text("Page 2".to_string(), "".to_string()));
        assert_eq!(dialog.get_page_label(), "1 / 2");

        dialog.next_page();
        assert_eq!(dialog.get_page_label(), "2 / 2");
    }

    #[test]
    fn test_reset_to_first() {
        let mut dialog = HelpDialog::new();
        dialog.clear_pages();
        dialog.add_page(HelpPage::new_text("Page 1".to_string(), "".to_string()));
        dialog.add_page(HelpPage::new_text("Page 2".to_string(), "".to_string()));

        dialog.goto_page(1);
        assert_eq!(dialog.current_page_number, 1);
        
        dialog.reset_to_first_page();
        assert_eq!(dialog.current_page_number, 0);
    }

    #[test]
    fn test_default_pages_loaded() {
        let dialog = HelpDialog::new();
        // Should have multiple default pages
        assert!(dialog.total_pages() > 10);
        
        // Check for specific pages
        let has_shortcuts = dialog.pages.iter().any(|p| p.title.contains("Shortcut"));
        assert!(has_shortcuts);
    }

    #[test]
    fn test_empty_dialog() {
        let mut dialog = HelpDialog::new();
        dialog.clear_pages();
        
        assert_eq!(dialog.total_pages(), 0);
        assert!(dialog.get_current_page().is_none());
        
        // Should not crash on navigation
        dialog.next_page();
        dialog.previous_page();
    }

    #[test]
    fn test_has_next_previous() {
        let mut dialog = HelpDialog::new();
        dialog.clear_pages();
        
        assert!(!dialog.has_next_page());
        assert!(!dialog.has_previous_page());
        
        dialog.add_page(HelpPage::new_text("Page".to_string(), "".to_string()));
        
        assert!(dialog.has_next_page());
        assert!(dialog.has_previous_page());
    }

    #[test]
    fn test_find_shortcut_keyword() {
        let mut dialog = HelpDialog::new();
        dialog.clear_pages();
        
        let shortcuts = vec![
            ("F1".to_string(), "Toggle Character".to_string()),
            ("F2".to_string(), "Toggle Inventory".to_string()),
        ];
        dialog.add_page(HelpPage::new_shortcut("Keys".to_string(), shortcuts));

        let results = dialog.find_page_by_keyword("inventory");
        assert_eq!(results.len(), 1);
    }
}
