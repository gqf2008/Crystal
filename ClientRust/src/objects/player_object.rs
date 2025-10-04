// player_object.rs - Player character base class
// Mirrors C# Client/MirObjects/PlayerObject.cs
//
// This is the base class for UserObject and HeroObject.
// It contains all player-specific fields and methods.

use std::time::Instant;

use mir2_shared::{
    enums::{MirClass, MirGender, Spell, SpellEffect},
    Point,
};

use super::map_object::MapObject;
use super::monster_object::FrameSet;
use super::effect::Effect;

/// Player object - base class for UserObject and HeroObject
/// 
/// Mirrors C# Client/MirObjects/PlayerObject.cs
/// 
/// This is the base class containing all player-specific functionality.
/// In C#: UserObject : PlayerObject : MapObject
/// In Rust: We use composition - PlayerObject contains MapObject
#[derive(Debug, Clone)]
pub struct PlayerObject {
    // ==================== Inherited from MapObject ====================
    /// Base MapObject (contains ObjectID, Name, Location, etc.)
    pub map_object: MapObject,
    
    // ==================== Player Appearance ====================
    /// Player gender
    pub gender: MirGender,
    
    /// Player class
    pub class: MirClass,
    
    /// Hair style index
    pub hair: u8,
    
    /// Player level
    pub level: u16,
    
    // ==================== Visual Assets (Libraries) ====================
    // NOTE: These are texture library references in C#
    // In Rust, we'll handle these differently (likely through a resource manager)
    // For now, we keep the indices/IDs
    
    /// Armour appearance index
    pub armour: i32,
    
    /// Weapon appearance index
    pub weapon: i32,
    
    /// Weapon effect index
    pub weapon_effect: i32,
    
    /// Armour offset for library selection
    pub armour_offset: i32,
    
    /// Hair offset for library selection
    pub hair_offset: i32,
    
    /// Weapon offset for library selection
    pub weapon_offset: i32,
    
    /// Wing offset for library selection
    pub wing_offset: i32,
    
    /// Mount offset for library selection
    pub mount_offset: i32,
    
    // ==================== Sound Effects ====================
    /// Die sound index
    pub die_sound: i32,
    
    /// Flinch sound index
    pub flinch_sound: i32,
    
    /// Attack sound index
    pub attack_sound: i32,
    
    // ==================== Animation ====================
    /// Frame set for animations
    pub frames: FrameSet,
    
    /// Current animation frame (TODO: Phase 2 - Define Frame struct)
    /// C# has: public Frame Frame
    /// For now, we'll use the frame indices below until Frame struct is defined
    // pub frame: Option<Frame>,  // TODO: Phase 2
    
    /// Wing animation frame (TODO: Phase 2)
    /// C# has: public Frame WingFrame
    // pub wing_frame: Option<Frame>,  // TODO: Phase 2
    
    /// Current frame index
    pub frame_index: i32,
    
    /// Frame interval timer (milliseconds)
    pub frame_interval: i32,
    
    /// Effect frame index (for special effects)
    pub effect_frame_index: i32,
    
    /// Effect frame interval (milliseconds)
    pub effect_frame_interval: i32,
    
    /// Slow frame index (for slow motion effects)
    pub slow_frame_index: i32,
    
    /// Skip frame update counter
    pub skip_frame_update: u8,
    
    // ==================== Spell Casting ====================
    /// Current spell being cast
    pub spell: Option<Spell>,
    
    /// Spell level
    pub spell_level: u8,
    
    /// Is currently casting?
    pub cast: bool,
    
    /// Target object ID
    pub target_id: u32,
    
    /// Secondary target IDs (for multi-target spells)
    pub secondary_target_ids: Vec<u32>,
    
    /// Target point (for ground-targeted spells)
    pub target_point: Point,
    
    // ==================== Buffs & Effects ====================
    /// Magic shield active?
    pub magic_shield: bool,
    
    /// Shield effect instance
    pub shield_effect: Option<Effect>,
    
    /// Elemental barrier active?
    pub elemental_barrier: bool,
    
    /// Elemental barrier effect instance
    pub elemental_barrier_effect: Option<Effect>,
    
    /// Wing effect type
    pub wing_effect: u8,
    
    /// Current spell effect
    pub current_effect: SpellEffect,
    
    // ==================== Elemental System (Archer) ====================
    /// Has elemental buff active?
    pub elemental_buff: bool,
    
    /// Is concentrating?
    pub concentrating: bool,
    
