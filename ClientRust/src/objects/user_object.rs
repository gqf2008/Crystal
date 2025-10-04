// UserObject.rs - Player character object (the user)
// Mirrors Client/MirObjects/UserObject.cs

use mir2_shared::{packets::*,
    data::{
        stats::Stats, 
        client_data::{ClientMagic, ClientIntelligentCreature, ClientQuestProgress, ClientMail},
        item::ItemSets,  // C# Shared/Data/ItemData.cs ItemSets
    },
    enums::{MirDirection, MirAction, Spell, SpecialItemMode, EquipmentSlot, IntelligentCreatureType, MirClass, MirGender},
    Point, UserItem,
};

use super::player_object::PlayerObject;

/// User object - represents the current player
/// 
/// Architecture: UserObject composes PlayerObject (which composes MapObject)
/// This mirrors C# inheritance: UserObject : PlayerObject : MapObject
#[derive(Debug, Clone)]
pub struct UserObject {
    // ==================== PlayerObject Composition ====================
    /// Player object containing all player-specific fields and methods
    /// Includes: appearance, animation, spell casting, drawing, etc.
    pub player: PlayerObject,
    
    // ==================== UserObject Specific Fields ====================
    
    /// User ID (different from ObjectID in MapObject)
    pub id: u32,
    
    /// Current HP
    pub hp: i32,
    
    /// Current MP
    pub mp: i32,
    
    /// Attack speed
    pub attack_speed: i32,
    
    /// Current stats (after equipment)
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
    pub item_sets: Vec<ItemSets>,  // C#: ItemSets (Shared/Data/ItemData.cs)
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

// Note: All data types imported from mir2_shared to maintain consistency:
//   - ClientMagic, ClientIntelligentCreature, ClientQuestProgress, ClientMail (client_data)
//   - EquipmentSlot, IntelligentCreatureType, MirAction (enums)
//   - ItemSetStatus (item) - corresponds to C# ItemSets in Shared/Data/ItemData.cs

/// Queued action for delayed execution
/// Mirrors C#: Client/MirObjects/PlayerObject.cs QueuedAction class
#[derive(Debug, Clone)]
pub struct QueuedAction {
    pub action: MirAction,      // C# uses MirAction, not a separate enum
    pub location: Point,
    pub direction: MirDirection,
    // pub params: Vec<Box<dyn Any>>,  // C#: List<object> Params
    // Note: C# rarely uses Params field, so we omit it for now
    // Can be added later when needed with an enum for type-safe params
}

impl UserObject {
    /// Create a new user object
    pub fn new(object_id: u32) -> Self {
        // Create player object with default values
        // Actual values will be set by load() method when server data arrives
        let player = PlayerObject::new(
            object_id,
            String::new(),
            MirClass::Warrior,  // Default, will be set by load()
            MirGender::Male,    // Default, will be set by load()
        );
        
        Self {
            player,
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
            item_mode: SpecialItemMode::NONE,
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
        
        // Set PlayerObject fields
        self.player.map_object.set_name(info.name.clone());
        self.player.map_object.set_name_colour_argb(info.name_colour);
        self.player.level = info.level;
        self.player.guild_name = info.guild_name.clone();
        self.player.guild_rank_name = info.guild_rank.clone();
        self.player.class = info.class;
        self.player.gender = info.gender;
        self.player.hair = info.hair;
        
        // Set location and direction
        let location = Point::new(info.location_x, info.location_y);
        self.player.map_object.set_location(location);
        self.player.map_object.set_direction(info.direction);
        
        self.hp = info.hp;
        self.mp = info.mp;
        
        self.experience = info.experience;
        self.max_experience = info.max_experience;
        
        // Load inventory arrays
        self.inventory = info.inventory.clone().unwrap_or_default();
        self.equipment = info.equipment.clone().unwrap_or_default();
        self.quest_inventory = info.quest_inventory.clone().unwrap_or_default();
        
        self.has_expanded_storage = info.has_expanded_storage;
        // Convert i64 milliseconds to SystemTime
        if info.expanded_storage_expiry_time > 0 {
            let duration = std::time::Duration::from_millis(info.expanded_storage_expiry_time as u64);
            self.expanded_storage_expiry_time = Some(std::time::UNIX_EPOCH + duration);
        } else {
            self.expanded_storage_expiry_time = None;
        }
        
        // Load magics
        self.magics = info.magics.clone();
        // Adjust cooldown times (add current time)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        for magic in &mut self.magics {
            if magic.delay > 0 {
                magic.delay += now;
            }
        }
        
        // Load intelligent creatures
        self.summoned_creature_type = IntelligentCreatureType::try_from(info.summoned_creature_type)
            .unwrap_or(IntelligentCreatureType::None);
        self.creature_summoned = info.creature_summoned;
        
        // Bind items (associate with ItemInfo - TODO: implement fully)
        self.bind_all_items();
        
        // Refresh stats
        self.refresh_stats();
        
        // Set initial action
        self.set_action();
    }

    /// Bind all items to their ItemInfo (associate item data)
    fn bind_all_items(&mut self) {
        // TODO: Implement item binding
        // In C#, this looks up items in GameScene.ItemInfoList
        // For now, we skip this as items already have their info embedded
        // This will be needed when we have a centralized ItemInfo registry
    }
    
    /// Set initial action (standing pose)
    fn set_action(&mut self) {
        // Set to standing action based on current direction
        // This will be implemented when MapObject's action system is complete
        // For now, just ensure the object is in a valid state
        let direction = self.player.map_object.direction();
        self.player.map_object.set_direction(direction);
    }
    
