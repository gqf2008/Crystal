// player_object.rs - Player character base class
// Mirrors C# Client/MirObjects/PlayerObject.cs
//
// This is the base class for UserObject and HeroObject.
// It contains all player-specific fields and methods.

use std::time::Instant;
use std::collections::HashMap;

use mir2_shared::{
    enums::{MirAction, MirClass, MirDirection, MirGender, Spell, SpellEffect},
    Point,
};

use super::map_object::MapObject;
use super::effect::Effect;
use super::frames::Frame;

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
    /// C# equivalent: public FrameSet Frames (which is Dictionary<MirAction, Frame>)
    pub frames: HashMap<MirAction, Frame>,
    
    /// Current animation frame
    /// C# has: public Frame Frame
    pub frame: Option<Frame>,
    
    /// Wing animation frame
    /// C# has: public Frame WingFrame
    pub wing_frame: Option<Frame>,
    
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
    
    // ==================== Action System ====================
    /// Action queue for delayed/queued actions
    /// C# equivalent: public List<QueuedAction> ActionFeed
    pub action_feed: Vec<QueuedAction>,
    
    /// Current action being performed
    /// C# equivalent: public MirAction CurrentAction (in MapObject)
    pub current_action: MirAction,
    
    /// Time when next motion update should occur
    /// C# equivalent: public long NextMotion (in MapObject)
    pub next_motion: u64,
    
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
            frames: HashMap::new(),
            frame: None,  // Set by SetLibraries() or action changes
            wing_frame: None,
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
            action_feed: Vec::new(),
            current_action: MirAction::Standing,
            next_motion: 0,
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
    
    /// Load player data from network packet
    /// 
    /// Mirrors C# PlayerObject.Load(S.ObjectPlayer info), lines 113-168
    /// 
    /// This method syncs all player data from the server and initializes the player object.
    pub fn load(&mut self, packet: &mir2_shared::packets::server::ObjectPlayer) {
        // Sync base MapObject fields
        self.map_object.sync_from_player_packet(packet);
        
        // Guild information (C# lines 118-119)
        self.guild_name = packet.guild_name.clone();
        self.guild_rank_name = packet.guild_rank_name.clone();
        
        // Basic stats (C# lines 120-122)
        self.class = packet.class;
        self.gender = packet.gender;
        self.level = packet.level;
        
        // Hair (C# line 126)
        self.hair = packet.hair;
        
        // Equipment appearance (C# lines 128-130)
        self.weapon = packet.weapon as i32;
        self.weapon_effect = packet.weapon_effect as i32;
        self.armour = packet.armour as i32;
        
        // Wing effect (C# line 142)
        self.wing_effect = packet.wing_effect;
        
        // Current effect (C# line 143)
        self.current_effect = packet.effect;
        
        // Mount (C# lines 145-146)
        self.mount_type = packet.mount_type;
        self.riding_mount = packet.riding_mount;
        
        // Fishing (C# line 148)
        self.fishing = packet.fishing;
        
        // Transform (C# line 150)
        self.transform_type = packet.transform_type;
        
        // Set texture libraries (C# line 152)
        self.set_libraries();
        
        // If dead, add death action to queue (C# line 154)
        // TODO: Implement ActionFeed
        // if self.map_object.dead {
        //     self.map_object.action_feed.push(QueuedAction {
        //         action: MirAction::Dead,
        //         direction: self.map_object.direction,
        //         location: self.map_object.current_location,
        //     });
        // }
        
        // Extra effect (C# line 155)
        // TODO: Implement when Effect system is complete
        // if packet.extra {
        //     self.map_object.effects.push(Effect::new(...));
        // }
        
        // Elemental system (C# lines 157-159)
        self.element_effect = packet.element_orb_effect as i32;
        self.elements_level = packet.element_orb_lvl as i32;
        self.element_orb_max = packet.element_orb_max as i32;
        
        // Process buffs (C# lines 163-165)
        self.process_buffs();
        
        // Set action (C# line 167)
        // TODO: Implement SetAction()
        // self.set_action();
        
        // Set effects (C# line 169)
        // TODO: Implement SetEffects()
        // self.set_effects();
    }
    
    /// Update player appearance from network packet
    /// 
    /// Mirrors C# PlayerObject.Update(S.PlayerUpdate info), lines 170-180
    pub fn update(&mut self, packet: &mir2_shared::packets::server::PlayerUpdate) {
        // Update equipment (C# lines 172-174)
        self.weapon = packet.weapon as i32;
        self.weapon_effect = packet.weapon_effect as i32;
        self.armour = packet.armor as i32;
        
        // Update light (C# line 175)
        self.map_object.light = packet.light as i32;
        
        // Update wing effect (C# line 176)
        self.wing_effect = packet.wings_effect;
        
        // Reload libraries (C# line 178)
        self.set_libraries();
        
        // Reload effects (C# line 179)
        // TODO: Implement SetEffects()
        // self.set_effects();
    }
    
    /// Process all active buffs and add their visual effects
    /// 
    /// Mirrors C# PlayerObject.ProcessBuffs(), lines 182-186
    pub fn process_buffs(&mut self) {
        // Clone buffs to avoid borrow checker issues
        let buffs = self.map_object.buffs.clone();
        
        for buff_type in &buffs {
            self.map_object.add_buff_effect(*buff_type);
        }
    }
    
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
    
    /// Update frame animation
    /// 
    /// Mirrors C# PlayerObject.ProcessFrames() (simplified version)
    /// 
    /// This method advances the animation frame based on elapsed time.
    /// 
    /// # Arguments
    /// 
    /// * `delta_time` - Time elapsed since last update (in seconds)
    /// 
    /// # Phase 1 Implementation
    /// 
    /// This is a simplified version that:
    /// - Updates frame_index based on frame_interval
    /// - Wraps frame_index when reaching frame.count
    /// - Updates effect_frame_index similarly
    /// 
    /// # TODO Phase 2
    /// 
    /// - Integrate with CurrentAction (from MapObject)
    /// - Handle SkipFrameUpdate logic
    /// - Implement FastRun/Sprint speed modifiers
    /// - Handle Reverse animations
    /// - Integrate with movement offsets
    pub fn update_frame_animation(&mut self, delta_time: f32) {
        // Convert delta_time to milliseconds
        let delta_ms = (delta_time * 1000.0) as i32;
        
        // Update main frame
        if let Some(frame) = &self.frame {
            self.frame_interval += delta_ms;
            
            // Check if it's time to advance frame
            while self.frame_interval >= frame.interval && frame.interval > 0 {
                self.frame_interval -= frame.interval;
                
                // Advance frame index
                if !frame.reverse {
                    self.frame_index += 1;
                    if self.frame_index >= frame.count {
                        self.frame_index = 0; // Loop for now (TODO: check action repeat)
                    }
                } else {
                    self.frame_index -= 1;
                    if self.frame_index < 0 {
                        self.frame_index = frame.count - 1;
                    }
                }
            }
        }
        
        // Update effect frame (wings, weapon effects, etc.)
        if let Some(frame) = &self.wing_frame {
            self.effect_frame_interval += delta_ms;
            
            while self.effect_frame_interval >= frame.effect_interval && frame.effect_interval > 0 {
                self.effect_frame_interval -= frame.effect_interval;
                
                if !frame.reverse {
                    self.effect_frame_index += 1;
                    if self.effect_frame_index >= frame.effect_count {
                        self.effect_frame_index = 0;
                    }
                } else {
                    self.effect_frame_index -= 1;
                    if self.effect_frame_index < 0 {
                        self.effect_frame_index = frame.effect_count - 1;
                    }
                }
            }
        }
    }
    
    /// Calculate draw frame index
    /// 
    /// Mirrors C# PlayerObject.Process() DrawFrame calculation:
    /// DrawFrame = Frame.Start + (Frame.OffSet * Direction) + FrameIndex
    /// 
    /// # Arguments
    /// 
    /// * `direction` - Current facing direction (0-7)
    /// 
    /// # Returns
    /// 
    /// The absolute frame index to draw from the sprite sheet
    pub fn calc_draw_frame(&self, direction: u8) -> i32 {
        if let Some(frame) = &self.frame {
            frame.start + (frame.offset() * direction as i32) + self.frame_index
        } else {
            0
        }
    }
    
    /// Calculate wing/effect draw frame index
    /// 
    /// Mirrors C# PlayerObject.Process() DrawWingFrame calculation
    pub fn calc_wing_frame(&self, direction: u8) -> i32 {
        if let Some(frame) = &self.wing_frame {
            frame.effect_start + (frame.effect_offset() * direction as i32) + self.effect_frame_index
        } else {
            0
        }
    }

    // ==================== Spell Casting Methods ====================

    /// Cast a spell
    /// 
    /// C# equivalent: PlayerObject.Process() - MirAction.Spell case
    /// 
    /// Phase 1 simplified: Basic spell setup without complex effects
    /// TODO Phase 2: Add full spell effects, sound, missiles, etc.
    pub fn cast_spell(
        &mut self,
        spell: Spell,
        target_id: u32,
        target_point: Point,
        spell_level: u8,
        secondary_targets: Vec<u32>,
    ) {
        self.spell = Some(spell);
        self.target_id = target_id;
        self.target_point = target_point;
        self.spell_level = spell_level;
        self.secondary_target_ids = secondary_targets;
        self.cast = true;

        // TODO Phase 2: Add spell-specific logic
        // - Sound effects (SoundManager.PlaySound)
        // - Visual effects (Effects.Add)
        // - Missile creation (CreateProjectile)
        // - Special spell mechanics (Blizzard, Reincarnation, etc.)
        
        // For now, just mark as casting
        // The animation system will handle frame updates
    }

    /// Process next action after spell cast completes
    /// 
    /// C# equivalent: PlayerObject.NextSpellAction() (implied in C# Process)
    /// 
    /// This is called when spell animation completes
    pub fn next_spell_action(&mut self) {
        // Reset casting state
        self.cast = false;
        
        // TODO Phase 2: Handle spell completion
        // - Check if more actions in queue
        // - Return to standing/stance
        // - Process spell effects
        
        // For Phase 1, just clear the cast flag
    }

    /// Create spell effect (placeholder)
    /// 
    /// C# equivalent: Effects.Add(new Effect(...))
    /// 
    /// TODO Phase 2: Integrate with effect system
    pub fn create_spell_effect(&mut self, _spell: Spell) -> Option<Effect> {
        // TODO Phase 2: Create actual spell effects
        // This requires integration with:
        // - Effect system (Effect struct)
        // - Library system (texture libraries)
        // - Animation timing
        
        None
    }

    /// Check if player can cast spell
    /// 
    /// Helper method for validation
    pub fn can_cast_spell(&self) -> bool {
        // Basic checks
        if self.cast {
            return false; // Already casting
        }
        
        // TODO Phase 2: Add more checks
        // - Cooldown
        // - Mana cost
        // - Skill requirements
        // - Stun/frozen status
        
        true
    }

    /// Clear spell state
    /// 
    /// Helper method to reset spell-related fields
    pub fn clear_spell_state(&mut self) {
        self.spell = None;
        self.spell_level = 0;
        self.cast = false;
        self.target_id = 0;
        self.target_point = Point { x: 0, y: 0 };
        self.secondary_target_ids.clear();
    }

    // ==================== Drawing Methods ====================

    /// Main draw method - draws the player character
    /// 
    /// C# equivalent: PlayerObject.Draw()
    /// 
    /// Drawing order:
    /// 1. Behind effects
    /// 2. Mount (if riding)
    /// 3. Weapon (left side - drawn before body)
    /// 4. Body
    /// 5. Head/Hair
    /// 6. Wings
    /// 7. Weapon (right side - drawn after body)
    /// 
    /// Phase 1 simplified: Method framework without actual rendering
    /// TODO Phase 2: Integrate with graphics/rendering system
    pub fn draw(&self, _draw_location: Point) {
        // TODO Phase 2: Implement actual drawing
        // This requires:
        // - Graphics system integration
        // - Library/texture loading
        // - Sprite rendering
        // - Layer ordering
        
        // C# logic:
        // DrawBehindEffects(Settings.Effect);
        // DrawMount();
        // if (!RidingMount) {
        //     if (Direction == Left/Up/UpLeft/DownLeft) DrawWeapon();
        //     else DrawWeapon2();
        // }
        // DrawBody();
        // if (Direction == Up/UpLeft/UpRight/Right/Left) {
        //     DrawHead();
        //     DrawWings();
        // } else {
        //     DrawWings();
        //     DrawHead();
        // }
        // if (!RidingMount) {
        //     if (Direction == UpRight/Right/DownRight/Down) DrawWeapon();
        //     else DrawWeapon2();
        //     if (Class == Archer && HasClassWeapon) DrawWeapon2();
        // }
        
        // For Phase 1, this is a placeholder
    }

    /// Draw player body
    /// 
    /// C# equivalent: PlayerObject.DrawBody()
    /// 
    /// Phase 1 simplified: Returns the draw parameters
    /// TODO Phase 2: Actual rendering
    pub fn draw_body(&self, _draw_location: Point) -> DrawParams {
        // Calculate frame index
        let direction = self.map_object.direction() as u8;
        let frame_index = self.calc_draw_frame(direction);
        
        // C# logic:
        // BodyLibrary.Draw(DrawFrame + ArmourOffSet, DrawLocation, drawColour, true);
        
        DrawParams {
            library_type: LibraryType::Body,
            frame_index: frame_index + self.armour_offset,
            location: _draw_location,
            color: 0xFFFFFF, // White (TODO: apply effects)
            blend: false,
        }
    }

    /// Draw player head/hair
    /// 
    /// C# equivalent: PlayerObject.DrawHead()
    /// 
    /// Phase 1 simplified: Returns the draw parameters
    pub fn draw_head(&self, _draw_location: Point) -> DrawParams {
        let direction = self.map_object.direction() as u8;
        let frame_index = self.calc_draw_frame(direction);
        
        // C# logic:
        // HairLibrary.Draw(DrawFrame + HairOffSet, DrawLocation, DrawColour, true);
        
        DrawParams {
            library_type: LibraryType::Hair,
            frame_index: frame_index + self.hair_offset,
            location: _draw_location,
            color: 0xFFFFFF,
            blend: false,
        }
    }

    /// Draw primary weapon
    /// 
    /// C# equivalent: PlayerObject.DrawWeapon()
    /// 
    /// Phase 1 simplified: Returns the draw parameters
    pub fn draw_weapon(&self, _draw_location: Point) -> Option<DrawParams> {
        if self.weapon < 0 {
            return None;
        }
        
        let direction = self.map_object.direction() as u8;
        let frame_index = self.calc_draw_frame(direction);
        
        // C# logic:
        // WeaponLibrary1.Draw(DrawFrame + WeaponOffSet, DrawLocation, DrawColour, true);
        // if (WeaponEffectLibrary1 != null)
        //     WeaponEffectLibrary1.DrawBlend(DrawFrame + WeaponOffSet, DrawLocation, DrawColour, true, 0.4F);
        
        Some(DrawParams {
            library_type: LibraryType::Weapon,
            frame_index: frame_index + self.weapon_offset,
            location: _draw_location,
            color: 0xFFFFFF,
            blend: false,
        })
    }

    /// Draw secondary weapon (off-hand or two-handed)
    /// 
    /// C# equivalent: PlayerObject.DrawWeapon2()
    pub fn draw_weapon2(&self, _draw_location: Point) -> Option<DrawParams> {
        if self.weapon == -1 {
            return None;
        }
        
        let direction = self.map_object.direction() as u8;
        let frame_index = self.calc_draw_frame(direction);
        
        // C# logic:
        // WeaponLibrary2.Draw(DrawFrame + WeaponOffSet, DrawLocation, DrawColour, true);
        
        Some(DrawParams {
            library_type: LibraryType::Weapon,
            frame_index: frame_index + self.weapon_offset,
            location: _draw_location,
            color: 0xFFFFFF,
            blend: false,
        })
    }

    /// Draw wings
    /// 
    /// C# equivalent: PlayerObject.DrawWings()
    pub fn draw_wings(&self, _draw_location: Point) -> Option<DrawParams> {
        if self.wing_effect == 0 || self.wing_effect >= 100 {
            return None;
        }
        
        let direction = self.map_object.direction() as u8;
        let frame_index = self.calc_wing_frame(direction);
        
        // C# logic:
        // WingLibrary.DrawBlend(DrawWingFrame + WingOffset, DrawLocation, DrawColour, true);
        
        Some(DrawParams {
            library_type: LibraryType::Wing,
            frame_index: frame_index + self.wing_offset,
            location: _draw_location,
            color: 0xFFFFFF,
            blend: true, // Wings always use blend
        })
    }

    /// Draw mount
    /// 
    /// C# equivalent: PlayerObject.DrawMount()
    pub fn draw_mount(&self, _draw_location: Point) -> Option<DrawParams> {
        if self.mount_type < 0 || !self.riding_mount {
            return None;
        }
        
        let direction = self.map_object.direction() as u8;
        let frame_index = self.calc_draw_frame(direction);
        
        // C# logic:
        // MountLibrary.Draw(DrawFrame - 416 + MountOffset, DrawLocation, DrawColour, true);
        
        Some(DrawParams {
            library_type: LibraryType::Mount,
            frame_index: frame_index - 416 + self.mount_offset,
            location: _draw_location,
            color: 0xFFFFFF,
            blend: false,
        })
    }

    /// Check if weapon should be drawn before body (left side)
    /// 
    /// Helper for draw order logic
    pub fn weapon_drawn_before_body(&self) -> bool {
        let dir = self.map_object.direction();
        matches!(
            dir,
            MirDirection::Left | MirDirection::Up | MirDirection::UpLeft | MirDirection::DownLeft
        )
    }

    /// Check if head should be drawn before wings
    /// 
    /// Helper for draw order logic
    pub fn head_drawn_before_wings(&self) -> bool {
        let dir = self.map_object.direction();
        matches!(
            dir,
            MirDirection::Up
                | MirDirection::UpLeft
                | MirDirection::UpRight
                | MirDirection::Right
                | MirDirection::Left
        )
    }
    
    // ==================== Action System Methods ====================
    
    /// Get next action from action feed
    /// 
    /// C# equivalent: public QueuedAction NextAction { get { return ActionFeed.Count > 0 ? ActionFeed[0] : null; } }
    pub fn next_action(&self) -> Option<&QueuedAction> {
        self.action_feed.first()
    }
    
    /// Set current action from action feed
    /// 
    /// Mirrors C# PlayerObject.SetAction(), lines 885-975
    /// 
    /// This is the base implementation that processes the action queue and sets up
    /// the current animation frame. UserObject overrides this to handle QueuedAction.
    /// 
    /// # Phase 1 Implementation
    /// 
    /// This is a simplified version that:
    /// - Processes action_feed queue
    /// - Sets current_action and frame_index
    /// - Handles mount transformations
    /// - Sets up frame intervals
    /// 
    /// # TODO Phase 2
    /// 
    /// - Integrate with GameScene.CanMove
    /// - Handle MapControl.NextAction timing
    /// - Integrate with actual frame system
    /// - Handle stance/fishing animations
    /// - MapObject location updates
    pub fn set_action(&mut self) {
        // Check if we should skip due to movement restrictions
        // C# lines 887-903: if (NextAction != null && !GameScene.CanMove) { ... }\n        // TODO Phase 2: Add GameScene.CanMove check
        
        if self.action_feed.is_empty() {
            // No queued actions - set to standing/idle (C# lines 909-913)
            self.current_action = MirAction::Standing;
            
            // Check for blizzard effect (C# line 913)
            // TODO Phase 2: Implement blizzard stop time check
            // if current_time <= self.blizzard_stop_time { self.current_action = MirAction::Stance2; }
            
            // Transform standing action if riding mount (C# lines 915-934)
            if self.riding_mount {
                self.current_action = match self.current_action {
                    MirAction::Standing => MirAction::MountStanding,
                    MirAction::Walking => MirAction::MountWalking,
                    MirAction::Running => MirAction::MountRunning,
                    MirAction::Struck => MirAction::MountStruck,
                    MirAction::Attack1 => MirAction::MountAttack,
                    _ => self.current_action,
                };
            }
            
            // Handle stance timing (C# lines 936-942)
            // TODO Phase 2: Implement stance time check
            // if self.current_action == MirAction::Standing {
            //     if self.class == MirClass::Archer && self.has_class_weapon() {
            //         self.current_action = MirAction::Standing;
            //     } else if current_time <= self.stance_time {
            //         self.current_action = MirAction::Stance;
            //     }
            // }
            
            // Handle fishing (C# line 944)
            if self.fishing {
                self.current_action = MirAction::FishingWait;
            }
            
            // Get frame for current action (C# lines 946-948)
            self.frame = self.frames.get(&self.current_action).cloned();
            self.frame_index = 0;
            self.effect_frame_index = 0;
            
            // Set frame intervals (C# lines 957-958)
            if let Some(ref frame) = self.frame {
                self.frame_interval = frame.interval;
                self.effect_frame_interval = frame.effect_interval;
            }
            
            // Update libraries (C# line 960)
            self.set_libraries();
        } else {
            // Process next action from queue (C# lines 963-975)
            let action = self.action_feed.remove(0);
            
            self.current_action = action.action;
            
            // Transform action if riding mount (C# lines 967-985)
            if self.riding_mount {
                self.current_action = match self.current_action {
                    MirAction::Standing => MirAction::MountStanding,
                    MirAction::Walking => MirAction::MountWalking,
                    MirAction::Running => MirAction::MountRunning,
                    MirAction::Struck => MirAction::MountStruck,
                    MirAction::Attack1 => MirAction::MountAttack,
                    _ => self.current_action,
                };
            }
            
            // TODO Phase 2: Handle direction, location updates (C# lines 988-997)
            // self.map_object.set_direction(action.direction);
            // self.map_object.set_current_location(action.location);
            
            // Get frame for action (C# line 999)
            self.frame = self.frames.get(&self.current_action).cloned();
            self.frame_index = 0;
            self.effect_frame_index = 0;
            
            // Set frame intervals
            if let Some(ref frame) = self.frame {
                self.frame_interval = frame.interval;
                self.effect_frame_interval = frame.effect_interval;
            }
            
            // Update libraries
            self.set_libraries();
        }
    }
    
    /// Process animation frames
    /// 
    /// Mirrors C# PlayerObject.ProcessFrames(), lines 2300-2724
    /// 
    /// This is the base implementation that advances animation frames based on time.
    /// UserObject overrides this to handle QueuedAction clearing.
    /// 
    /// # Phase 1 Implementation
    /// 
    /// This is a highly simplified placeholder that:\n    /// - Updates frame_index when time passes
    /// - Calls SetAction() when animation completes
    /// 
    /// # TODO Phase 2
    /// 
    /// The full C# implementation is 400+ lines and handles:
    /// - Walking/Running with movement offsets (lines 2303-2337)
    /// - Jump animations (lines 2339-2359)
    /// - Dash animations (lines 2360-2397)
    /// - Standing/Idle animations (lines 2407-2428)
    /// - Fishing animations (lines 2430-2500)
    /// - Attack animations (lines 2502-2545)
    /// - Range attack animations (lines 2547-2724)
    /// - Spell casting animations
    /// - Sound effects (footsteps, attacks)
    /// - Movement validation (GameScene.CanMove)
    /// - Map texture invalidation
    /// 
    /// Each action type has specific frame handling logic:
    /// - Frame count checks
    /// - SkipFrames logic
    /// - Sound triggers at specific frames
    /// - Effect spawning
    /// - Movement offset updates
    /// 
    /// Phase 2 Implementation: Basic frame processing with time system integration
    /// Full implementation requires:
    /// - GameScene (CanMove, MapControl) - TODO Phase 3
    /// - Sound system (SoundManager) - TODO Phase 3
    /// - Effect system (MapControl.Effects) - TODO Phase 3
    pub fn process_frames(&mut self, current_time: u64) {
        // C#: if (Frame == null) return; (line 2302)
        if self.frame.is_none() {
            return;
        }
        
        let frame_count = self.frame.as_ref().map(|f| f.count).unwrap_or(0);
        
        match self.current_action {
            // Movement actions (C# lines 2305-2333)
            MirAction::Walking
            | MirAction::Running
            | MirAction::MountWalking
            | MirAction::MountRunning
            | MirAction::Sneek
            | MirAction::DashAttack => {
                // TODO Phase 3: Check GameScene.CanMove
                // TODO Phase 3: Invalidate texture/floor
                
                // Handle skip frames (C# line 2312)
                if self.map_object.skip_frames {
                    self.frame_index = frame_count;
                }
                
                // Update frame (C# line 2314)
                self.update_frame();
                
                if self.frame_index >= frame_count {
                    self.frame_index = frame_count - 1;
                    // Will call set_action in parent (UserObject)
                } else {
                    // TODO Phase 3: Play step sound at frames 1 and 4 for user
                }
                
                // TODO Phase 3: UpdateWingEffect()
            }
            
            // Jump action (C# lines 2335-2356)
            MirAction::Jump => {
                // TODO Phase 3: Check GameScene.CanMove
                // TODO Phase 3: Invalidate texture/floor
                
                if self.map_object.skip_frames {
                    self.update_frame();
                }
                
                self.update_frame();
                
                if self.frame_index >= frame_count {
                    self.frame_index = frame_count - 1;
                    // Will call set_action in parent
                } else {
                    // TODO Phase 3: Play jump sounds at frames 1 and 7
                }
                
                // TODO Phase 3: UpdateWingEffect()
            }
            
            // Dash left (C# lines 2357-2368)
            MirAction::DashL => {
                // TODO Phase 3: Check GameScene.CanMove
                // TODO Phase 3: Invalidate texture/floor
                
                self.update_frame();
                if self.frame_index >= 3 {
                    self.frame_index = 2;
                    // Will call set_action in parent
                }
                
                self.update_frame_2();
                if self.effect_frame_index >= 3 {
                    self.effect_frame_index = 2;
                }
            }
            
            // Dash right (C# lines 2369-2381)
            MirAction::DashR => {
                // TODO Phase 3: Check GameScene.CanMove
                // TODO Phase 3: Invalidate texture/floor
                
                self.update_frame();
                if self.frame_index >= 6 {
                    self.frame_index = 5;
                    // Will call set_action in parent
                }
                
                self.update_frame_2();
                if self.effect_frame_index >= 6 {
                    self.effect_frame_index = 5;
                }
            }
            
            // Pushed back (C# lines 2382-2398)
            MirAction::Pushed => {
                // TODO Phase 3: Check GameScene.CanMove
                // TODO Phase 3: Invalidate texture/floor
                
                self.frame_index -= 2;
                self.effect_frame_index -= 2;
                
                if self.frame_index < 0 {
                    self.frame_index = 0;
                    // Will call set_action in parent
                }
                
                if self.effect_frame_index < 0 {
                    self.effect_frame_index = 0;
                }
            }
            
            // Standing and idle actions (C# lines 2400-2420)
            MirAction::Standing
            | MirAction::MountStanding
            | MirAction::DashFail
            | MirAction::Harvest
            | MirAction::Stance
            | MirAction::Stance2 => {
                if current_time >= self.next_motion {
                    // TODO Phase 3: Invalidate texture
                    
                    if self.map_object.skip_frames {
                        self.update_frame();
                    }
                    
                    self.update_frame();
                    
                    if self.frame_index >= frame_count {
                        self.frame_index = frame_count - 1;
                        // Will call set_action in parent
                    } else {
                        self.next_motion += self.frame_interval as u64;
                    }
                }
                
                // TODO Phase 3: UpdateWingEffect()
            }
            
            // Fishing actions (C# lines 2423-2485)
            MirAction::FishingCast
            | MirAction::FishingReel
            | MirAction::FishingWait => {
                if current_time >= self.next_motion {
                    // TODO Phase 3: Invalidate texture
                    
                    if self.map_object.skip_frames {
                        self.update_frame();
                    }
                    
                    self.update_frame();
                    
                    if self.frame_index >= frame_count {
                        self.frame_index = frame_count - 1;
                        // Will call set_action in parent
                    } else {
                        // TODO Phase 3: Play fishing sounds and effects at frame 1
                        self.next_motion += self.frame_interval as u64;
                    }
                }
                
                // TODO Phase 3: UpdateWingEffect()
            }
            
            // Attack actions (C# lines 2487-2508)
            MirAction::Attack1
            | MirAction::Attack2
            | MirAction::Attack3
            | MirAction::Attack4
            | MirAction::MountAttack
            | MirAction::Mine => {
                if current_time >= self.next_motion {
                    // TODO Phase 3: Invalidate texture
                    
                    if self.map_object.skip_frames {
                        self.update_frame();
                    }
                    
                    self.update_frame();
                    
                    if self.frame_index >= frame_count {
                        // Set stance time (C# line 2498)
                        // TODO Phase 3: stance_time uses Instant type, needs conversion
                        // self.stance_time = current_time + self.stance_delay;
                        self.frame_index = frame_count - 1;
                        // Will call set_action in parent
                    } else {
                        // TODO Phase 3: Play attack sound at frame 1
                        self.next_motion += self.frame_interval as u64;
                    }
                }
                
                // TODO Phase 3: UpdateWingEffect()
            }
            
            // Ranged attack actions (C# lines 2510-2900+)
            MirAction::AttackRange1
            | MirAction::AttackRange2
            | MirAction::AttackRange3 => {
                if current_time >= self.next_motion {
                    // TODO Phase 3: Invalidate texture
                    
                    if self.map_object.skip_frames {
                        self.update_frame();
                    }
                    
                    self.update_frame();
                    
                    if self.frame_index >= frame_count {
                        self.frame_index = frame_count - 1;
                        // Will call set_action in parent
                    } else {
                        // TODO Phase 3: Spell-specific logic and projectile creation
                        self.next_motion += self.frame_interval as u64;
                    }
                }
                
                // TODO Phase 3: UpdateWingEffect()
            }
            
            // Spell casting (C# lines 2900+)
            MirAction::Spell => {
                if current_time >= self.next_motion {
                    // TODO Phase 3: Invalidate texture
                    
                    if self.map_object.skip_frames {
                        self.update_frame();
                    }
                    
                    self.update_frame();
                    
                    if self.frame_index >= frame_count {
                        self.frame_index = frame_count - 1;
                        // Will call set_action in parent
                    } else {
                        // TODO Phase 3: Spell-specific effects
                        self.next_motion += self.frame_interval as u64;
                    }
                }
                
                // TODO Phase 3: UpdateWingEffect()
            }
            
            // Other actions - no frame updates needed
            _ => {
                // Actions like Dead, Struck, etc. handled elsewhere
            }
        }
    }
    
    /// Update main frame index
    /// C#: UpdateFrame(), line ~3400
    fn update_frame(&mut self) -> i32 {
        self.frame_index += 1;
        self.frame_index
    }
    
    /// Update effect frame index (for dual-frame animations)
    /// C#: UpdateFrame2(), line ~3410
    fn update_frame_2(&mut self) -> i32 {
        self.effect_frame_index += 1;
        self.effect_frame_index
    }
}

