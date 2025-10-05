// MainDialog - Main game UI
// Mirrors Client/MirScenes/Dialogs/MainDialog.cs

use super::Dialog;

/// Main dialog - primary game UI
#[derive(Debug)]
pub struct MainDialog {
    pub visible: bool,

    // Position and size
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,

    // Health/Mana display
    pub hp: i32,
    pub max_hp: i32,
    pub mp: i32,
    pub max_mp: i32,

    // Experience
    pub experience: i64,
    pub max_experience: i64,

    // Level
    pub level: u16,

    // Gold display
    pub gold: u32,

    // Character info
    pub character_name: String,

    // Weight info
    pub current_bag_weight: i32,
    pub max_bag_weight: i32,

    // Inventory space
    pub inventory_slots: usize,
    pub used_inventory_slots: usize,

    // UI Elements (simplified for now)
    pub inventory_button_pressed: bool,
    pub character_button_pressed: bool,
    pub skill_button_pressed: bool,
    pub quest_button_pressed: bool,
    pub option_button_pressed: bool,
    pub menu_button_pressed: bool,
    pub game_shop_button_pressed: bool,

    // Hero system
    pub hero_menu_button_visible: bool,
    pub hero_summon_button_visible: bool,

    // Mode display
    pub attack_mode_text: String,
    pub pet_mode_text: String,
    pub skill_mode_text: String,

    // HP display mode
    pub hp_view: bool,
    pub hp_only: bool, // Warrior level < 26
}

impl MainDialog {
    pub fn new() -> Self {
        Self {
            visible: true, // Main dialog usually always visible
            x: 0,
            y: 0,
            width: 800,
            height: 100,
            hp: 100,
            max_hp: 100,
            mp: 50,
            max_mp: 50,
            experience: 0,
            max_experience: 100,
            level: 1,
            gold: 0,
            character_name: String::new(),
            current_bag_weight: 0,
            max_bag_weight: 100,
            inventory_slots: 46, // Default inventory size
            used_inventory_slots: 0,
            inventory_button_pressed: false,
            character_button_pressed: false,
            skill_button_pressed: false,
            quest_button_pressed: false,
            option_button_pressed: false,
            menu_button_pressed: false,
            game_shop_button_pressed: false,
            hero_menu_button_visible: false,
            hero_summon_button_visible: false,
            attack_mode_text: String::new(),
            pet_mode_text: String::new(),
            skill_mode_text: String::new(),
            hp_view: true,
            hp_only: false,
        }
    }
    
    /// Update HP display
    pub fn set_hp(&mut self, hp: i32, max_hp: i32) {
        self.hp = hp;
        self.max_hp = max_hp;
    }

    /// Update MP display
    pub fn set_mp(&mut self, mp: i32, max_mp: i32) {
        self.mp = mp;
        self.max_mp = max_mp;
    }

    /// Update experience display
    pub fn set_experience(&mut self, exp: i64, max_exp: i64) {
        self.experience = exp;
        self.max_experience = max_exp;
    }

    /// Update level
    pub fn set_level(&mut self, level: u16) {
        self.level = level;
    }

    /// Update gold
    pub fn set_gold(&mut self, gold: u32) {
        self.gold = gold;
    }

    /// Update character name
    pub fn set_character_name(&mut self, name: String) {
        self.character_name = name;
    }

    /// Update weight info
    pub fn set_weight(&mut self, current: i32, max: i32) {
        self.current_bag_weight = current;
        self.max_bag_weight = max;
    }

    /// Update inventory space
    pub fn set_inventory_space(&mut self, used: usize, total: usize) {
        self.used_inventory_slots = used;
        self.inventory_slots = total;
    }

    /// Set HP view mode
    pub fn set_hp_view(&mut self, hp_view: bool) {
        self.hp_view = hp_view;
    }

    /// Set HP only mode (for warriors level < 26)
    pub fn set_hp_only(&mut self, hp_only: bool) {
        self.hp_only = hp_only;
    }

    /// Update attack mode text
    pub fn set_attack_mode_text(&mut self, text: String) {
        self.attack_mode_text = text;
    }

    /// Update pet mode text
    pub fn set_pet_mode_text(&mut self, text: String) {
        self.pet_mode_text = text;
    }

    /// Update skill mode text
    pub fn set_skill_mode_text(&mut self, text: String) {
        self.skill_mode_text = text;
    }

