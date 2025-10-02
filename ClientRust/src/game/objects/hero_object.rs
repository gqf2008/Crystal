// HeroObject.rs - Hero character object (player's companion)
// Mirrors Client/MirObjects/HeroObject.cs

use mir2_shared::{
    enums::{HeroSpawnState, MirClass, MirDirection, MirGender},
    stats::Stats,
    Point, UserItem,
};

use super::{map_object::MapObject, user_object::ClientMagic};
use crate::protocol::HeroInformation;

/// Hero spawn state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeroState {
    None,
    Spawned,
    Unsummoned,
    Dead,
}

/// Hero object - represents the player's hero companion
#[derive(Debug, Clone)]
pub struct HeroObject {
    // Inherited from MapObject (via PlayerObject concept)
    pub map_object: MapObject,
    
    // Hero specific fields
    pub owner_name: String,
    pub owner_id: u32,
    
    // Stats
    pub hp: i32,
    pub mp: i32,
    pub max_hp: i32,
    pub max_mp: i32,
    pub level: u16,
    pub experience: i64,
    pub max_experience: i64,
    
    // Hero state
    pub spawn_state: HeroState,
    pub loyalty: u16, // 忠诚度
    pub summoned_time: i64,
    
    // Inventory
    pub inventory: Vec<Option<UserItem>>, // 40 slots
    pub equipment: Vec<Option<UserItem>>, // 14 slots
    
    // Skills
    pub magics: Vec<ClientMagic>,
    
    // Combat
    pub attack_speed: i32,
    pub stats: Stats,
    
    // Appearance
    pub class: MirClass,
    pub gender: MirGender,
    pub hair: u8,
    pub weapon: i32,
    pub weapon_effect: i32,
    pub armour: i32,
    
    // Flags
    pub auto_attack: bool,
    pub auto_pickup: bool,
    pub follow_owner: bool,
}

impl HeroObject {
    /// Create a new hero object
    pub fn new(object_id: u32) -> Self {
        Self {
            map_object: MapObject::new_hero(object_id),
            owner_name: String::new(),
            owner_id: 0,
            hp: 0,
            mp: 0,
            max_hp: 0,
            max_mp: 0,
            level: 1,
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
            class: MirClass::Warrior,
            gender: MirGender::Male,
            hair: 0,
            weapon: 0,
            weapon_effect: 0,
            armour: 0,
            auto_attack: false,
            auto_pickup: false,
            follow_owner: true,
        }
    }

    /// Load hero information from server
    pub fn load(&mut self, info: &HeroInformation) {
        self.map_object.name = info.name.clone();
        self.owner_name = info.owner_name.clone();
        self.owner_id = info.owner_id;
        
        self.map_object.current_location = info.location;
        self.map_object.map_location = info.location;
        self.map_object.direction = info.direction;
        
        self.class = info.class;
        self.gender = info.gender;
        self.level = info.level;
        self.hair = info.hair;
        
        self.hp = info.hp;
        self.mp = info.mp;
        self.max_hp = info.max_hp;
        self.max_mp = info.max_mp;
        
        self.experience = info.experience;
        self.max_experience = info.max_experience;
        
        self.inventory = info.inventory.clone();
        self.equipment = info.equipment.clone();
        
        // TODO: Load magics
        // self.magics = info.magics.clone();
        
        self.weapon = info.weapon;
        self.weapon_effect = info.weapon_effect;
        self.armour = info.armour;
        
        self.spawn_state = match info.spawn_state {
            HeroSpawnState::None => HeroState::None,
            HeroSpawnState::Spawned => HeroState::Spawned,
            HeroSpawnState::Unsummoned => HeroState::Unsummoned,
            HeroSpawnState::Dead => HeroState::Dead,
        };
    }

    /// Check if hero is spawned and alive
    pub fn is_active(&self) -> bool {
        self.spawn_state == HeroState::Spawned && !self.map_object.dead
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
    pub fn update_loyalty(&mut self, delta_time: f32) {
        if self.is_active() {
            // Loyalty decreases slowly when hero is active
            // TODO: Implement actual loyalty decrease logic
            // self.loyalty = self.loyalty.saturating_sub(1);
        }
    }

    /// Get hero's damage output
    pub fn get_damage(&self) -> (i32, i32) {
        // Return (min_damage, max_damage)
        let base_dc = self.stats.min_dc + self.stats.max_dc;
        (base_dc / 2, base_dc)
    }

    /// Check if hero should follow owner
    pub fn should_follow_owner(&self, owner_pos: Point) -> bool {
        if !self.follow_owner || !self.is_active() {
            return false;
        }
        
        // Check distance from owner
        let distance = self.map_object.current_location.distance_to(&owner_pos);
        distance > 5 // Follow if more than 5 tiles away
    }

    /// Gain experience
    pub fn gain_experience(&mut self, amount: i64) {
        self.experience += amount;
        
        // Check for level up
        while self.experience >= self.max_experience && self.level < 255 {
            self.level_up();
        }
    }

    /// Level up
    fn level_up(&mut self) {
        self.level += 1;
        self.experience -= self.max_experience;
        
        // TODO: Calculate new max experience
        // self.max_experience = calculate_next_level_exp(self.level);
        
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
        let hero = HeroObject::new(1);
        assert_eq!(hero.map_object.object_id, 1);
        assert_eq!(hero.spawn_state, HeroState::None);
        assert_eq!(hero.loyalty, 100);
    }

    #[test]
    fn test_hero_summon_unsummon() {
        let mut hero = HeroObject::new(1);
        
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
        let mut hero = HeroObject::new(1);
        hero.level = 1;
        hero.experience = 0;
        hero.max_experience = 100;
        hero.max_hp = 100;
        hero.max_mp = 50;
        hero.hp = 50; // Damaged
        hero.mp = 25; // Low MP
        
        // Gain enough exp to level up
        hero.gain_experience(100);
        
        assert_eq!(hero.level, 2);
        assert_eq!(hero.hp, hero.max_hp); // HP restored
        assert_eq!(hero.mp, hero.max_mp); // MP restored
    }

    #[test]
    fn test_hero_inventory() {
        let hero = HeroObject::new(1);
        assert_eq!(hero.inventory.len(), 40);
        assert!(!hero.is_inventory_full());
        assert_eq!(hero.find_empty_inventory_slot(), Some(0));
    }
}
