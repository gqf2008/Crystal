// MainDialog - Main game UI
// Mirrors Client/MirScenes/Dialogs/MainDialog.cs

use super::Dialog;

/// Main dialog - primary game UI
#[derive(Debug)]
pub struct MainDialog {
    pub visible: bool,
    
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
    
    // TODO: Button states
    // pub inventory_button: Button,
    // pub character_button: Button,
    // pub skills_button: Button,
    // pub guild_button: Button,
    // pub quest_button: Button,
}

impl MainDialog {
    pub fn new() -> Self {
        Self {
            visible: true, // Main dialog usually always visible
            hp: 100,
            max_hp: 100,
            mp: 50,
            max_mp: 50,
            experience: 0,
            max_experience: 100,
            level: 1,
            gold: 0,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main_dialog_creation() {
        let dialog = MainDialog::new();
        assert!(dialog.visible);
        assert_eq!(dialog.level, 1);
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
}
