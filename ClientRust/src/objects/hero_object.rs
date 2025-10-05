// HeroObject.rs - Hero character object (player's companion)
// Mirrors Client/MirObjects/HeroObject.cs

use mir2_shared::{
    data::stats::Stats,
    enums::{MirClass, MirGender, Spell},
    packets::server::{ObjectHero, HeroInformation},
    Point, UserItem,
};

use super::player_object::PlayerObject;
use mir2_shared::data::client_data::ClientMagic;

/// Hero spawn state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeroState {
    None,
    Spawned,
    Unsummoned,
    Dead,
}

/// Hero object - represents the player's hero companion
/// 
/// Architecture: HeroObject composes PlayerObject (which composes MapObject)
/// This mirrors C# inheritance: HeroObject : PlayerObject : MapObject
#[derive(Debug, Clone)]
pub struct HeroObject {
    // ==================== PlayerObject Composition ====================
    /// Player object containing all player-specific fields and methods
    /// Includes: appearance, animation, spell casting, drawing, etc.
    pub player: PlayerObject,
    
    // ==================== HeroObject Specific Fields ====================
    
    /// Owner's name
    pub owner_name: String,
    
    /// Owner's ID
    pub owner_id: u32,
    
    /// Current HP
    pub hp: i32,
    
    /// Current MP
    pub mp: i32,
    
    /// Max HP
    pub max_hp: i32,
    
    /// Max MP
    pub max_mp: i32,
    
    /// Current experience
    pub experience: i64,
    
    /// Experience needed for next level
    pub max_experience: i64,
    
    /// Hero spawn state
    pub spawn_state: HeroState,
    
    /// Loyalty (忠诚度, 0-100)
    pub loyalty: u16,
    
    /// Time when summoned (timestamp)
    pub summoned_time: i64,
    
    /// Hero inventory (40 slots)
    pub inventory: Vec<Option<UserItem>>,
    
    /// Hero equipment (14 slots)
    pub equipment: Vec<Option<UserItem>>,
    
    /// Hero skills
    pub magics: Vec<ClientMagic>,
    
    /// Attack speed
    pub attack_speed: i32,
    
    /// Current stats (after equipment)
    pub stats: Stats,
    
    /// Auto-attack flag
    pub auto_attack: bool,
    
    /// Auto-pickup flag
    pub auto_pickup: bool,
    
    /// Follow owner flag
    pub follow_owner: bool,
}

impl HeroObject {
    /// Create a new hero object
    pub fn new(object_id: u32, name: String, class: MirClass, gender: MirGender) -> Self {
        // Create player object with default values
        // Actual values will be set by load() method when server data arrives
        let player = PlayerObject::new(object_id, name, class, gender);
        
        Self {
            player,
            owner_name: String::new(),
            owner_id: 0,
            hp: 0,
            mp: 0,
            max_hp: 0,
            max_mp: 0,
            experience: 0,
            max_experience: 0,
            spawn_state: HeroState::None,
            loyalty: 100,
            summoned_time: 0,
            inventory: vec![None; 40],
            equipment: vec![None; 14],
            magics: Vec::new(),
            attack_speed: 0,
            stats: Stats::default(),
            auto_attack: false,
            auto_pickup: false,
            follow_owner: true,
        }
    }

    /// Load hero information from ObjectHero packet (spawning hero)
    pub fn load_from_object(&mut self, info: &ObjectHero) {
        let player = &info.player;
        
        // Set basic info from ObjectPlayer
        // Set PlayerObject fields
        self.player.map_object.set_name(player.name.clone());
        self.player.map_object.set_name_colour_argb(player.name_colour);
        self.owner_name = info.owner_name.clone();
        
        let location = Point::new(player.location_x, player.location_y);
        self.player.map_object.set_location(location);
        self.player.map_object.set_direction(player.direction);
        
        // Appearance (PlayerObject fields)
        self.player.class = player.class;
        self.player.gender = player.gender;
        self.player.level = player.level;
        self.player.hair = player.hair;
        self.player.weapon = player.weapon as i32;
        self.player.weapon_effect = player.weapon_effect as i32;
        self.player.armour = player.armour as i32;
        
        // State
        self.player.map_object.set_light(player.light as i32);
        self.player.map_object.set_poison(player.poison);
        self.player.map_object.set_dead(player.dead);
        self.player.map_object.set_hidden(player.hidden);
        self.player.map_object.set_buffs(player.buffs.clone());
        
        // Set as spawned
        self.spawn_state = HeroState::Spawned;
    }