    /// Concentrating effect (TODO: Phase 2 - Define InterruptionEffect type)
    /// C# has: public InterruptionEffect ConcentratingEffect
    // pub concentrating_effect: Option<Effect>,  // TODO: Phase 2
    
    /// Concentrate interrupted?
    pub concentrate_interrupted: bool,
    
    /// Has elements?
    pub has_elements: bool,
    
    /// Element casted?
    pub element_casted: bool,
    
    /// Element effect (orb count)
    pub element_effect: i32,
    
    /// Elements level
    pub elements_level: i32,
    
    /// Max element orbs
    pub element_orb_max: i32,
    
    // ==================== Mount & Transform ====================
    /// Is riding a mount?
    pub riding_mount: bool,
    
    /// Sprint mode active?
    pub sprint: bool,
    
    /// Fast run active?
    pub fast_run: bool,
    
    /// Mount type (-1 = no mount)
    pub mount_type: i16,
    
    /// Transform type (-1 = no transform)
    pub transform_type: i16,
    
    /// Stance delay in milliseconds (for Assassin stance)
    /// C# has: private short StanceDelay = 2500
    pub stance_delay: i16,
    
    /// Stance time (for animations)
    pub stance_time: Instant,
    
    /// Mount time (for animations)
    pub mount_time: Instant,
    
    // ==================== Fishing ====================
    /// Is fishing?
    pub fishing: bool,
    
    /// Found fish?
    pub found_fish: bool,
    
    /// Fishing point location
    pub fishing_point: Point,
    
    /// Fishing time
    pub fishing_time: Instant,
    
    // ==================== Special Timers ====================
    /// Blizzard stop time
    pub blizzard_stop_time: Instant,
    
    /// Reincarnation stop time
    pub reincarnation_stop_time: Instant,
    
    /// Slashing burst time
    pub slashing_burst_time: Instant,
    
    // ==================== Guild ====================
    /// Guild name
    pub guild_name: String,
    
    /// Guild rank name
    pub guild_rank_name: String,
    
    // ==================== Level Effects (TODO: Phase 3) ====================
    // Level effects flags (visual effects for high-level players)
    // C# has: public LevelEffects LevelEffects
    // TODO: Phase 3 - Define LevelEffects enum (BlueDragon, RedDragon, Mist, Rebirth, etc.)
    // pub level_effects: LevelEffects,
}

impl PlayerObject {
    /// Create a new PlayerObject with the given object ID, name, class, and gender
    /// 
    /// Mirrors C# PlayerObject(uint objectID) constructor
    pub fn new(object_id: u32, name: String, class: MirClass, gender: MirGender) -> Self {
        Self {
            map_object: MapObject::for_user(object_id, name),
            gender,
            class,
            hair: 0,
            level: 1,
            armour: 0,
            weapon: 0,
            weapon_effect: 0,
            armour_offset: 0,
            hair_offset: 0,
            weapon_offset: 0,
            wing_offset: 0,
            mount_offset: 0,
            die_sound: 0,
            flinch_sound: 0,
            attack_sound: 0,
            frames: FrameSet::default(),
            frame_index: 0,
            frame_interval: 0,
            effect_frame_index: 0,
            effect_frame_interval: 0,
            slow_frame_index: 0,
            skip_frame_update: 0,
            spell: None,
            spell_level: 0,
            cast: false,
            target_id: 0,
            secondary_target_ids: Vec::new(),
            target_point: Point::new(0, 0),
            magic_shield: false,
            shield_effect: None,
            elemental_barrier: false,
            elemental_barrier_effect: None,
            wing_effect: 0,
            current_effect: SpellEffect::None,
            elemental_buff: false,
            concentrating: false,
            concentrate_interrupted: false,
            has_elements: false,
            element_casted: false,
            element_effect: 0,
            elements_level: 0,
            element_orb_max: 0,
            riding_mount: false,
            sprint: false,
            fast_run: false,
            mount_type: -1,
            transform_type: -1,
            stance_delay: 2500,  // C# default value
            stance_time: Instant::now(),
            mount_time: Instant::now(),
            fishing: false,
            found_fish: false,
            fishing_point: Point::new(0, 0),
            fishing_time: Instant::now(),
            blizzard_stop_time: Instant::now(),
            reincarnation_stop_time: Instant::now(),
            slashing_burst_time: Instant::now(),
            guild_name: String::new(),
            guild_rank_name: String::new(),
        }
    }
    
    // ==================== Properties (C# property equivalents) ====================
    
