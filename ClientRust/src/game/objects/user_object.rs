// UserObject.rs - Player character object (the user)
// Mirrors Client/MirObjects/UserObject.cs

use mir2_shared::{
    enums::{MirClass, MirDirection, MirGender, Spell},
    stats::Stats,
    Point, UserItem,
};

use super::map_object::MapObject;
use crate::network::network::protocol::UserInformation;

/// Special item mode for UI interaction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialItemMode {
    None,
    Parcel,
}

/// User object - represents the current player
#[derive(Debug, Clone)]
pub struct UserObject {
    // Inherited from MapObject
    pub map_object: MapObject,
    
    // UserObject specific fields
    pub id: u32,
    pub hp: i32,
    pub mp: i32,
    pub attack_speed: i32,
    pub stats: Stats,
    
    // Weight tracking
    pub current_hand_weight: i32,
    pub current_wear_weight: i32,
    pub current_bag_weight: i32,
    
    // Experience
    pub experience: i64,
    pub max_experience: i64,
    
    // Trading
    pub trade_locked: bool,
    pub trade_gold_amount: u32,
    pub allow_trade: bool,
    
    // Rental system
    pub rental_gold_locked: bool,
    pub rental_item_locked: bool,
    pub rental_gold_amount: u32,
    
    pub item_mode: SpecialItemMode,
    
    // Core stats (base stats before equipment)
    pub core_stats: Stats,
    
    // Inventory arrays
    pub inventory: Vec<Option<UserItem>>,      // 46 slots
    pub equipment: Vec<Option<UserItem>>,      // 14 slots
    pub trade: Vec<Option<UserItem>>,          // 10 slots
    pub quest_inventory: Vec<Option<UserItem>>, // 40 slots
    
    // Belt configuration
    pub belt_idx: i32,
    pub hero_belt_idx: i32,
    
    // Storage expansion
    pub has_expanded_storage: bool,
    pub expanded_storage_expiry_time: Option<std::time::SystemTime>,
    
    // Magic/Skills
    pub magics: Vec<ClientMagic>,
    pub item_sets: Vec<ItemSets>,
    pub mir_set: Vec<EquipmentSlot>,
    
    // Intelligent creatures (pets)
    pub intelligent_creatures: Vec<ClientIntelligentCreature>,
    pub summoned_creature_type: IntelligentCreatureType,
    pub creature_summoned: bool,
    pub pearl_count: i32,
    
    // Quests
    pub current_quests: Vec<ClientQuestProgress>,
    pub completed_quests: Vec<i32>,
    
    // Mail system
    pub mail: Vec<ClientMail>,
    
    // Combat skills status
    pub slaying: bool,
    pub thrusting: bool,
    pub half_moon: bool,
    pub cross_half_moon: bool,
    pub double_slash: bool,
    pub twin_drake_blade: bool,
    pub flaming_sword: bool,
    
    // Next magic to cast
    pub next_magic: Option<ClientMagic>,
    pub next_magic_location: Point,
    pub next_magic_object: Option<u32>, // Object ID
    pub next_magic_direction: MirDirection,
    
    // Queued action
    pub queued_action: Option<QueuedAction>,
}

/// Client magic data
#[derive(Debug, Clone)]
pub struct ClientMagic {
    pub spell: Spell,
    pub key: u8,
    pub level: u8,
    pub experience: u16,
    pub cooldown: u64,
}

/// Item set information
#[derive(Debug, Clone)]
pub struct ItemSets {
    pub set_id: i32,
    pub set_name: String,
    pub parts_equipped: i32,
    pub full_set: bool,
}

/// Equipment slot enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipmentSlot {
    Weapon,
    Armour,
    Helmet,
    Torch,
    Necklace,
    BraceletL,
    BraceletR,
    RingL,
    RingR,
    Amulet,
    Belt,
    Boots,
    Stone,
    Mount,
}

/// Intelligent creature (pet) data
#[derive(Debug, Clone)]
pub struct ClientIntelligentCreature {
    pub creature_type: IntelligentCreatureType,
    pub pet_name: String,
    pub level: u16,
    pub hp: i32,
    pub max_hp: i32,
    pub hunger: i32,
    pub summoned: bool,
}

/// Intelligent creature types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntelligentCreatureType {
    None,
    BabyPig,
    Chick,
    Kitten,
    BabySkeleton,
    Baekdon,
    Wimaen,
    BlackKitten,
    BabyDragon,
    OlympicFlame,
    BabySnowMan,
    Frog,
    BabyMonkey,
    AngryBird,
    Foxey,
}

/// Quest progress tracking
#[derive(Debug, Clone)]
pub struct ClientQuestProgress {
    pub quest_id: i32,
    pub quest_name: String,
    pub quest_group: String,
    pub task_type: String,
    pub current_count: i32,
    pub max_count: i32,
}

/// Mail message
#[derive(Debug, Clone)]
pub struct ClientMail {
    pub mail_id: i64,
    pub sender_name: String,
    pub subject: String,
    pub message: String,
    pub date_sent: std::time::SystemTime,
    pub opened: bool,
    pub gold: u32,
    pub items: Vec<UserItem>,
}