    /// Update hero information from HeroInformation packet (hero ID only)
    pub fn load_hero_info(&mut self, _info: &HeroInformation) {
        // HeroInformation only contains hero_id
        // The actual hero data should be in ObjectHero or other detailed packets
        // This packet is mainly used to trigger hero-related events
    }

    /// Check if hero is spawned and alive
    pub fn is_active(&self) -> bool {
        self.spawn_state == HeroState::Spawned && !self.player.map_object.is_dead()
    }

    /// Check if hero can be summoned
    pub fn can_summon(&self) -> bool {
        matches!(
            self.spawn_state,
            HeroState::None | HeroState::Unsummoned
        )
    }

    /// Summon hero
    pub fn summon(&mut self) {
        if self.can_summon() {
            self.spawn_state = HeroState::Spawned;
            self.summoned_time = get_current_time();
        }
    }

    /// Unsummon hero
    pub fn unsummon(&mut self) {
        if self.spawn_state == HeroState::Spawned {
            self.spawn_state = HeroState::Unsummoned;
        }
    }

    /// Update loyalty (decreases over time when summoned)
    pub fn update_loyalty(&mut self, _delta_time: f32) {
        if self.is_active() {
            // Loyalty decreases slowly when hero is active
            // TODO: Implement actual loyalty decrease logic
            // self.loyalty = self.loyalty.saturating_sub(1);
        }
    }

    /// Get hero's damage output
    pub fn get_damage(&self) -> (i32, i32) {
        // Return (min_damage, max_damage)
        // TODO: Stats structure needs proper DC (Damage Class) fields
        // For now, return placeholder values
        (10, 20)
    }

    /// Check if hero should follow owner
    pub fn should_follow_owner(&self, owner_pos: Point) -> bool {
        if !self.follow_owner || !self.is_active() {
            return false;
        }
        
        // Check distance from owner
        let hero_pos = self.player.map_object.location();
        let dx = (hero_pos.x - owner_pos.x).abs();
        let dy = (hero_pos.y - owner_pos.y).abs();
        let distance = (dx * dx + dy * dy) as f32;
        distance.sqrt() > 5.0 // Follow if more than 5 tiles away
    }

    /// Gain experience
    pub fn gain_experience(&mut self, amount: i64) {
        self.experience += amount;
        
        // Check for level up
        while self.experience >= self.max_experience && self.player.level < 255 {
            self.level_up();
        }
    }

    /// Level up
    fn level_up(&mut self) {
        self.player.level += 1;
        self.experience -= self.max_experience;
        
        // TODO: Calculate new max experience
        // self.max_experience = calculate_next_level_exp(self.player.level);
        
        // TODO: Increase stats based on class
        // self.increase_stats();
        
        // Restore HP/MP
        self.hp = self.max_hp;
        self.mp = self.max_mp;
    }

    /// Check if inventory is full
    pub fn is_inventory_full(&self) -> bool {
        self.inventory.iter().all(|slot| slot.is_some())
    }

    /// Find empty inventory slot
    pub fn find_empty_inventory_slot(&self) -> Option<usize> {
        self.inventory.iter().position(|slot| slot.is_none())
    }
    
    // ==================== Delegation Methods to PlayerObject ====================
    