    /// Set hero buttons visibility
    pub fn set_hero_buttons_visible(&mut self, visible: bool) {
        self.hero_menu_button_visible = visible;
        self.hero_summon_button_visible = visible;
    }

    /// Get available inventory space
    pub fn get_available_inventory_space(&self) -> usize {
        self.inventory_slots - self.used_inventory_slots
    }

    /// Get available weight
    pub fn get_available_weight(&self) -> i32 {
        self.max_bag_weight - self.current_bag_weight
    }
    
    /// Get HP percentage
    pub fn get_hp_percent(&self) -> f32 {
        if self.max_hp == 0 {
            return 0.0;
        }
        (self.hp as f32 / self.max_hp as f32) * 100.0
    }
    
    /// Get MP percentage
    pub fn get_mp_percent(&self) -> f32 {
        if self.max_mp == 0 {
            return 0.0;
        }
        (self.mp as f32 / self.max_mp as f32) * 100.0
    }
    
    /// Get EXP percentage
    pub fn get_exp_percent(&self) -> f32 {
        if self.max_experience == 0 {
            return 0.0;
        }
        (self.experience as f32 / self.max_experience as f32) * 100.0
    }
}

impl Default for MainDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl Dialog for MainDialog {
    fn show(&mut self) {
        self.visible = true;
    }
    
    fn hide(&mut self) {
        self.visible = false;
    }
    
    fn update(&mut self, _delta_time: f32) {
        // TODO: Update animations
        // TODO: Update button states
    }
    
    fn draw(&self) {
        if !self.visible {
            return;
        }
        
        // TODO: Draw main UI background
        // TODO: Draw HP/MP bars
        // TODO: Draw EXP bar
        // TODO: Draw level
        // TODO: Draw gold
        // TODO: Draw buttons
    }
    
    fn is_visible(&self) -> bool {
        self.visible
    }
    
    fn name(&self) -> &str {
        "MainDialog"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main_dialog_creation() {
        let dialog = MainDialog::new();
        assert!(dialog.visible);
        assert_eq!(dialog.level, 1);
        assert_eq!(dialog.inventory_slots, 46);
        assert!(dialog.hp_view);
        assert!(!dialog.hp_only);
    }

    #[test]
    fn test_hp_mp_updates() {
        let mut dialog = MainDialog::new();

        dialog.set_hp(50, 100);
        assert_eq!(dialog.hp, 50);
        assert_eq!(dialog.get_hp_percent(), 50.0);

        dialog.set_mp(25, 100);
        assert_eq!(dialog.mp, 25);
        assert_eq!(dialog.get_mp_percent(), 25.0);
    }

    #[test]
    fn test_character_info_updates() {
        let mut dialog = MainDialog::new();

        dialog.set_level(15);
        assert_eq!(dialog.level, 15);

        dialog.set_character_name("TestPlayer".to_string());
        assert_eq!(dialog.character_name, "TestPlayer");

        dialog.set_gold(12345);
        assert_eq!(dialog.gold, 12345);
    }

    #[test]
    fn test_weight_and_inventory() {
        let mut dialog = MainDialog::new();

        dialog.set_weight(75, 100);
        assert_eq!(dialog.current_bag_weight, 75);
        assert_eq!(dialog.max_bag_weight, 100);
        assert_eq!(dialog.get_available_weight(), 25);

        dialog.set_inventory_space(20, 46);
        assert_eq!(dialog.used_inventory_slots, 20);
        assert_eq!(dialog.inventory_slots, 46);
        assert_eq!(dialog.get_available_inventory_space(), 26);
    }

    #[test]
    fn test_mode_settings() {
        let mut dialog = MainDialog::new();

        dialog.set_attack_mode_text("Peace".to_string());
        assert_eq!(dialog.attack_mode_text, "Peace");

        dialog.set_pet_mode_text("Both".to_string());
        assert_eq!(dialog.pet_mode_text, "Both");

        dialog.set_skill_mode_text("Ctrl".to_string());
        assert_eq!(dialog.skill_mode_text, "Ctrl");

        dialog.set_hp_view(false);
        assert!(!dialog.hp_view);

        dialog.set_hp_only(true);
        assert!(dialog.hp_only);
    }

    #[test]
    fn test_hero_buttons() {
        let mut dialog = MainDialog::new();

        assert!(!dialog.hero_menu_button_visible);
        assert!(!dialog.hero_summon_button_visible);

        dialog.set_hero_buttons_visible(true);
        assert!(dialog.hero_menu_button_visible);
        assert!(dialog.hero_summon_button_visible);
    }
}