// ==================== Drawing Support Types ====================

/// Parameters for drawing a sprite
/// 
/// This is returned by draw methods in Phase 1
/// TODO Phase 2: Replace with actual rendering calls
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawParams {
    pub library_type: LibraryType,
    pub frame_index: i32,
    pub location: Point,
    pub color: u32, // ARGB
    pub blend: bool,
}

/// Library type for texture resources
/// 
/// C# has separate Library objects (BodyLibrary, HairLibrary, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryType {
    Body,
    Hair,
    Weapon,
    Wing,
    Mount,
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
    
    #[test]
    fn test_frame_animation_basic() {
        let mut player = PlayerObject::new(1, "Warrior".to_string(), MirClass::Warrior, MirGender::Male);
        
        // Set up a basic animation frame (4 frames, 100ms per frame)
        player.frame = Some(Frame::basic(0, 4, 0, 100));
        player.frame_index = 0;
        player.frame_interval = 0;
        
        // Advance 50ms - should not change frame yet
        player.update_frame_animation(0.05);
        assert_eq!(player.frame_index, 0);
        assert_eq!(player.frame_interval, 50);
        
        // Advance another 60ms - should advance to frame 1
        player.update_frame_animation(0.06);
        assert_eq!(player.frame_index, 1);
        assert_eq!(player.frame_interval, 10); // 50 + 60 - 100 = 10
        
        // Advance 200ms - should advance 2 more frames
        player.update_frame_animation(0.2);
        assert_eq!(player.frame_index, 3);
    }
    
    #[test]
    fn test_frame_animation_loop() {
        let mut player = PlayerObject::new(1, "Wizard".to_string(), MirClass::Wizard, MirGender::Female);
        
        // Set up animation at last frame
        player.frame = Some(Frame::basic(0, 4, 0, 100));
        player.frame_index = 3; // Last frame
        player.frame_interval = 0;
        
        // Advance past interval - should loop back to 0
        player.update_frame_animation(0.15);
        assert_eq!(player.frame_index, 0);
    }
    
    #[test]
    fn test_calc_draw_frame() {
        let mut player = PlayerObject::new(1, "Warrior".to_string(), MirClass::Warrior, MirGender::Male);
        
        // Set up frame: start=100, count=4, skip=2
        // offset = count + skip = 6
        player.frame = Some(Frame::basic(100, 4, 2, 100));
        player.frame_index = 2;
        
        // Direction 0: 100 + (6 * 0) + 2 = 102
        assert_eq!(player.calc_draw_frame(0), 102);
        
        // Direction 3: 100 + (6 * 3) + 2 = 120
        assert_eq!(player.calc_draw_frame(3), 120);
    }
    
    #[test]
    fn test_calc_wing_frame() {
        let mut player = PlayerObject::new(1, "Wizard".to_string(), MirClass::Wizard, MirGender::Male);
        
        // Set up wing frame with effect data
        let mut frame = Frame::basic(0, 4, 0, 100);
        frame.effect_start = 200;
        frame.effect_count = 3;
        frame.effect_skip = 1;
        player.wing_frame = Some(frame);
        player.effect_frame_index = 1;
        
        // effect_offset = 3 + 1 = 4
        // Direction 0: 200 + (4 * 0) + 1 = 201
        assert_eq!(player.calc_wing_frame(0), 201);
        
        // Direction 2: 200 + (4 * 2) + 1 = 209
        assert_eq!(player.calc_wing_frame(2), 209);
    }

    // ==================== Spell Casting Tests ====================

    #[test]
    fn test_cast_spell_basic() {
        let mut player = PlayerObject::new(1, "Wizard".to_string(), MirClass::Wizard, MirGender::Male);
        
        // Cast FireBall
        let target = Point { x: 105, y: 105 };
        player.cast_spell(Spell::FireBall, 123, target, 3, vec![]);
        
        assert_eq!(player.spell, Some(Spell::FireBall));
        assert_eq!(player.target_id, 123);
        assert_eq!(player.target_point, target);
        assert_eq!(player.spell_level, 3);
        assert!(player.cast);
    }

    #[test]
    fn test_cast_spell_multi_target() {
        let mut player = PlayerObject::new(1, "Wizard".to_string(), MirClass::Wizard, MirGender::Male);
        
        // Cast multi-target spell
        let secondary_targets = vec![101, 102, 103];
        player.cast_spell(
            Spell::ThunderStorm,
            0,
            Point { x: 110, y: 110 },
            5,
            secondary_targets.clone(),
        );
        
        assert_eq!(player.spell, Some(Spell::ThunderStorm));
        assert_eq!(player.secondary_target_ids, secondary_targets);
    }

    #[test]
    fn test_next_spell_action() {
        let mut player = PlayerObject::new(1, "Wizard".to_string(), MirClass::Wizard, MirGender::Male);
        
        // Setup casting state
        player.cast_spell(Spell::Healing, 0, Point { x: 0, y: 0 }, 2, vec![]);
        assert!(player.cast);
        
        // Complete spell
        player.next_spell_action();
        assert!(!player.cast);
    }

    #[test]
    fn test_can_cast_spell() {
        let mut player = PlayerObject::new(1, "Taoist".to_string(), MirClass::Taoist, MirGender::Female);
        
        // Should be able to cast initially
        assert!(player.can_cast_spell());
        
        // Start casting
        player.cast_spell(Spell::Poisoning, 456, Point { x: 0, y: 0 }, 1, vec![]);
        
        // Should not be able to cast while already casting
        assert!(!player.can_cast_spell());
        
        // Complete spell
        player.next_spell_action();
        
        // Should be able to cast again
        assert!(player.can_cast_spell());
    }

    #[test]
    fn test_clear_spell_state() {
        let mut player = PlayerObject::new(1, "Wizard".to_string(), MirClass::Wizard, MirGender::Male);
        
        // Setup spell state
        player.cast_spell(
            Spell::FireBall,
            999,
            Point { x: 200, y: 200 },
            7,
            vec![1, 2, 3],
        );
        
        // Clear all spell state
        player.clear_spell_state();
        
        assert_eq!(player.spell, None);
        assert_eq!(player.spell_level, 0);
        assert!(!player.cast);
        assert_eq!(player.target_id, 0);
        assert_eq!(player.target_point, Point { x: 0, y: 0 });
        assert!(player.secondary_target_ids.is_empty());
    }

    // ==================== Drawing System Tests ====================

    #[test]
    fn test_draw_body() {
        let mut player = PlayerObject::new(1, "Warrior".to_string(), MirClass::Warrior, MirGender::Male);
        player.set_libraries();
        
        // Set up frame
        player.frame = Some(Frame::basic(100, 4, 2, 100));
        player.frame_index = 2;
        
        let draw_location = Point { x: 50, y: 50 };
        let params = player.draw_body(draw_location);
        
        assert_eq!(params.library_type, LibraryType::Body);
        assert_eq!(params.frame_index, 102); // 100 + (6 * 0) + 2 + armour_offset(0)
        assert_eq!(params.location, draw_location);
        assert!(!params.blend);
    }

    #[test]
    fn test_draw_head() {
        let mut player = PlayerObject::new(1, "Wizard".to_string(), MirClass::Wizard, MirGender::Female);
        player.set_libraries();
        
        player.frame = Some(Frame::basic(200, 4, 2, 100));
        player.frame_index = 1;
        
        let draw_location = Point { x: 100, y: 100 };
        let params = player.draw_head(draw_location);
        
        assert_eq!(params.library_type, LibraryType::Hair);
        // Female wizard has hair_offset = 808
        assert_eq!(params.frame_index, 201 + 808); // 200 + (6*0) + 1 + hair_offset = 1009
    }

    #[test]
    fn test_draw_weapon_none() {
        let mut player = PlayerObject::new(1, "Warrior".to_string(), MirClass::Warrior, MirGender::Male);
        player.weapon = -1; // No weapon
        
        let result = player.draw_weapon(Point { x: 0, y: 0 });
        assert!(result.is_none());
    }

    #[test]
    fn test_draw_weapon_equipped() {
        let mut player = PlayerObject::new(1, "Warrior".to_string(), MirClass::Warrior, MirGender::Male);
        player.weapon = 5;
        player.frame = Some(Frame::basic(300, 4, 2, 100));
        player.frame_index = 3;
        
        let draw_location = Point { x: 150, y: 150 };
        let params = player.draw_weapon(draw_location).unwrap();
        
        assert_eq!(params.library_type, LibraryType::Weapon);
        assert_eq!(params.frame_index, 303); // 300 + 3 + weapon_offset(0)
        assert_eq!(params.location, draw_location);
    }

    #[test]
    fn test_draw_wings() {
        let mut player = PlayerObject::new(1, "Wizard".to_string(), MirClass::Wizard, MirGender::Male);
        player.wing_effect = 5;
        
        // Set up wing frame
        let mut frame = Frame::basic(0, 4, 0, 100);
        frame.effect_start = 500;
        frame.effect_count = 3;
        frame.effect_skip = 1;
        player.wing_frame = Some(frame);
        player.effect_frame_index = 2;
        
        let draw_location = Point { x: 200, y: 200 };
        let params = player.draw_wings(draw_location).unwrap();
        
        assert_eq!(params.library_type, LibraryType::Wing);
        assert_eq!(params.frame_index, 502); // 500 + (4 * 0) + 2 + wing_offset(0)
        assert!(params.blend); // Wings always blend
    }

    #[test]
    fn test_draw_wings_none() {
        let mut player = PlayerObject::new(1, "Warrior".to_string(), MirClass::Warrior, MirGender::Male);
        player.wing_effect = 0; // No wings
        
        let result = player.draw_wings(Point { x: 0, y: 0 });
        assert!(result.is_none());
    }

    #[test]
    fn test_draw_mount() {
        let mut player = PlayerObject::new(1, "Warrior".to_string(), MirClass::Warrior, MirGender::Male);
        player.mount_type = 2;
        player.riding_mount = true;
        player.frame = Some(Frame::basic(1000, 4, 2, 100));
        player.frame_index = 1;
        
        let draw_location = Point { x: 250, y: 250 };
        let params = player.draw_mount(draw_location).unwrap();
        
        assert_eq!(params.library_type, LibraryType::Mount);
        assert_eq!(params.frame_index, 1001 - 416); // 1000 + 1 - 416 + mount_offset(0)
    }

    #[test]
    fn test_weapon_drawn_before_body() {
        let mut player = PlayerObject::new(1, "Warrior".to_string(), MirClass::Warrior, MirGender::Male);
        
        // Test left-side directions
        player.map_object.set_direction(MirDirection::Left);
        assert!(player.weapon_drawn_before_body());
        
        player.map_object.set_direction(MirDirection::Up);
        assert!(player.weapon_drawn_before_body());
        
        player.map_object.set_direction(MirDirection::UpLeft);
        assert!(player.weapon_drawn_before_body());
        
        // Test right-side directions
        player.map_object.set_direction(MirDirection::Right);
        assert!(!player.weapon_drawn_before_body());
        
        player.map_object.set_direction(MirDirection::Down);
        assert!(!player.weapon_drawn_before_body());
    }

    #[test]
    fn test_head_drawn_before_wings() {
        let mut player = PlayerObject::new(1, "Wizard".to_string(), MirClass::Wizard, MirGender::Female);
        
        // Test top-side directions (head before wings)
        player.map_object.set_direction(MirDirection::Up);
        assert!(player.head_drawn_before_wings());
        
        player.map_object.set_direction(MirDirection::UpLeft);
        assert!(player.head_drawn_before_wings());
        
        player.map_object.set_direction(MirDirection::Right);
        assert!(player.head_drawn_before_wings());
        
        // Test bottom-side directions (wings before head)
        player.map_object.set_direction(MirDirection::Down);
        assert!(!player.head_drawn_before_wings());
        
        player.map_object.set_direction(MirDirection::DownRight);
        assert!(!player.head_drawn_before_wings());
    }
}

/// Queued action for delayed execution
/// 
/// Mirrors C# Client/MirObjects/PlayerObject.cs, lines 5277-5286
#[derive(Debug, Clone)]
pub struct QueuedAction {
    /// C#: public MirAction Action;
    pub action: MirAction,
    
    /// C#: public Point Location;
    pub location: Point,
    
    /// C#: public MirDirection Direction;
    pub direction: MirDirection,
    
    // C#: public List<object> Params;
    // Note: Rarely used in C#, omitted for now
    // Can be added later with an enum for type-safe params if needed
    // pub params: Vec<Box<dyn Any>>,
}