    /// Get current level (delegates to PlayerObject)
    pub fn level(&self) -> u16 {
        self.player.level
    }
    
    /// Set level (delegates to PlayerObject)
    pub fn set_level(&mut self, level: u16) {
        self.player.level = level;
    }
    
    /// Get class (delegates to PlayerObject)
    pub fn class(&self) -> MirClass {
        self.player.class
    }
    
    /// Get gender (delegates to PlayerObject)
    pub fn gender(&self) -> MirGender {
        self.player.gender
    }
    
    /// Get object ID (delegates to MapObject via PlayerObject)
    pub fn object_id(&self) -> u32 {
        self.player.map_object.object_id()
    }
    
    /// Get name (delegates to MapObject via PlayerObject)
    pub fn name(&self) -> &str {
        self.player.map_object.name()
    }
    
    /// Get location (delegates to MapObject via PlayerObject)
    pub fn location(&self) -> Point {
        self.player.map_object.location()
    }
    
    /// Get direction (delegates to MapObject via PlayerObject)
    pub fn direction(&self) -> mir2_shared::enums::MirDirection {
        self.player.map_object.direction()
    }
    
    /// Draw the hero (delegates to PlayerObject)
    pub fn draw(&self, draw_location: Point) {
        self.player.draw(draw_location);
    }
    
    /// Cast a spell (delegates to PlayerObject)
    pub fn cast_spell(
        &mut self, 
        spell: Spell, 
        target_id: u32, 
        target_point: Point,
        spell_level: u8,
        secondary_targets: Vec<u32>,
    ) {
        self.player.cast_spell(spell, target_id, target_point, spell_level, secondary_targets);
    }
    
    /// Update frame animation (delegates to PlayerObject)
    pub fn update_frame_animation(&mut self, delta_time: f32) {
        self.player.update_frame_animation(delta_time);
    }
    
    /// Set appearance (delegates to PlayerObject)
    pub fn set_libraries(&mut self) {
        self.player.set_libraries();
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
    fn test_hero_object_creation() {
        let hero = HeroObject::new(1, "TestHero".to_string(), MirClass::Warrior, MirGender::Male);
        assert_eq!(hero.player.map_object.object_id(), 1);
        assert_eq!(hero.spawn_state, HeroState::None);
        assert_eq!(hero.loyalty, 100);
    }

    #[test]
    fn test_hero_summon_unsummon() {
        let mut hero = HeroObject::new(1, "TestHero".to_string(), MirClass::Warrior, MirGender::Male);
        
        // Initially can summon
        assert!(hero.can_summon());
        assert!(!hero.is_active());
        
        // Summon hero
        hero.summon();
        assert_eq!(hero.spawn_state, HeroState::Spawned);
        assert!(hero.is_active());
        
        // Unsummon hero
        hero.unsummon();
        assert_eq!(hero.spawn_state, HeroState::Unsummoned);
        assert!(!hero.is_active());
    }

    #[test]
    fn test_hero_level_up() {
        let mut hero = HeroObject::new(1, "TestHero".to_string(), MirClass::Warrior, MirGender::Male);
        hero.player.level = 1;
        hero.experience = 0;
        hero.max_experience = 100;
        hero.max_hp = 100;
        hero.max_mp = 50;
        hero.hp = 50; // Damaged
        hero.mp = 25; // Low MP
        
        // Gain enough exp to level up
        hero.gain_experience(100);
        
        assert_eq!(hero.level(), 2);
        assert_eq!(hero.hp, hero.max_hp); // HP restored
        assert_eq!(hero.mp, hero.max_mp); // MP restored
    }

    #[test]
    fn test_hero_inventory() {
        let hero = HeroObject::new(1, "TestHero".to_string(), MirClass::Warrior, MirGender::Male);
        assert_eq!(hero.inventory.len(), 40);
        assert!(!hero.is_inventory_full());
        assert_eq!(hero.find_empty_inventory_slot(), Some(0));
    }
}
