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
use super::stats_ext::StatsExt;  // Import Stats extensions

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
    /// 
    /// Mirrors C# UserObject.Load(S.UserInformation info), lines 63-122
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
        
        // Set location and direction (C# lines 87-89)
        let location = Point::new(info.location_x, info.location_y);
        self.player.map_object.set_current_location(location);
        self.player.map_object.set_map_location(location);
        
        // C# line 90: GameScene.Scene.MapControl.AddObject(this);
        // TODO: Add to map control when scene system is ready
        
        // Set direction and hair (C# lines 92-93)
        self.player.map_object.set_direction(info.direction);
        
        // HP/MP (C# lines 100-101)
        self.hp = info.hp;
        self.mp = info.mp;
        
        // Experience (C# lines 103-104)
        self.experience = info.experience;
        self.max_experience = info.max_experience;
        
        // Level effects (C# line 106)
        // TODO: self.player.level_effects = info.level_effects;
        
        // Load inventory arrays (C# lines 108-110)
        self.inventory = info.inventory.clone().unwrap_or_default();
        self.equipment = info.equipment.clone().unwrap_or_default();
        self.quest_inventory = info.quest_inventory.clone().unwrap_or_default();
        
        // Storage expansion (C# lines 112-113)
        self.has_expanded_storage = info.has_expanded_storage;
        if info.expanded_storage_expiry_time > 0 {
            let duration = std::time::Duration::from_millis(info.expanded_storage_expiry_time as u64);
            self.expanded_storage_expiry_time = Some(std::time::UNIX_EPOCH + duration);
        } else {
            self.expanded_storage_expiry_time = None;
        }
        
        // Load magics (C# lines 115-119)
        self.magics = info.magics.clone();
        // C# line 117-118: Magics[i].CastTime += CMain.Time;
        // Adjust cast times (add current time offset)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        for magic in &mut self.magics {
            if magic.delay > 0 {
                magic.delay += now;
            }
        }
        
        // Load intelligent creatures (C# lines 121-123)
        self.intelligent_creatures = info.intelligent_creatures.clone();
        self.summoned_creature_type = IntelligentCreatureType::try_from(info.summoned_creature_type)
            .unwrap_or(IntelligentCreatureType::None);
        self.creature_summoned = info.creature_summoned;
        
        // Bind items (C# line 125)
        self.bind_all_items();
        
        // Refresh stats (C# line 127)
        self.refresh_stats();
        
        // Set initial action (C# line 129)
        self.set_action();
    }
    
    /// Update inventory and equipment slots from server
    /// 
    /// Mirrors C# UserObject.SetSlots(S.UserSlotsRefresh p), lines 132-139
    pub fn set_slots(&mut self, inventory: Vec<Option<UserItem>>, equipment: Vec<Option<UserItem>>) {
        self.inventory = inventory;
        self.equipment = equipment;
        
        self.bind_all_items();
        self.refresh_stats();
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
    /// 
    /// Mirrors C# UserObject.RefreshStats(), lines 148-171
    pub fn refresh_stats(&mut self) {
        // Clear current stats (C# line 150)
        self.stats = Stats::default();
        
        // Start with level-based stats (C# line 152)
        self.refresh_level_stats();
        
        // Calculate bag weight (C# implicit in RefreshBagWeight)
        self.refresh_bag_weight();
        
        // Add equipment stats (C# line 153)
        self.refresh_equipment_stats();
        
        // Add item set bonuses (C# line 154)
        self.refresh_item_set_stats();
        
        // Add Mir set bonuses (C# line 155)
        self.refresh_mir_set_stats();
        
        // Add skill bonuses (C# line 156)
        self.refresh_skills();
        
        // Add buff stats (C# line 157)
        self.refresh_buffs();
        
        // Add guild buffs (C# line 158)
        self.refresh_guild_buffs();
        
        // Re-call SetLibraries and SetEffects (C# lines 160-161)
        self.player.set_libraries();
        // TODO: self.player.set_effects();
        
        // Apply percentage bonuses (C# lines 163-170)
        self.apply_percentage_bonuses();
        
        // Apply stat caps (C# line 172)
        self.refresh_stat_caps();
        
        // Ensure minimum light level for user (C# line 174)
        // C#: if (this == User && Light < 3) Light = 3;
        // Note: UserObject is always the player, so we always apply this
        if self.player.map_object.light < 3 {
            self.player.map_object.light = 3;
        }
        
        // Calculate attack speed (C# lines 175-176)
        self.calculate_attack_speed();
        
        // Update health percentage (C# line 178)
        let max_hp = self.stats.get_max_hp();
        if max_hp > 0 {
            let percent = ((self.hp as f32 / max_hp as f32) * 100.0) as u8;
            self.player.map_object.set_percent_health(percent);
        }
        
        // Request scene redraw (C# line 180)
        // TODO: GameScene.Scene.Redraw();
    }
    
    /// Apply percentage-based stat bonuses
    /// 
    /// Mirrors C# lines 163-170
    fn apply_percentage_bonuses(&mut self) {
        // HP += (HP * HPRatePercent) / 100
        let hp_bonus = (self.stats.get_max_hp() * self.stats.get_hp_rate_percent()) / 100;
        self.stats.add_max_hp(hp_bonus);
        
        // MP += (MP * MPRatePercent) / 100
        let mp_bonus = (self.stats.get_max_mp() * self.stats.get_mp_rate_percent()) / 100;
        self.stats.add_max_mp(mp_bonus);
        
        // MaxAC += (MaxAC * MaxACRatePercent) / 100
        let ac_bonus = (self.stats.get_max_ac() * self.stats.get_max_ac_rate_percent()) / 100;
        self.stats.add_max_ac(ac_bonus);
        
        // MaxMAC += (MaxMAC * MaxMACRatePercent) / 100
        let mac_bonus = (self.stats.get_max_mac() * self.stats.get_max_mac_rate_percent()) / 100;
        self.stats.add_max_mac(mac_bonus);
        
        // MaxDC += (MaxDC * MaxDCRatePercent) / 100
        let dc_bonus = (self.stats.get_max_dc() * self.stats.get_max_dc_rate_percent()) / 100;
        self.stats.add_max_dc(dc_bonus);
        
        // MaxMC += (MaxMC * MaxMCRatePercent) / 100
        let mc_bonus = (self.stats.get_max_mc() * self.stats.get_max_mc_rate_percent()) / 100;
        self.stats.add_max_mc(mc_bonus);
        
        // MaxSC += (MaxSC * MaxSCRatePercent) / 100
        let sc_bonus = (self.stats.get_max_sc() * self.stats.get_max_sc_rate_percent()) / 100;
        self.stats.add_max_sc(sc_bonus);
        
        // AttackSpeed += (AttackSpeed * AttackSpeedRatePercent) / 100
        let aspd_bonus = (self.stats.get_attack_speed() * self.stats.get_attack_speed_rate_percent()) / 100;
        self.stats.add_attack_speed(aspd_bonus);
    }
    
    /// Apply stat caps
    /// 
    /// Mirrors C# RefreshStatCaps(), lines 665-687
    fn refresh_stat_caps(&mut self) {
        use mir2_shared::enums::Stat;
        
        // C# lines 667-670: Apply custom caps from CoreStats.Caps
        // TODO: Implement when BaseStats system is complete
        // for (stat, cap) in &self.core_stats.caps.values {
        //     let current = self.stats.get(*stat);
        //     self.stats.set(*stat, current.min(*cap));
        // }
        
        // C# lines 672-683: Ensure minimum values (>= 0)
        for stat in [
            Stat::HP, Stat::MP,
            Stat::MinAC, Stat::MaxAC,
            Stat::MinMAC, Stat::MaxMAC,
            Stat::MinDC, Stat::MaxDC,
            Stat::MinMC, Stat::MaxMC,
            Stat::MinSC, Stat::MaxSC,
        ] {
            let value = self.stats.get(stat);
            if value < 0 {
                self.stats.set(stat, 0);
            }
        }
        
        // C# lines 685-687: Ensure Min <= Max for damage stats
        let min_dc = self.stats.get_min_dc();
        let max_dc = self.stats.get_max_dc();
        if min_dc > max_dc {
            self.stats.set(Stat::MinDC, max_dc);
        }
        
        let min_mc = self.stats.get_min_mc();
        let max_mc = self.stats.get_max_mc();
        if min_mc > max_mc {
            self.stats.set(Stat::MinMC, max_mc);
        }
        
        let min_sc = self.stats.get_min_sc();
        let max_sc = self.stats.get_max_sc();
        if min_sc > max_sc {
            self.stats.set(Stat::MinSC, max_sc);
        }
    }
    
    /// Refresh stats based on character level
    /// 
    /// Mirrors C# RefreshLevelStats(), lines 182-189
    fn refresh_level_stats(&mut self) {
        // Reset light (C# line 184)
        self.player.map_object.light = 0;
        
        // C# lines 186-189: foreach (var stat in CoreStats.Stats)
        //                   Stats[stat.Type] = stat.Calculate(Class, Level);
        // CoreStats contains base stat formulas that calculate values based on class/level
        // For now, we use CoreStats directly as it should already contain calculated values
        self.stats = self.core_stats.clone();
    }
    
    /// Refresh bag weight
    /// 
    /// Mirrors C# RefreshBagWeight(), lines 191-202
    fn refresh_bag_weight(&mut self) {
        self.current_bag_weight = 0;
        
        // C# lines 195-200: CurrentBagWeight += item.Weight
        // Note: C# UserItem.Weight property = Info.Weight * Count (except Amulet/Bait)
        // Rust weight() method implements the same logic internally
        for slot in &self.inventory {
            if let Some(item) = slot {
                self.current_bag_weight += item.weight(None) as i32;
            }
        }
    }
    
    /// Refresh equipment stats
    /// 
    /// Mirrors C# RefreshEquipmentStats(), lines 204-296
    fn refresh_equipment_stats(&mut self) {
        // C# lines 206-215: Reset equipment-related fields
        // Weapon = -1; WeaponEffect = 0; Armour = 0; WingEffect = 0;
        // MountType = -1; CurrentWearWeight = 0; CurrentHandWeight = 0;
        // ItemMode = SpecialItemMode.None; FastRun = false;
        self.current_wear_weight = 0;
        self.current_hand_weight = 0;
        
        // C# lines 217-218: Clear item set tracking
        // ItemSets.Clear(); MirSet.Clear();
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
    /// 
    /// Mirrors C# RefreshItemSetStats(), lines 349-540
    fn refresh_item_set_stats(&mut self) {
        use mir2_shared::enums::{ItemSet, ItemType};
        
        // Special bonuses that should only apply once per set
        let mut has_smash_set_bonus = false;
        let mut has_purity_set_bonus = false;
        let mut has_hwan_devil_set_bonus = false;
        
        for item_set in &self.item_sets {
            let set = item_set.set;
            let types = &item_set.types;
            
            // C# lines 355-389: Special 2-piece bonuses (Ring + Bracelet combinations)
            if set == ItemSet::Smash && types.contains(&ItemType::Ring) && types.contains(&ItemType::Bracelet) {
                if !has_smash_set_bonus {
                    self.stats.add_attack_speed(2);
                    has_smash_set_bonus = true;
                }
            }
            
            if set == ItemSet::Purity && types.contains(&ItemType::Ring) && types.contains(&ItemType::Bracelet) {
                if !has_purity_set_bonus {
                    self.stats.add_holy(3);
                    has_purity_set_bonus = true;
                }
            }
            
            if set == ItemSet::HwanDevil && types.contains(&ItemType::Ring) && types.contains(&ItemType::Bracelet) {
                if !has_hwan_devil_set_bonus {
                    self.stats.add_wear_weight(5);
                    self.stats.add_bag_weight(20);
                    has_hwan_devil_set_bonus = true;
                }
            }
            
            // DarkGhost: Necklace + Bracelet bonus (C# lines 388-391)
            if set == ItemSet::DarkGhost && types.contains(&ItemType::Necklace) && types.contains(&ItemType::Bracelet) {
                self.stats.add_max_hp(25);
            }
            
            // Skip incomplete sets (C# line 393)
            if !item_set.is_complete() {
                continue;
            }
            
            // C# lines 395-538: Complete set bonuses
            match set {
                ItemSet::Mundane => {
                    self.stats.add_max_hp(50);
                }
                ItemSet::NokChi => {
                    self.stats.add_max_mp(50);
                }
                ItemSet::TaoProtect => {
                    self.stats.add_max_hp(30);
                    self.stats.add_max_mp(30);
                }
                ItemSet::RedOrchid => {
                    self.stats.add_accuracy(2);
                }
                ItemSet::RedFlower => {
                    self.stats.add_max_hp(50);
                    self.stats.add_max_mp(-50);  // Reduces MP
                }
                ItemSet::Smash => {
                    self.stats.add_min_dc(1);
                    self.stats.add_max_dc(3);
                }
                ItemSet::HwanDevil => {
                    self.stats.add_min_mc(1);
                    self.stats.add_max_mc(2);
                }
                ItemSet::Purity => {
                    self.stats.add_min_sc(1);
                    self.stats.add_max_sc(2);
                }
                ItemSet::FiveString => {
                    // HP += (HP / 100) * 30
                    let hp_bonus = (self.stats.get_max_hp() / 100) * 30;
                    self.stats.add_max_hp(hp_bonus);
                    self.stats.add_min_ac(2);
                    self.stats.add_max_ac(2);
                }
                ItemSet::Spirit => {
                    self.stats.add_min_dc(2);
                    self.stats.add_max_dc(5);
                    self.stats.add_attack_speed(2);
                }
                ItemSet::Bone => {
                    self.stats.add_max_ac(2);
                    self.stats.add_max_mc(1);
                    self.stats.add_max_sc(1);
                }
                ItemSet::Bug => {
                    self.stats.add_max_dc(1);
                    self.stats.add_max_mc(1);
                    self.stats.add_max_sc(1);
                    self.stats.add_max_mac(1);
                    self.stats.add_poison_resist(1);
                }
                ItemSet::WhiteGold => {
                    self.stats.add_max_dc(2);
                    self.stats.add_max_ac(2);
                }
                ItemSet::WhiteGoldH => {
                    self.stats.add_max_dc(3);
                    self.stats.add_max_hp(30);
                    self.stats.add_attack_speed(2);
                }
                ItemSet::RedJade => {
                    self.stats.add_max_mc(2);
                    self.stats.add_max_mac(2);
                }
                ItemSet::RedJadeH => {
                    self.stats.add_max_mc(2);
                    self.stats.add_max_mp(40);
                    self.stats.add_agility(2);
                }
                ItemSet::Nephrite => {
                    self.stats.add_max_sc(2);
                    self.stats.add_max_ac(1);
                    self.stats.add_max_mac(1);
                }
                ItemSet::NephriteH => {
                    self.stats.add_max_sc(2);
                    self.stats.add_max_hp(15);
                    self.stats.add_max_mp(20);
                    self.stats.add_holy(1);
                    self.stats.add_accuracy(1);
                }
                ItemSet::Whisker1 => {
                    self.stats.add_max_dc(1);
                    self.stats.add_bag_weight(25);
                }
                ItemSet::Whisker2 => {
                    self.stats.add_max_mc(1);
                    self.stats.add_bag_weight(17);
                }
                ItemSet::Whisker3 => {
                    self.stats.add_max_sc(1);
                    self.stats.add_bag_weight(17);
                }
                ItemSet::Whisker4 => {
                    self.stats.add_max_dc(1);
                    self.stats.add_bag_weight(20);
                }
                ItemSet::Whisker5 => {
                    self.stats.add_max_dc(1);
                    self.stats.add_bag_weight(17);
                }
                ItemSet::Hyeolryong => {
                    self.stats.add_max_sc(2);
                    self.stats.add_max_hp(15);
                    self.stats.add_max_mp(20);
                    self.stats.add_holy(1);
                    self.stats.add_accuracy(1);
                }
                ItemSet::Monitor => {
                    self.stats.add_magic_resist(1);
                    self.stats.add_poison_resist(1);
                }
                ItemSet::Oppressive => {
                    self.stats.add_max_ac(1);
                    self.stats.add_agility(1);
                }
                ItemSet::BlueFrost => {
                    self.stats.add_min_dc(1);
                    self.stats.add_max_dc(1);
                    self.stats.add_min_mc(1);
                    self.stats.add_max_mc(1);
                    self.stats.add_hand_weight(1);
                    self.stats.add_wear_weight(2);
                }
                ItemSet::BlueFrostH => {
                    self.stats.add_min_dc(1);
                    self.stats.add_max_dc(2);
                    self.stats.add_max_mc(2);
                    self.stats.add_accuracy(1);
                    self.stats.add_max_hp(50);
                }
                ItemSet::DarkGhost => {
                    self.stats.add_max_mp(25);
                    self.stats.add_attack_speed(2);
                }
                _ => {
                    // Other sets not implemented or don't have bonuses
                }
            }
        }
    }
    
    /// Refresh Mir set bonuses
    /// 
    /// Mirrors C# RefreshMirSetStats(), lines 542-596
    fn refresh_mir_set_stats(&mut self) {
        use mir2_shared::enums::EquipmentSlot;
        
        let mir_count = self.mir_set.len();
        
        // C# lines 544-555: Full 10-piece set bonus
        if mir_count == 10 {
            self.stats.add_max_ac(1);
            self.stats.add_max_mac(1);
            self.stats.add_bag_weight(70);
            self.stats.add_luck(2);
            self.stats.add_attack_speed(2);
            self.stats.add_max_hp(70);
            self.stats.add_max_mp(80);
            self.stats.add_magic_resist(6);
            self.stats.add_poison_resist(6);
        }
        
        // C# lines 557-564: Ring pair bonus
        if self.mir_set.contains(&EquipmentSlot::RingL) && self.mir_set.contains(&EquipmentSlot::RingR) {
            self.stats.add_max_mac(1);
            self.stats.add_max_ac(1);
        }
        
        // C# lines 565-569: Bracelet pair bonus
        if self.mir_set.contains(&EquipmentSlot::BraceletL) && self.mir_set.contains(&EquipmentSlot::BraceletR) {
            self.stats.add_min_ac(1);
            self.stats.add_min_mac(1);
        }
        
        // C# lines 570-576: Ring/Bracelet + Necklace combo
        let has_ring = self.mir_set.contains(&EquipmentSlot::RingL) || self.mir_set.contains(&EquipmentSlot::RingR);
        let has_bracelet = self.mir_set.contains(&EquipmentSlot::BraceletL) || self.mir_set.contains(&EquipmentSlot::BraceletR);
        if has_ring && has_bracelet && self.mir_set.contains(&EquipmentSlot::Necklace) {
            self.stats.add_max_mac(1);
            self.stats.add_max_ac(1);
            self.stats.add_bag_weight(30);
            self.stats.add_wear_weight(17);
        }
        
        // C# lines 577-583: Full jewelry set (both rings + both bracelets + necklace)
        if self.mir_set.contains(&EquipmentSlot::RingL) 
            && self.mir_set.contains(&EquipmentSlot::RingR)
            && self.mir_set.contains(&EquipmentSlot::BraceletL)
            && self.mir_set.contains(&EquipmentSlot::BraceletR)
            && self.mir_set.contains(&EquipmentSlot::Necklace) {
            self.stats.add_max_mac(1);
            self.stats.add_max_ac(1);
            self.stats.add_bag_weight(20);
            self.stats.add_wear_weight(10);
        }
        
        // C# lines 584-590: Armour + Helmet + Weapon combo
        if self.mir_set.contains(&EquipmentSlot::Armour)
            && self.mir_set.contains(&EquipmentSlot::Helmet)
            && self.mir_set.contains(&EquipmentSlot::Weapon) {
            self.stats.add_max_dc(2);
            self.stats.add_max_mc(1);
            self.stats.add_max_sc(1);
            self.stats.add_agility(1);
        }
        
        // C# lines 591-596: Armour + Boots + Belt combo
        if self.mir_set.contains(&EquipmentSlot::Armour)
            && self.mir_set.contains(&EquipmentSlot::Boots)
            && self.mir_set.contains(&EquipmentSlot::Belt) {
            self.stats.add_max_dc(1);
            self.stats.add_max_mc(1);
            self.stats.add_max_sc(1);
            self.stats.add_hand_weight(17);
        }
        
        // C# lines 597-605: Full armor + weapon set (5 pieces)
        if self.mir_set.contains(&EquipmentSlot::Armour)
            && self.mir_set.contains(&EquipmentSlot::Boots)
            && self.mir_set.contains(&EquipmentSlot::Belt)
            && self.mir_set.contains(&EquipmentSlot::Helmet)
            && self.mir_set.contains(&EquipmentSlot::Weapon) {
            self.stats.add_min_dc(1);
            self.stats.add_max_dc(1);
            self.stats.add_min_mc(1);
            self.stats.add_max_mc(1);
            self.stats.add_min_sc(1);
            self.stats.add_max_sc(1);
            self.stats.add_hand_weight(17);
        }
    }
    
    /// Refresh guild buff bonuses
    /// 
    /// Mirrors C# RefreshGuildBuffs(), lines 645-663
    fn refresh_guild_buffs(&mut self) {
        // TODO: Implement when guild system is ready
        // C# checks GameScene.Scene.GuildDialog.EnabledBuffs
        // and adds each active buff's stats to the player
        // 
        // for (int i = 0; i < GameScene.Scene.GuildDialog.EnabledBuffs.Count; i++)
        // {
        //     GuildBuff buff = GameScene.Scene.GuildDialog.EnabledBuffs[i];
        //     if (buff == null) continue;
        //     if (!buff.Active) continue;
        //     Stats.Add(buff.Info.Stats);
        // }
    }
    
    /// Refresh skill bonuses
    /// 
    /// Mirrors C# RefreshSkills(), lines 607-628
    fn refresh_skills(&mut self) {
        use mir2_shared::enums::Spell;
        
        // C# lines 609-610: Lookup tables for skill bonuses
        const SPIRIT_SWORD_LV_PLUS: [i32; 4] = [0, 3, 5, 8];
        const SLAYING_LV_PLUS: [i32; 4] = [5, 6, 7, 8];
        
        for magic in &self.magics {
            let level = magic.level as usize;
            
            match magic.spell {
                // C# lines 614-617: Fencing (Accuracy bonus)
                Spell::Fencing => {
                    self.stats.add_accuracy((magic.level as i32) * 3);
                    // C# also has commented out: Stats[Stat.MaxAC] += (magic.Level + 1) * 3;
                }
                
                // C# lines 618-622: Slaying (Accuracy + MaxDC)
                Spell::Slaying => {
                    self.stats.add_accuracy(magic.level as i32);
                    if level < SLAYING_LV_PLUS.len() {
                        self.stats.add_max_dc(SLAYING_LV_PLUS[level]);
                    }
                }
                
                // C# lines 623-626: SpiritSword (Accuracy bonus)
                Spell::SpiritSword => {
                    if level < SPIRIT_SWORD_LV_PLUS.len() {
                        self.stats.add_accuracy(SPIRIT_SWORD_LV_PLUS[level]);
                    }
                    // C# also has commented out:
                    // Stats[Stat.MaxDC] += (int)(Stats[Stat.MaxSC] * (magic.Level + 1) * 0.1F);
                }
                
                _ => {
                    // Other spells don't provide passive stat bonuses
                }
            }
        }
    }
    
    /// Refresh buff stats
    /// 
    /// Mirrors C# RefreshBuffs(), lines 630-643
    fn refresh_buffs(&mut self) {
        // TODO: Implement when buff system is complete
        // C# iterates through BuffDialog.Buffs
        // 
        // for (int i = 0; i < dialog.Buffs.Count; i++)
        // {
        //     ClientBuff buff = dialog.Buffs[i];
        //     Stats.Add(buff.Stats);
        //     
        //     switch (buff.Type)
        //     {
        //         case BuffType.SwiftFeet:
        //             Sprint = true;
        //             break;
        //         case BuffType.Transform:
        //             if (buff.Paused) continue;
        //             TransformType = (short)buff.Values[0];
        //             FastRun = true;
        //             break;
        //     }
        // }
        
        // Note: MapObject.buffs only tracks BuffType, not full ClientBuff with Stats
        // Need BuffDialog integration to get full buff data
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
    /// 
    /// Note: C# UserItem.Weight = Info.Weight * Count (except Amulet/Bait)
    /// Rust weight() method implements the same logic
    pub fn calculate_bag_weight(&self) -> i32 {
        let mut weight = 0;
        for slot in &self.inventory {
            if let Some(item) = slot {
                weight += item.weight(None) as i32;
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