    /// Update user stats (called after equipment/buff changes)
    pub fn refresh_stats(&mut self) {
        // Clear current stats
        self.stats = Stats::default();
        
        // Start with level-based stats
        self.refresh_level_stats();
        
        // Add equipment stats
        self.refresh_equipment_stats();
        
        // Add item set bonuses
        self.refresh_item_set_stats();
        
        // Add skill bonuses
        self.refresh_skills();
        
        // Add buff stats
        self.refresh_buffs();
        
        // TODO: Add guild buffs
        // self.refresh_guild_buffs();
        
        // TODO: Apply percentage bonuses
        // Stats[HP] += (Stats[HP] * Stats[HPRatePercent]) / 100
        
        // TODO: Apply stat caps
        // self.refresh_stat_caps();
        
        // Calculate attack speed
        self.calculate_attack_speed();
    }
    
    /// Refresh stats based on character level
    fn refresh_level_stats(&mut self) {
        // TODO: Calculate level-based stats from CoreStats
        // For now, use CoreStats directly
        // In C#: Stats[stat.Type] = stat.Calculate(Class, Level)
        self.stats = self.core_stats.clone();
    }
    
    /// Refresh equipment stats
    fn refresh_equipment_stats(&mut self) {
        self.current_wear_weight = 0;
        self.current_hand_weight = 0;
        
        // Clear equipment-specific fields
        // TODO: Set weapon, armour, mount type, etc.
        
        for slot in &self.equipment {
            if let Some(item) = slot {
                // Add weight
                let weight = item.weight(None) as i32;
                // TODO: Distinguish hand weight vs wear weight based on item type
                self.current_wear_weight += weight;
                
                // Add item stats
                // TODO: self.stats.add(&item.stats);
                // TODO: self.stats.add(&item.added_stats);
                
                // TODO: Handle durability check (skip if dura == 0)
                // TODO: Handle awakening stats
                // TODO: Handle sockets
                // TODO: Track item sets
            }
        }
    }
    
    /// Refresh item set bonuses
    fn refresh_item_set_stats(&mut self) {
        // TODO: Implement item set bonus system
        // Check which set pieces are equipped
        // Apply set bonuses based on item count
    }
    
    /// Refresh skill bonuses
    fn refresh_skills(&mut self) {
        // TODO: Implement skill stat bonuses
        // Some skills provide passive stat increases
    }
    
    /// Refresh buff stats
    fn refresh_buffs(&mut self) {
        // TODO: Iterate through active buffs
        // Add stats based on buff type
        for _buff in self.player.map_object.buffs() {
            // match buff.buff_type {
            //     BuffType::某种Buff => self.stats.add_hp(value),
            //     ...
            // }
        }
    }
    
    /// Calculate attack speed based on stats and level
    fn calculate_attack_speed(&mut self) {
        // C#: AttackSpeed = 1400 - ((Stats[Stat.AttackSpeed] * 60) + Math.Min(370, (Level * 14)));
        // if (AttackSpeed < 550) AttackSpeed = 550;
        
        let attack_speed_stat = 0; // TODO: self.stats.get(StatType::AttackSpeed);
        let speed = 1400 - (attack_speed_stat * 60 + std::cmp::min(370, self.player.level as i32 * 14));
        self.attack_speed = std::cmp::max(550, speed);
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
                .as_millis() as i64;
            return now < magic.delay;
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
                weight += item.weight(None) as i32 * item.count as i32;
            }
        }
        weight
    }

    /// Calculate total equipment weight
    pub fn calculate_equipment_weight(&self) -> i32 {
        let mut weight = 0;
        for slot in &self.equipment {
            if let Some(item) = slot {
                weight += item.weight(None) as i32;
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
        
        // Check for level up
        while self.experience >= self.max_experience && self.max_experience > 0 {
            self.level_up();
        }
    }

    /// Check if can level up
    pub fn can_level_up(&self) -> bool {
        self.experience >= self.max_experience && self.max_experience > 0
    }
    
    /// Level up the character
    fn level_up(&mut self) {
        self.player.level += 1;
        self.experience -= self.max_experience;
        
        // TODO: Calculate new max_experience based on level
        // TODO: Play level up effects
        // TODO: Show level up message
        // TODO: Update stats
        
        // For now, just refresh stats
        self.refresh_stats();
    }
    
    // ==================== Delegation Methods to PlayerObject ====================
    
    /// Get current level (delegates to PlayerObject)
    pub fn level(&self) -> u16 {
        self.player.level
    }
    
    /// Get class (delegates to PlayerObject)
    pub fn class(&self) -> MirClass {
        self.player.class
    }
    
    /// Get gender (delegates to PlayerObject)
    pub fn gender(&self) -> MirGender {
        self.player.gender
    }
    
    /// Get guild name (delegates to PlayerObject)
    pub fn guild_name(&self) -> &str {
        &self.player.guild_name
    }
    
    /// Get guild rank name (delegates to PlayerObject)
    pub fn guild_rank_name(&self) -> &str {
        &self.player.guild_rank_name
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
    pub fn direction(&self) -> MirDirection {
        self.player.map_object.direction()
    }
    
    /// Draw the user (delegates to PlayerObject)
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
    /// Updates class, gender, armour, weapon fields and recalculates offsets
    pub fn set_libraries(&mut self) {
        self.player.set_libraries();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_object_creation() {
        let user = UserObject::new(1);
        assert_eq!(user.player.map_object.object_id(), 1);
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