    /// Check if player has a class-specific weapon
    /// 
    /// Mirrors C# PlayerObject.HasClassWeapon property
    pub fn has_class_weapon(&self) -> bool {
        const CLASS_WEAPON_COUNT: i32 = 50; // Globals.ClassWeaponCount
        
        match self.weapon / CLASS_WEAPON_COUNT {
            0 => {
                // Default weapon types
                self.class == MirClass::Wizard 
                    || self.class == MirClass::Warrior 
                    || self.class == MirClass::Taoist
            }
            1 => self.class == MirClass::Assassin,
            2 => self.class == MirClass::Archer,
            _ => false,
        }
    }
    
    /// Check if player has a fishing rod equipped
    /// 
    /// Mirrors C# PlayerObject.HasFishingRod property
    pub fn has_fishing_rod(&self) -> bool {
        // FishingRodShapes: 49, 50, 51, 52 (from Globals.cs)
        (49..=52).contains(&self.weapon)
    }
    
    // ==================== Methods ====================
    
    /// Set libraries based on class, gender, armour, weapon, mount, transform
    /// 
    /// Mirrors C# PlayerObject.SetLibraries()
    /// 
    /// **Phase 1 Implementation**: Simplified version supporting only basic cases:
    /// - Warrior/Wizard/Taoist classes
    /// - No mount/transform support yet
    /// - Basic weapon/armour offsets
    /// 
    /// **TODO Phase 2**: Add full implementation with:
    /// - Archer class (altAnim, bow animations)
    /// - Assassin class (dual weapon, stance animations)
    /// - Transform support (39 transform types)
    /// - Mount support
    /// - Fishing rod special handling
    /// - Wing effects (100+ types)
    /// 
    /// # Notes
    /// 
    /// This method sets the library offsets used for texture selection.
    /// In C#, it assigns MLibrary references. In Rust, we set indices/offsets
    /// that will be used by the graphics system to load textures.
    pub fn set_libraries(&mut self) {
        // TODO: Phase 1 - Basic implementation for Warrior/Wizard/Taoist
        // This is a placeholder. Full implementation requires:
        // 1. Graphics library system integration
        // 2. CurrentAction tracking (from MapObject)
        // 3. Transform/Mount state handling
        
        // For now, set basic offsets based on gender and class
        match self.class {
            MirClass::Warrior | MirClass::Wizard | MirClass::Taoist => {
                // C# code:
                // ArmourOffSet = Gender == MirGender.Male ? 0 : 808;
                // HairOffSet = Gender == MirGender.Male ? 0 : 808;
                // WeaponOffSet = Gender == MirGender.Male ? 0 : 416;
                // WingOffset = Gender == MirGender.Male ? 0 : 840;
                
                self.armour_offset = if self.gender == MirGender::Male { 0 } else { 808 };
                self.hair_offset = if self.gender == MirGender::Male { 0 } else { 808 };
                self.weapon_offset = if self.gender == MirGender::Male { 0 } else { 416 };
                self.wing_offset = if self.gender == MirGender::Male { 0 } else { 840 };
                self.mount_offset = 0;
            }
            MirClass::Archer => {
                // TODO: Phase 2 - Archer altAnim logic
                // Requires CurrentAction and HasClassWeapon checks
                self.armour_offset = 0;
                self.hair_offset = 0;
                self.weapon_offset = 0;
                self.wing_offset = 0;
                self.mount_offset = 0;
            }
            MirClass::Assassin => {
                // TODO: Phase 2 - Assassin altAnim logic
                // Requires CurrentAction and HasClassWeapon checks
                self.armour_offset = 0;
                self.hair_offset = 0;
                self.weapon_offset = 0;
                self.wing_offset = 0;
                self.mount_offset = 0;
            }
        }
        
        // Sound effects (simple implementation)
        self.die_sound = if self.gender == MirGender::Male { 20 } else { 21 }; // SoundList.MaleDie/FemaleDie
        self.flinch_sound = if self.gender == MirGender::Male { 22 } else { 23 }; // SoundList.MaleFlinch/FemaleFlinch
        self.attack_sound = 0; // TODO: Set based on weapon type
    }
    
    /// Clear current spell casting state
    /// 
    /// Mirrors spell clearing logic in C# PlayerObject
    pub fn clear_spell(&mut self) {
        self.spell = None;
        self.spell_level = 0;
        self.cast = false;
        self.target_id = 0;
        self.secondary_target_ids.clear();
        self.target_point = Point::new(0, 0);
    }
    
    /// Update frame index by delta
    /// 
    /// Helper method for animation updates
    pub fn update_frame_index(&mut self, delta: i32) {
        self.frame_index += delta;
        // TODO: Add frame wrapping logic based on current action
    }
}

