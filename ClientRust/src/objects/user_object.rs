// UserObject.rs - Player character object (the user)
// Mirrors Client/MirObjects/UserObject.cs

use std::convert::TryFrom;

use ggez::{Context, GameResult};
use ggez::graphics::Canvas;
use mir2_shared::{
    data::{
        client_data::{ClientIntelligentCreature, ClientMail, ClientMagic, ClientQuestProgress},
        item::ItemSets,
        stats::Stats,
    },
    enums::{
        EquipmentSlot, IntelligentCreatureType, ItemSet, ItemType, MirAction, MirClass,
        MirDirection, MirGender, SpecialItemMode, Spell, Stat,
    },
    packets::*,
    Point, UserItem,
};

use super::player_object::{PlayerObject, QueuedAction};
use super::stats_ext::StatsExt;  // Import Stats extensions
use super::drawable::DrawableMapObject;

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
    
    // Currency
    pub gold: u32,
    pub credit: u32,
    
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
// QueuedAction is defined in player_object.rs (mirrors C# PlayerObject.QueuedAction)

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
        
        Self::new_from_player(player)
    }
    
    /// Create UserObject from an existing PlayerObject
    /// Used by ObjectFactory when creating from server packets
    pub fn new_from_player(player: PlayerObject) -> Self {
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
            gold: 0,
            credit: 0,
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
        // TODO: Implement when IntelligentCreatures is added to UserInformation packet
        // self.intelligent_creatures = info.intelligent_creatures.clone();
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
    /// Binds all items (inventory, equipment, quest inventory) to the game scene
    /// 
    /// Mirrors C# BindAllItems(), lines 696-717
    pub fn bind_all_items(&mut self) {
        // TODO: Implement item binding
        // In C#, this looks up items in GameScene.ItemInfoList
        // For now, we skip this as items already have their info embedded
        // This will be needed when we have a centralized ItemInfo registry
    }
    
    /// Override SetAction to handle QueuedAction
    /// 
    /// Mirrors C# UserObject.SetAction() override, lines 787-799
    /// 
    /// C# Logic:
    /// ```csharp
    /// public override void SetAction()
    /// {
    ///     if (QueuedAction != null && !GameScene.Observing)
    ///     {
    ///         if ((ActionFeed.Count == 0) || (ActionFeed.Count == 1 && NextAction.Action == MirAction.Stance))
    ///         {
    ///             ActionFeed.Clear();
    ///             ActionFeed.Add(QueuedAction);
    ///             QueuedAction = null;
    ///         }
    ///     }
    ///     base.SetAction();
    /// }
    /// ```
    pub fn set_action(&mut self) {
        // Handle QueuedAction (C# lines 789-797)
        if let Some(queued) = self.queued_action.take() {
            // TODO Phase 2: Add GameScene.Observing check
            // if !GameScene.Observing {
            
            // Check if action feed is empty or only has stance (C# line 791)
            let should_add = self.player.action_feed.is_empty() 
                || (self.player.action_feed.len() == 1 
                    && self.player.next_action().map(|a| a.action) == Some(MirAction::Stance));
            
            if should_add {
                // Clear and add queued action (C# lines 793-794)
                self.player.action_feed.clear();
                self.player.action_feed.push(queued);
            } else {
                // Put it back if we can't add it
                self.queued_action = Some(queued);
            }
            
            // }  // End GameScene.Observing check
        }
        
        // Call base implementation (C# line 798)
        self.player.set_action();
    }
    
    /// Override ProcessFrames to clear QueuedAction and trigger next action
    /// 
    /// Mirrors C# UserObject.ProcessFrames() override, lines 800-809
    /// 
    /// C# Logic:
    /// ```csharp
    /// public override void ProcessFrames()
    /// {
    ///     bool clear = CMain.Time >= NextMotion;
    ///     base.ProcessFrames();
    ///     if (clear) QueuedAction = null;
    ///     if ((CurrentAction == MirAction.Standing || CurrentAction == MirAction.MountStanding || 
    ///          CurrentAction == MirAction.Stance || CurrentAction == MirAction.Stance2 || 
    ///          CurrentAction == MirAction.DashFail) && (QueuedAction != null || NextAction != null))
    ///         SetAction();
    /// }
    /// ```
    /// 
    /// Phase 2 Update: Now accepts current_time parameter for proper motion timing
    pub fn process_frames(&mut self, current_time: u64) {
        // Check if motion time has passed (C# line 802)
        let clear = current_time >= self.player.next_motion;
        
        // Call base implementation (C# line 803)
        self.player.process_frames(current_time);
        
        // Clear queued action if motion completed (C# line 804)
        if clear {
            self.queued_action = None;
        }
        
        // Trigger next action if in idle state (C# lines 805-807)
        let is_idle = matches!(
            self.player.current_action,
            MirAction::Standing | MirAction::MountStanding | 
            MirAction::Stance | MirAction::Stance2 | MirAction::DashFail
        );
        
        let has_next = self.queued_action.is_some() || self.player.next_action().is_some();
        
        if is_idle && has_next {
            self.set_action();
        }
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
    
    /// Applies stat caps to prevent values from exceeding limits
    /// 
    /// Mirrors C# RefreshStatCaps(), lines 670-694
    pub fn refresh_stat_caps(&mut self) {
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
        self.player.weapon = -1;
        self.player.weapon_effect = 0;
        self.player.armour = 0;
        self.player.wing_effect = 0;
        self.player.mount_type = -1;
        self.player.fast_run = false;
        self.item_mode = SpecialItemMode::NONE;
        self.current_wear_weight = 0;
        self.current_hand_weight = 0;
        self.item_sets.clear();
        self.mir_set.clear();

        for (index, slot) in self.equipment.iter().enumerate() {
            let Some(item) = slot.as_ref() else { continue; };
            let Some(info) = item.info.as_ref() else { continue; };

            let weight = item.weight(Some(info));
            if matches!(info.item_type, ItemType::Weapon | ItemType::Torch) {
                self.current_hand_weight = self.current_hand_weight.saturating_add(weight);
            } else {
                self.current_wear_weight = self.current_wear_weight.saturating_add(weight);
            }

            if item.current_dura == 0 && info.durability > 0 {
                continue;
            }

            match info.item_type {
                ItemType::Armour => {
                    self.player.armour = info.shape as i32;
                    self.player.wing_effect = info.effect;
                }
                ItemType::Weapon => {
                    self.player.weapon = info.shape as i32;
                    self.player.weapon_effect = info.effect as i32;
                }
                ItemType::Mount => {
                    self.player.mount_type = info.shape;
                }
                _ => {}
            }

            if info.item_type == ItemType::Weapon && info.is_fishing_rod() {
                continue;
            }

            self.stats.add_assign(&info.stats);
            self.stats.add_assign(&item.added_stats);

            let awake_ac = item.awake.get_ac();
            self.stats.add_min_ac(awake_ac);
            self.stats.add_max_ac(awake_ac);

            let awake_mac = item.awake.get_mac();
            self.stats.add_min_mac(awake_mac);
            self.stats.add_max_mac(awake_mac);

            let awake_dc = item.awake.get_dc();
            self.stats.add_min_dc(awake_dc);
            self.stats.add_max_dc(awake_dc);

            let awake_mc = item.awake.get_mc();
            self.stats.add_min_mc(awake_mc);
            self.stats.add_max_mc(awake_mc);

            let awake_sc = item.awake.get_sc();
            self.stats.add_min_sc(awake_sc);
            self.stats.add_max_sc(awake_sc);

            let awake_hp_mp = item.awake.get_hp_mp();
            self.stats.add_max_hp(awake_hp_mp);
            self.stats.add_max_mp(awake_hp_mp);

            self.player.map_object.light =
                self.player.map_object.light.max(info.light as i32);
            self.item_mode |= info.unique;

            if info.can_fast_run {
                self.player.fast_run = true;
            }

            if !(info.item_type == ItemType::Mount && !self.player.riding_mount) {
                for socket in &item.slots {
                    let Some(socket_item) = socket.as_ref() else { continue; };
                    let Some(socket_info) = socket_item.info.as_ref() else { continue; };

                    let socket_weight = socket_item.weight(Some(socket_info));
                    if matches!(socket_info.item_type, ItemType::Weapon | ItemType::Torch) {
                        self.current_hand_weight =
                            self.current_hand_weight.saturating_add(socket_weight);
                    } else {
                        self.current_wear_weight =
                            self.current_wear_weight.saturating_add(socket_weight);
                    }

                    if socket_item.current_dura == 0 && socket_info.durability > 0 {
                        continue;
                    }

                    self.stats.add_assign(&socket_info.stats);
                    self.stats.add_assign(&socket_item.added_stats);

                    self.player.map_object.light =
                        self.player.map_object.light.max(socket_info.light as i32);
                    self.item_mode |= socket_info.unique;
                }
            }

            if info.set == ItemSet::None {
                continue;
            }

            let item_type = info.item_type;
            if let Some(existing) = self
                .item_sets
                .iter_mut()
                .find(|set| set.set == info.set && !set.is_complete() && !set.types.contains(&item_type))
            {
                existing.types.push(item_type);
                existing.count = existing.count.saturating_add(1);
            } else {
                self.item_sets.push(ItemSets {
                    set: info.set,
                    types: vec![item_type],
                    count: 1,
                });
            }

            if info.set == ItemSet::Mir {
                if let Ok(slot_enum) = EquipmentSlot::try_from(index as u8) {
                    if !self.mir_set.contains(&slot_enum) {
                        self.mir_set.push(slot_enum);
                    }
                }
            }
        }

        if self.item_mode.contains(SpecialItemMode::MUSCLE) {
            let bag = self.stats.get(Stat::BagWeight) * 2;
            let wear = self.stats.get(Stat::WearWeight) * 2;
            let hand = self.stats.get(Stat::HandWeight) * 2;

            self.stats.set(Stat::BagWeight, bag);
            self.stats.set(Stat::WearWeight, wear);
            self.stats.set(Stat::HandWeight, hand);
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
    
    /// Refreshes guild buffs and applies them to stats
    /// 
    /// Mirrors C# RefreshGuildBuffs(), lines 649-667
    pub fn refresh_guild_buffs(&mut self) {
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
        let attack_speed_stat = self.stats.get_attack_speed();
        let level_component = std::cmp::min(370, self.player.level as i32 * 14);
        let calculated = 1400 - (attack_speed_stat * 60 + level_component);
        self.attack_speed = std::cmp::max(550, calculated);
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

    /// Count free slots in an inventory array
    /// 
    /// Mirrors C# UserObject.FreeSpace(UserItem[] array), lines 772-785
    fn free_space(array: &[Option<UserItem>]) -> usize {
        array.iter().filter(|slot| slot.is_none()).count()
    }

    /// Calculate how many of an item can be gained based on free space and stack limits
    /// 
    /// Mirrors C# UserObject.GetMaxGain(UserItem item), lines 731-768
    /// 
    /// Modifies `item.count` to the maximum amount that can be gained.
    /// If nothing can be gained, sets count to 0.
    pub fn get_max_gain(&self, item: &mut UserItem) {
        let free_slots = Self::free_space(&self.inventory);
        
        // If there are free slots, can gain full amount
        if free_slots > 0 {
            return;
        }
        
        // No free slots - check for stackable items
        let mut can_gain: u16 = 0;
        let item_info = match &item.info {
            Some(info) => info,
            None => {
                item.count = 0;
                return;
            }
        };
        
        for slot in &self.inventory {
            let Some(inv_item) = slot.as_ref() else { continue; };
            let Some(inv_info) = inv_item.info.as_ref() else { continue; };
            
            // Check if same item type
            if inv_info.index != item_info.index {
                continue;
            }
            
            // Calculate available stack space
            let available_stack = inv_info.stack_size.saturating_sub(inv_item.count);
            
            if available_stack == 0 {
                continue;
            }
            
            can_gain = can_gain.saturating_add(available_stack);
            
            if can_gain >= item.count {
                return;
            }
        }
        
        // Can't gain anything or partial amount
        item.count = can_gain;
    }

    /// Clear next magic casting state
    /// 
    /// Mirrors C# UserObject.ClearMagic(), lines 812-817
    pub fn clear_magic(&mut self) {
        self.next_magic = None;
        self.next_magic_direction = MirDirection::Up;
        self.next_magic_location = Point::new(0, 0);
        self.next_magic_object = None;
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
    /// 
    /// Mirrors C# SetLibraries(), lines 128-131
    pub fn set_libraries(&mut self) {
        self.player.set_libraries();
    }
    
    /// Sets visual effects based on current state (buffs, equipment, level effects)
    /// 
    /// Mirrors C# SetEffects(), lines 133-136 (UserObject), 631-741 (PlayerObject base implementation)
    pub fn set_effects(&mut self) {
        // TODO: This requires Effect system integration
        // The C# implementation (in PlayerObject.SetEffects):
        // 1. Clears all existing SpecialEffects (line 633-636)
        // 2. Returns early if riding mount (line 638)
        // 3. Adds magic shield effect if CurrentEffect == MagicShieldUp (line 640-648)
        // 4. Adds wing effects based on WingEffect value (line 650-666)
        // 5. Adds level effects based on LevelEffects flags (line 670-741):
        //    - BlueDragon: Libraries.Effect, 1210, 20 frames, 3200ms
        //    - RedDragon: Libraries.Effect, 990, 20 frames, 3200ms + secondary effect
        //    - Mist: Libraries.Effect, 296, 32 frames, 3600ms
        //    - Rebirth1: Libraries.Magic3, 6800, 20 frames, 3600ms
        //    - Rebirth2: Libraries.Magic3, 6870, 19 frames, 3600ms + secondary effect
        //    - Rebirth3: Libraries.Magic3, 6906, 19 frames, 3600ms + secondary effect
        //    - NewBlue: Libraries.Magic3, 7040, 31 frames, 3600ms + secondary effect
        //    - YellowDragon: Libraries.Magic3, 7120, 31 frames, 3600ms + secondary effect
        //    - Phoenix: Libraries.Magic3, 6970, 26 frames, 3600ms + secondary effect
        // Each effect uses SpecialEffect with repeat, delay, and specific animation parameters
        
        // For now, this is a placeholder that would delegate to PlayerObject
        // Once the Effect system is implemented, this will:
        // - Call self.player.set_effects() for base implementation
        // - Add any UserObject-specific effects
    }
    
    // ==================== Additional Helper Methods ====================
    
    /// Find an item in inventory by unique ID
    pub fn find_inventory_item(&self, unique_id: u64) -> Option<(usize, &UserItem)> {
        self.inventory.iter()
            .enumerate()
            .find_map(|(index, slot)| {
                slot.as_ref().and_then(|item| {
                    if item.unique_id == unique_id {
                        Some((index, item))
                    } else {
                        None
                    }
                })
            })
    }
    
    /// Find an item in equipment by unique ID
    pub fn find_equipment_item(&self, unique_id: u64) -> Option<(usize, &UserItem)> {
        self.equipment.iter()
            .enumerate()
            .find_map(|(index, slot)| {
                slot.as_ref().and_then(|item| {
                    if item.unique_id == unique_id {
                        Some((index, item))
                    } else {
                        None
                    }
                })
            })
    }
    
    /// Find an item in quest inventory by unique ID
    pub fn find_quest_item(&self, unique_id: u64) -> Option<(usize, &UserItem)> {
        self.quest_inventory.iter()
            .enumerate()
            .find_map(|(index, slot)| {
                slot.as_ref().and_then(|item| {
                    if item.unique_id == unique_id {
                        Some((index, item))
                    } else {
                        None
                    }
                })
            })
    }
    
    /// Check if player has a specific spell/magic learned
    pub fn has_magic(&self, spell: Spell) -> bool {
        self.magics.iter().any(|m| m.spell == spell)
    }
    
    /// Get the count of a specific item type in inventory
    pub fn count_item(&self, item_index: i32) -> u16 {
        self.inventory.iter()
            .filter_map(|slot| slot.as_ref())
            .filter(|item| {
                item.info.as_ref().map_or(false, |info| info.index == item_index)
            })
            .map(|item| item.count)
            .sum()
    }
    
    /// Check if inventory has enough free space for n items
    pub fn has_space_for(&self, count: usize) -> bool {
        let free_slots = Self::free_space(&self.inventory);
        free_slots >= count
    }
    
    /// Get current HP percentage (0-100)
    pub fn hp_percent(&self) -> u8 {
        let max_hp = self.stats.get_max_hp();
        if max_hp <= 0 {
            return 0;
        }
        ((self.hp as f32 / max_hp as f32) * 100.0).min(100.0) as u8
    }
    
    /// Get current MP percentage (0-100)
    pub fn mp_percent(&self) -> u8 {
        let max_mp = self.stats.get_max_mp();
        if max_mp <= 0 {
            return 0;
        }
        ((self.mp as f32 / max_mp as f32) * 100.0).min(100.0) as u8
    }
    
    /// Check if player is overweight
    pub fn is_overweight(&self) -> bool {
        let max_bag_weight = self.stats.get(Stat::BagWeight);
        let max_wear_weight = self.stats.get(Stat::WearWeight);
        let max_hand_weight = self.stats.get(Stat::HandWeight);
        
        self.current_bag_weight > max_bag_weight ||
        self.current_wear_weight > max_wear_weight ||
        self.current_hand_weight > max_hand_weight
    }
    
    /// Get total gold amount
    pub fn gold(&self) -> u32 {
        self.gold
    }
    
    /// Get total credit amount
    pub fn credit(&self) -> u32 {
        self.credit
    }
    
    /// Check if player can trade
    pub fn can_trade(&self) -> bool {
        self.allow_trade && !self.trade_locked
    }
    
    /// Get a mutable reference to inventory item at slot
    pub fn get_inventory_item_mut(&mut self, slot: usize) -> Option<&mut UserItem> {
        self.inventory.get_mut(slot).and_then(|s| s.as_mut())
    }
    
    /// Get a mutable reference to equipment item at slot
    pub fn get_equipment_item_mut(&mut self, slot: EquipmentSlot) -> Option<&mut UserItem> {
        self.equipment.get_mut(slot as usize).and_then(|s| s.as_mut())
    }
    
    /// Get a mutable reference to quest inventory item at slot
    pub fn get_quest_item_mut(&mut self, slot: usize) -> Option<&mut UserItem> {
        self.quest_inventory.get_mut(slot).and_then(|s| s.as_mut())
    }
    
    /// Check if a quest is completed
    pub fn is_quest_completed(&self, quest_id: i32) -> bool {
        self.completed_quests.contains(&quest_id)
    }
    
    /// Check if a quest is active
    pub fn has_active_quest(&self, quest_id: i32) -> bool {
        self.current_quests.iter().any(|q| q.id == quest_id)
    }
    
    // ==================== Action & Animation System ====================
    
    /// Set the current action with queued action handling
    /// 
    /// Mirrors C# UserObject.SetAction(), lines 787-800
    /// This is a public wrapper around the private set_action that handles queued actions
    pub fn perform_action(&mut self) {
        // C# lines 789-797: Handle queued actions before setting action
        // if (QueuedAction != null && !GameScene.Observing)
        if let Some(queued) = self.queued_action.take() {
            // Check if action feed is empty or only contains a stance action
            let should_add_queued = self.player.map_object.action_feed.is_empty() || 
                (self.player.map_object.action_feed.len() == 1 && 
                 self.player.map_object.action_feed.first()
                    .map_or(false, |a| a.action == MirAction::Stance));
            
            if should_add_queued {
                self.player.map_object.action_feed.clear();
                self.player.map_object.action_feed.push(queued);
                // Queued action is consumed (already taken above)
            } else {
                // Put it back if we can't use it yet
                self.queued_action = Some(queued);
            }
        }
        
        // C# line 799: Call base.SetAction()
        // Call the private set_action which sets initial standing pose
        self.set_action();
    }
    
    /// Process animation frames with user-specific logic
    /// 
    /// Mirrors C# UserObject.ProcessFrames(), lines 801-810
    pub fn process_animation(&mut self, delta_time: f32) {
        // C# line 803: bool clear = CMain.Time >= NextMotion;
        // In Rust, we track motion completion through the animation system
        // For now, we'll update animation and check if frame completed
        let motion_complete = {
            // Update frame animation
            self.player.update_frame_animation(delta_time);
            // Check if animation cycle completed (simplified check)
            // TODO: Proper motion timing when animation system is complete
            false // Placeholder
        };
        
        // C# line 807: if (clear) QueuedAction = null;
        if motion_complete {
            self.queued_action = None;
        }
        
        // C# line 808: Check if should set action when in standing/idle states
        let current_action = self.player.map_object.current_action();
        let is_standing = matches!(
            current_action,
            MirAction::Standing | MirAction::MountStanding | 
            MirAction::Stance | MirAction::Stance2 | MirAction::DashFail
        );
        
        let has_pending_action = self.queued_action.is_some() || 
            !self.player.map_object.action_feed.is_empty();
        
        if is_standing && has_pending_action {
            self.perform_action();
        }
    }
    
    /// Queue an action to be performed next
    /// 
    /// Used for combo attacks and delayed actions
    pub fn queue_action(&mut self, action: QueuedAction) {
        self.queued_action = Some(action);
    }
    
    /// Check if there's a queued action
    pub fn has_queued_action(&self) -> bool {
        self.queued_action.is_some()
    }
    
    /// Take the queued action (consumes it)
    pub fn take_queued_action(&mut self) -> Option<QueuedAction> {
        self.queued_action.take()
    }
    
    /// Add an action to the action feed
    pub fn add_action(&mut self, action: QueuedAction) {
        self.player.map_object.action_feed.push(action);
    }
    
    /// Clear all queued actions
    pub fn clear_actions(&mut self) {
        self.player.map_object.action_feed.clear();
        self.queued_action = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::stats_ext::StatsExt;
    use mir2_shared::{
        data::item::ItemInfo,
        enums::{AwakeType, EquipmentSlot, ItemType, SpecialItemMode, Stat},
    };

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

    #[test]
    fn refresh_equipment_stats_applies_item_and_awake_bonuses() {
        let mut user = UserObject::new(1);
        user.stats = Stats::default();

        let mut armour_info = ItemInfo::default();
        armour_info.item_type = ItemType::Armour;
        armour_info.shape = 5;
        armour_info.effect = 3;
        armour_info.weight = 10;
        armour_info.durability = 10;
        armour_info.light = 5;
        armour_info.can_fast_run = true;
        armour_info.stats.set(Stat::MinAC, 2);
        armour_info.stats.set(Stat::MaxAC, 5);

        let mut armour_item = UserItem::with_info(armour_info);
        armour_item.current_dura = 10;
        armour_item.max_dura = 10;
        armour_item.added_stats.set(Stat::MinAC, 1);
        armour_item.added_stats.set(Stat::MaxAC, 2);
        armour_item.awake.awake_type = AwakeType::Ac;
        armour_item.awake.levels = vec![1, 1];

        user.equipment[EquipmentSlot::Armour as usize] = Some(armour_item);

        let mut ring_info = ItemInfo::default();
        ring_info.item_type = ItemType::Ring;
        ring_info.weight = 1;
        ring_info.durability = 1;
        ring_info.unique = SpecialItemMode::MUSCLE;
        ring_info.stats.set(Stat::BagWeight, 10);
        ring_info.stats.set(Stat::HandWeight, 5);
        ring_info.stats.set(Stat::WearWeight, 6);

        let mut ring_item = UserItem::with_info(ring_info);
        ring_item.current_dura = 1;
        ring_item.max_dura = 1;

        user.equipment[EquipmentSlot::RingL as usize] = Some(ring_item);

        user.refresh_equipment_stats();

        assert_eq!(user.current_wear_weight, 11);
        assert_eq!(user.current_hand_weight, 0);
        assert_eq!(user.player.armour, 5);
        assert_eq!(user.player.wing_effect, 3);
        assert!(user.player.fast_run);
        assert_eq!(user.player.map_object.light, 5);
        assert_eq!(user.stats.get_min_ac(), 5);
        assert_eq!(user.stats.get_max_ac(), 9);
        assert!(user.item_mode.contains(SpecialItemMode::MUSCLE));
        assert_eq!(user.stats.get_bag_weight(), 20);
        assert_eq!(user.stats.get_hand_weight(), 10);
        assert_eq!(user.stats.get_wear_weight(), 12);
    }

    #[test]
    fn calculate_attack_speed_matches_csharp_logic() {
        let mut user = UserObject::new(1);
        user.stats.set(Stat::AttackSpeed, 5);
        user.player.level = 10;

        user.calculate_attack_speed();
        assert_eq!(user.attack_speed, 960);

        user.stats.set(Stat::AttackSpeed, 20);
        user.player.level = 90;
        user.calculate_attack_speed();
        assert_eq!(user.attack_speed, 550);
    }

    #[test]
    fn test_free_space() {
        let mut user = UserObject::new(1);
        // Initially all slots are empty (46 slots)
        assert_eq!(UserObject::free_space(&user.inventory), 46);

        // Fill some slots
        let mut item_info = ItemInfo::default();
        item_info.stack_size = 10;
        let item = UserItem::with_info(item_info);
        
        user.inventory[0] = Some(item.clone());
        user.inventory[1] = Some(item.clone());
        user.inventory[2] = Some(item.clone());
        
        assert_eq!(UserObject::free_space(&user.inventory), 43);
    }

    #[test]
    fn test_get_max_gain() {
        let mut user = UserObject::new(1);
        
        // Stackable items with no existing stacks - returns early
        let mut stackable = ItemInfo::default();
        stackable.index = 100;
        stackable.stack_size = 10;
        stackable.item_type = ItemType::Potion;
        stackable.shape = 1;
        
        let mut test_item = UserItem::with_info(stackable.clone());
        test_item.count = 10;
        
        // With free space, count remains unchanged
        user.get_max_gain(&mut test_item);
        assert_eq!(test_item.count, 10);

        // Fill all inventory slots
        for i in 0..46 {
            let mut filler = ItemInfo::default();
            filler.index = 999; // Different item
            filler.stack_size = 1;
            user.inventory[i] = Some(UserItem::with_info(filler));
        }
        
        // Now with no free space and no matching items, count = 0
        test_item.count = 10;
        user.get_max_gain(&mut test_item);
        assert_eq!(test_item.count, 0);

        // Add some partial stacks of the same item
        let mut item = UserItem::with_info(stackable.clone());
        item.count = 5; // Partial stack (can add 5 more)
        user.inventory[0] = Some(item.clone());
        
        test_item.count = 10;
        user.get_max_gain(&mut test_item);
        assert_eq!(test_item.count, 5); // Can gain 5

        item.count = 3; // Another partial stack (can add 7 more)
        user.inventory[1] = Some(item.clone());
        
        test_item.count = 20;
        user.get_max_gain(&mut test_item);
        assert_eq!(test_item.count, 12); // Can gain 5 + 7 = 12

        // Add a full stack - no change
        item.count = 10;
        user.inventory[2] = Some(item.clone());
        
        test_item.count = 20;
        user.get_max_gain(&mut test_item);
        assert_eq!(test_item.count, 12); // Still 12, no change from full stack
    }

    #[test]
    fn test_clear_magic() {
        use mir2_shared::enums::MirDirection;
        let mut user = UserObject::new(1);
        
        // Set up magic state - note: would need to properly create ClientMagic
        // For now just test the fields we can set
        user.next_magic_location = Point::new(10, 20);
        user.next_magic_object = Some(123);
        user.next_magic_direction = MirDirection::Up;
        
        // Clear magic
        user.clear_magic();
        
        // Verify all fields are cleared
        assert!(user.next_magic.is_none());
        assert_eq!(user.next_magic_location.x, 0);
        assert_eq!(user.next_magic_location.y, 0);
        assert!(user.next_magic_object.is_none());
        assert_eq!(user.next_magic_direction, MirDirection::Up); // Direction unchanged
    }

    #[test]
    fn test_process_frames_with_time() {
        use mir2_shared::enums::MirAction;
        
        let mut user = UserObject::new(1);
        
        // Set up action with timing
        user.player.current_action = MirAction::Standing;
        user.player.next_motion = 1000; // Next motion at time 1000
        user.player.frame_interval = 100;
        
        // Create a simple frame
        let mut frame = crate::objects::Frame::default();
        frame.count = 4;
        frame.interval = 100;
        user.player.frame = Some(frame);
        
        // Time before next_motion - should not update
        user.process_frames(500);
        assert_eq!(user.player.frame_index, 0);
        
        // Time at next_motion - should update frame
        user.player.frame_index = 0;
        user.process_frames(1000);
        assert_eq!(user.player.frame_index, 1);
        
        // Continue updating
        user.process_frames(1100);
        assert_eq!(user.player.frame_index, 2);
    }

    #[test]
    fn test_action_queue_processing() {
        use mir2_shared::enums::{MirAction, MirDirection};
        
        let mut user = UserObject::new(1);
        
        // Add actions to queue
        user.player.action_feed.push(crate::objects::QueuedAction {
            action: MirAction::Walking,
            direction: MirDirection::Up,
            location: Point::new(100, 100),
        });
        
        user.player.action_feed.push(crate::objects::QueuedAction {
            action: MirAction::Standing,
            direction: MirDirection::Up,
            location: Point::new(100, 101),
        });
        
        // Set action should process first queued action
        assert_eq!(user.player.action_feed.len(), 2);
        user.set_action();
        assert_eq!(user.player.current_action, MirAction::Walking);
        assert_eq!(user.player.action_feed.len(), 1);
        
        // Set action again should process second
        user.set_action();
        assert_eq!(user.player.current_action, MirAction::Standing);
        assert_eq!(user.player.action_feed.len(), 0);
    }

}

// Implement DrawableMapObject trait for UserObject
impl DrawableMapObject for UserObject {
    fn draw(&self, ctx: &mut Context, canvas: &mut Canvas, draw_location: Point) -> GameResult {
        use ggez::graphics::{Color, DrawMode, Mesh, Text, PxScale, DrawParam};
        use ggez::mint::Point2;
        
        // 🎨 占位符: 绘制蓝色矩形代表玩家
        let x = draw_location.x as f32;
        let y = draw_location.y as f32;
        
        let draw_pos = Point2 { x, y };
        
        // 绘制身体 (蓝色矩形, 32x48像素)
        let body_color = if self.player.map_object.is_dead() {
            Color::from_rgb(50, 50, 100)  // 死亡: 暗蓝色
        } else {
            Color::from_rgb(0, 100, 255)  // 活着: 亮蓝色
        };
        
        let body = Mesh::new_rectangle(
            ctx,
            DrawMode::fill(),
            ggez::graphics::Rect::new(0.0, -48.0, 32.0, 48.0),
            body_color,
        )?;
        
        canvas.draw(&body, DrawParam::default().dest(draw_pos));
        
        // 绘制名字 (白色文字)
        let name_pos = Point2 { x: x, y: y - 60.0 };
        let mut text = Text::new(&self.player.map_object.name);
        text.set_scale(PxScale::from(12.0));
        canvas.draw(
            &text,
            DrawParam::default()
                .dest(name_pos)
                .color(Color::WHITE),
        );
        
        // 绘制等级标签
        let level_text = format!("Lv.{}", self.player.level);
        let level_pos = Point2 { x: x, y: y - 75.0 };
        let mut level_label = Text::new(&level_text);
        level_label.set_scale(PxScale::from(10.0));
        canvas.draw(
            &level_label,
            DrawParam::default()
                .dest(level_pos)
                .color(Color::from_rgb(255, 255, 0)), // 黄色
        );
        
        Ok(())
    }
    
    fn object_id(&self) -> u32 {
        self.player.map_object.object_id()
    }
    
    fn is_dead(&self) -> bool {
        self.player.map_object.is_dead()
    }
    
    fn is_hidden(&self) -> bool {
        self.player.map_object.is_hidden()
    }
    
    fn draw_priority(&self) -> i32 {
        2 // Users draw after items and spells
    }
}