/// Queued action for delayed execution
#[derive(Debug, Clone)]
pub struct QueuedAction {
    pub action_type: QueuedActionType,
    pub location: Point,
    pub direction: MirDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueuedActionType {
    Move,
    Attack,
    Spell,
    Harvest,
}

impl UserObject {
    /// Create a new user object
    pub fn new(object_id: u32) -> Self {
        Self {
            map_object: MapObject::new_player(object_id),
            id: 0,
            hp: 0,
            mp: 0,
            attack_speed: 0,
            stats: Stats::default(),
            current_hand_weight: 0,
            current_wear_weight: 0,
            current_bag_weight: 0,
            experience: 0,
            max_experience: 0,
            trade_locked: false,
            trade_gold_amount: 0,
            allow_trade: false,
            rental_gold_locked: false,
            rental_item_locked: false,
            rental_gold_amount: 0,
            item_mode: SpecialItemMode::None,
            core_stats: Stats::default(),
            inventory: vec![None; 46],
            equipment: vec![None; 14],
            trade: vec![None; 10],
            quest_inventory: vec![None; 40],
            belt_idx: 6,
            hero_belt_idx: 2,
            has_expanded_storage: false,
            expanded_storage_expiry_time: None,
            magics: Vec::new(),
            item_sets: Vec::new(),
            mir_set: Vec::new(),
            intelligent_creatures: Vec::new(),
            summoned_creature_type: IntelligentCreatureType::None,
            creature_summoned: false,
            pearl_count: 0,
            current_quests: Vec::new(),
            completed_quests: Vec::new(),
            mail: Vec::new(),
            slaying: false,
            thrusting: false,
            half_moon: false,
            cross_half_moon: false,
            double_slash: false,
            twin_drake_blade: false,
            flaming_sword: false,
            next_magic: None,
            next_magic_location: Point::new(0, 0),
            next_magic_object: None,
            next_magic_direction: MirDirection::Up,
            queued_action: None,
        }
    }

    /// Load user information from server
    pub fn load(&mut self, info: &UserInformation) {
        self.id = info.real_id;
        self.map_object.name = info.name.clone();
        self.map_object.name_colour = info.name_colour;
        self.map_object.guild_name = Some(info.guild_name.clone());
        self.map_object.guild_rank_name = Some(info.guild_rank.clone());
        
        self.map_object.class = info.class;
        self.map_object.gender = info.gender;
        self.map_object.level = info.level;
        
        self.map_object.current_location = info.location;
        self.map_object.map_location = info.location;
        
        self.map_object.direction = info.direction;
        self.map_object.hair = info.hair;
        
        self.hp = info.hp;
        self.mp = info.mp;
        
        self.experience = info.experience;
        self.max_experience = info.max_experience;
        
        // Load inventory arrays
        self.inventory = info.inventory.clone();
        self.equipment = info.equipment.clone();
        self.quest_inventory = info.quest_inventory.clone();
        
        self.has_expanded_storage = info.has_expanded_storage;
        self.expanded_storage_expiry_time = info.expanded_storage_expiry_time;
        
        // TODO: Load magics, item sets, etc. from info
    }

    /// Update user stats (called after equipment/buff changes)
    pub fn refresh_stats(&mut self) {
        // Start with core stats
        let mut new_stats = self.core_stats;
        
        // Add equipment stats
        for slot in &self.equipment {
            if let Some(item) = slot {
                // TODO: Add item stats to new_stats
            }
        }
        
        // Add buff stats
        for buff in &self.map_object.buffs {
            // TODO: Add buff stats
        }
        
        self.stats = new_stats;
    }

    /// Get magic by spell type
    pub fn get_magic(&self, spell: Spell) -> Option<&ClientMagic> {
        self.magics.iter().find(|m| m.spell == spell)
    }

    /// Check if magic is on cooldown
    pub fn magic_on_cooldown(&self, spell: Spell) -> bool {
        if let Some(magic) = self.get_magic(spell) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            return now < magic.cooldown;
        }
        false
    }

    /// Get item from inventory by slot
    pub fn get_inventory_item(&self, slot: usize) -> Option<&UserItem> {
        self.inventory.get(slot).and_then(|item| item.as_ref())
    }

    /// Get equipment item by slot
    pub fn get_equipment_item(&self, slot: EquipmentSlot) -> Option<&UserItem> {
        let index = slot as usize;
        self.equipment.get(index).and_then(|item| item.as_ref())
    }

    /// Calculate total bag weight
    pub fn calculate_bag_weight(&self) -> i32 {
        let mut weight = 0;
        for slot in &self.inventory {
            if let Some(item) = slot {
                weight += item.weight as i32 * item.count as i32;
            }
        }
        weight
    }

    /// Calculate total equipment weight
    pub fn calculate_equipment_weight(&self) -> i32 {
        let mut weight = 0;
        for slot in &self.equipment {
            if let Some(item) = slot {
                weight += item.weight as i32;
            }
        }
        weight
    }

    /// Check if inventory is full
    pub fn is_inventory_full(&self) -> bool {
        self.inventory.iter().all(|slot| slot.is_some())
    }

    /// Find empty inventory slot
    pub fn find_empty_inventory_slot(&self) -> Option<usize> {
        self.inventory.iter().position(|slot| slot.is_none())
    }

    /// Update experience
    pub fn gain_experience(&mut self, amount: i64) {
        self.experience += amount;
        // TODO: Check for level up
    }

    /// Check if can level up
    pub fn can_level_up(&self) -> bool {
        self.experience >= self.max_experience
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_object_creation() {
        let user = UserObject::new(1);
        assert_eq!(user.map_object.object_id, 1);
        assert_eq!(user.inventory.len(), 46);
        assert_eq!(user.equipment.len(), 14);
    }

    #[test]
    fn test_inventory_operations() {
        let user = UserObject::new(1);
        assert!(user.is_inventory_full() == false);
        assert_eq!(user.find_empty_inventory_slot(), Some(0));
    }
}