// ==================== Unit Tests ====================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_player_object_creation() {
        let player = PlayerObject::new(123, "TestPlayer".to_string(), MirClass::Warrior, MirGender::Male);
        
        // Note: map_object fields are private, tested through accessors
        assert_eq!(player.class, MirClass::Warrior);
        assert_eq!(player.gender, MirGender::Male);
        assert_eq!(player.level, 1);
        assert_eq!(player.mount_type, -1);
        assert_eq!(player.transform_type, -1);
        assert!(!player.cast);
    }
    
    #[test]
    fn test_has_class_weapon_warrior() {
        let mut player = PlayerObject::new(1, "Warrior".to_string(), MirClass::Warrior, MirGender::Male);
        
        player.weapon = 10; // Weapon index < 50 (class 0)
        assert!(player.has_class_weapon());
        
        player.weapon = 60; // Weapon index 50-99 (class 1 - Assassin)
        assert!(!player.has_class_weapon());
    }
    
    #[test]
    fn test_has_class_weapon_assassin() {
        let mut player = PlayerObject::new(1, "Assassin".to_string(), MirClass::Assassin, MirGender::Female);
        
        player.weapon = 10; // Class 0 weapons
        assert!(!player.has_class_weapon());
        
        player.weapon = 60; // Class 1 weapons (50-99)
        assert!(player.has_class_weapon());
    }
    
    #[test]
    fn test_has_fishing_rod() {
        let mut player = PlayerObject::new(1, "Fisher".to_string(), MirClass::Warrior, MirGender::Male);
        
        player.weapon = 48;
        assert!(!player.has_fishing_rod());
        
        player.weapon = 49;
        assert!(player.has_fishing_rod());
        
        player.weapon = 52;
        assert!(player.has_fishing_rod());
        
        player.weapon = 53;
        assert!(!player.has_fishing_rod());
    }
    
    #[test]
    fn test_clear_spell() {
        let mut player = PlayerObject::new(1, "Wizard".to_string(), MirClass::Wizard, MirGender::Male);
        
        // Set spell casting state
        player.spell = Some(Spell::FireBall);
        player.spell_level = 3;
        player.cast = true;
        player.target_id = 456;
        player.secondary_target_ids = vec![1, 2, 3];
        player.target_point = Point::new(10, 20);
        
        // Clear spell
        player.clear_spell();
        
        assert!(player.spell.is_none());
        assert_eq!(player.spell_level, 0);
        assert!(!player.cast);
        assert_eq!(player.target_id, 0);
        assert!(player.secondary_target_ids.is_empty());
        assert_eq!(player.target_point, Point::new(0, 0));
    }
    
    #[test]
    fn test_set_libraries_male_warrior() {
        let mut player = PlayerObject::new(1, "Warrior".to_string(), MirClass::Warrior, MirGender::Male);
        
        player.set_libraries();
        
        // Male warrior should have 0 offsets
        assert_eq!(player.armour_offset, 0);
        assert_eq!(player.hair_offset, 0);
        assert_eq!(player.weapon_offset, 0);
        assert_eq!(player.wing_offset, 0);
        assert_eq!(player.mount_offset, 0);
        assert_eq!(player.die_sound, 20); // Male die sound
        assert_eq!(player.flinch_sound, 22); // Male flinch sound
    }
    
    #[test]
    fn test_set_libraries_female_wizard() {
        let mut player = PlayerObject::new(1, "Wizard".to_string(), MirClass::Wizard, MirGender::Female);
        
        player.set_libraries();
        
        // Female wizard should have gender-specific offsets
        assert_eq!(player.armour_offset, 808);
        assert_eq!(player.hair_offset, 808);
        assert_eq!(player.weapon_offset, 416);
        assert_eq!(player.wing_offset, 840);
        assert_eq!(player.mount_offset, 0);
        assert_eq!(player.die_sound, 21); // Female die sound
        assert_eq!(player.flinch_sound, 23); // Female flinch sound
    }
    
    #[test]
    fn test_set_libraries_male_taoist() {
        let mut player = PlayerObject::new(1, "Taoist".to_string(), MirClass::Taoist, MirGender::Male);
        
        player.set_libraries();
        
        // Male taoist should have 0 offsets (same as warrior)
        assert_eq!(player.armour_offset, 0);
        assert_eq!(player.hair_offset, 0);
        assert_eq!(player.weapon_offset, 0);
        assert_eq!(player.wing_offset, 0);
    }
}
