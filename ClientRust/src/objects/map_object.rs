// MapObject.rs - Base class for all game objects
// Mirrors Client/MirObjects/MapObject.cs
//
// ARCHITECTURE NOTE:
// This is a flattened game object that stores only common fields.
// It does NOT store network packet types directly.
// Network packets are only used in sync methods as data sources.

use std::time::Instant;

use mir2_shared::{
    enums::{BuffType, MirAction, MirDirection, PoisonType, Spell},
    Point,
};

use mir2_shared::packets::server::{ObjectPlayer, ObjectHero, ObjectMonster, ObjectNpc};

use super::frames::{AnimationState, AnimationStep};

/// Type of map object
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapObjectType {
    User,      // Player character (renamed from Player for consistency)
    Hero,      // Hero companion
    Monster,   // Monster/NPC (NPCs are stored as monsters)
}

/// Base game object that appears on the map.
/// Mirrors C# Client/MirObjects/MapObject.cs
/// 
/// C# Reference: MapObject.cs lines 11-141
#[derive(Debug, Clone)]
pub struct MapObject {
    // ==================== Identity ==================== 
    // C# line 60: public uint ObjectID;
    pub object_id: u32,
    object_type: MapObjectType,  // Internal type marker
    
    // C# line 61: public string Name = string.Empty;
    pub name: String,
    
    // ==================== Position ====================
    // C# line 62: public Point CurrentLocation, MapLocation;
    pub current_location: Point,  // 当前显示位置 (用于插值)
    pub map_location: Point,      // 地图格子位置 (服务器同步)
    
    // C# line 63: public MirDirection Direction;
    pub direction: MirDirection,
    
    // ==================== State Flags ====================
    // C# line 64: public bool Dead, Hidden, SitDown, Sneaking;
    pub dead: bool,
    pub hidden: bool,
    pub sit_down: bool,   // NEW
    pub sneaking: bool,   // NEW
    
    // C# line 65: public PoisonType Poison;
    pub poison: PoisonType,
    
    // C# line 66: public long DeadTime;
    pub dead_time: i64,   // NEW
    
    // C# line 67: public byte AI;
    pub ai: u8,
    
    // C# line 68: public bool InTrapRock;
    pub in_trap_rock: bool,  // NEW
    
    // C# line 69: public int JumpDistance;
    pub jump_distance: i32,  // NEW
    
    // ==================== Visual Blend ====================
    // C# line 71: public bool Blend = true;
    pub blend: bool,
    
    // ==================== Blind System ====================
    // C# line 73-74: public long BlindTime; public byte BlindCount;
    pub blind_time: i64,   // NEW
    pub blind_count: u8,   // NEW
    
    // ==================== Health/Mana Display ====================
    // C# line 76-89: public byte PercentHealth, PercentMana;
    pub percent_health: u8,  // NEW
    pub percent_mana: u8,    // NEW
    
    // C# line 88: public long HealthTime;
    pub health_time: i64,    // NEW
    
    // ==================== Action System (CORE!) ====================
    // C# line 97: public List<QueuedAction> ActionFeed = new List<QueuedAction>();
    pub action_feed: Vec<super::player_object::QueuedAction>,  // NEW
    
    // ==================== Effects & Buffs ====================
    // C# line 104: public List<Effect> Effects = new List<Effect>();
    pub effects: Vec<super::effect::Effect>,  // NEW
    
    // C# line 105: public List<BuffType> Buffs = new List<BuffType>();
    pub buffs: Vec<BuffType>,  // NEW (replaces BuffState abstraction)
    
    // ==================== Display Information ====================
    // C# line 109: public Color DrawColour, NameColour, LightColour;
    pub draw_colour: i32,   // NEW - ARGB
    pub name_colour: i32,   // Existing but renamed from name_colour
    pub light_colour: i32,  // NEW - ARGB
    
    // C# line 111-112: public long ChatTime;
    pub chat_time: i64,     // NEW
    
    // ==================== Drawing State ====================
    // C# line 113: public int DrawFrame, DrawWingFrame;
    pub draw_frame: i32,      // NEW
    pub draw_wing_frame: i32, // NEW
    
    // C# line 114: public Point DrawLocation, Movement, FinalDrawLocation, OffSetMove;
    pub draw_location: Point,       // NEW
    pub movement: Point,            // NEW
    pub final_draw_location: Point, // NEW
    pub offset_move: Point,         // NEW
    
    // C# line 115: public Rectangle DisplayRectangle;
    // Note: Rectangle will be calculated on-demand, not stored
    
    // C# line 116: public int Light, DrawY;
    pub light: i32,    // Changed from u8 to i32 to match C#
    pub draw_y: i32,   // NEW
    
    // ==================== Animation Timing ====================
    // C# line 117: public long NextMotion, NextMotion2;
    pub next_motion: i64,   // NEW
    pub next_motion2: i64,  // NEW
    
    // C# line 118: public MirAction CurrentAction;
    pub current_action: MirAction,  // NEW
    
    // C# line 119: public byte CurrentActionLevel;
    pub current_action_level: u8,  // NEW
    
    // C# line 120: public bool SkipFrames;
    pub skip_frames: bool,   // NEW
    
    // C# line 121: public FrameLoop FrameLoop = null;
    // FrameLoop will be handled separately
    
    // ==================== Sound ====================
    // C# line 124: public int StruckWeapon;
    pub struck_weapon: i32,  // NEW
    
    // ==================== Damage Display ====================
    // C# line 128: public List<Damage> Damages = new List<Damage>();
    pub damages: Vec<super::damage::Damage>,  // NEW
    
    // ==================== Internal State (not in C#) ====================
    animation: AnimationState,  // Keep for now, will migrate later
    last_update: Instant,       // Keep for Rust timing
}

impl MapObject {
    // ========================================
    // Factory Methods
    // ========================================
    
    /// Create a new user (player) map object with basic initialization
    /// C# reference: MapObject constructor (implicit)
    pub fn for_user(object_id: u32, name: String) -> Self {
        Self {
            // Identity
            object_id,
            object_type: MapObjectType::User,
            name,
            
            // Position
            current_location: Point::new(0, 0),
            map_location: Point::new(0, 0),
            direction: MirDirection::Up,
            
            // State flags
            dead: false,
            hidden: false,
            sit_down: false,
            sneaking: false,
            poison: PoisonType::empty(),
            dead_time: 0,
            ai: 0,
            in_trap_rock: false,
            jump_distance: 0,
            
            // Visual
            blend: true,  // C# default
            blind_time: 0,
            blind_count: 0,
            
            // Health/Mana
            percent_health: 0,
            percent_mana: 0,
            health_time: 0,
            
            // Action system
            action_feed: Vec::new(),
            
            // Effects & Buffs
            effects: Vec::new(),
            buffs: Vec::new(),
            
            // Display
            draw_colour: 0xFFFFFFFF_u32 as i32,  // White
            name_colour: 0xFFFFFFFF_u32 as i32,  // White
            light_colour: 0xFFFFFFFF_u32 as i32, // White
            chat_time: 0,
            
            // Drawing state
            draw_frame: 0,
            draw_wing_frame: 0,
            draw_location: Point::new(0, 0),
            movement: Point::new(0, 0), // Will be synced by set_current_location
            final_draw_location: Point::new(0, 0),
            offset_move: Point::new(0, 0),
            light: 0,
            draw_y: 0,
            
            // Animation timing
            next_motion: 0,
            next_motion2: 0,
            current_action: MirAction::Standing,
            current_action_level: 0,
            skip_frames: false,
            
            // Sound
            struck_weapon: 0,
            
            // Damage display
            damages: Vec::new(),
            
            // Internal state
            animation: AnimationState::default(),
            last_update: Instant::now(),
        }
    }
    
    /// Create a new hero map object with basic initialization
    /// C# reference: HeroObject constructor
    pub fn for_hero(object_id: u32, name: String) -> Self {
        let mut hero = Self::for_user(object_id, name);
        hero.object_type = MapObjectType::Hero;
        hero
    }
    
    /// Create a new monster/NPC map object with basic initialization
    /// C# reference: MonsterObject constructor
    pub fn for_monster(object_id: u32, name: String) -> Self {
        let mut monster = Self::for_user(object_id, name);
        monster.object_type = MapObjectType::Monster;
        monster
    }
    
    // ========================================
    // Convenience Methods - Create from Network Packets
    // ========================================
    
    /// Create a MapObject from an ObjectPlayer network packet
    pub fn from_player_packet(packet: &ObjectPlayer) -> (Self, SyncResult) {
        let mut obj = Self::for_user(packet.object_id, packet.name.clone());
        let sync_result = obj.sync_from_player_packet(packet);
        (obj, sync_result)
    }
    
    /// Create a MapObject from an ObjectHero network packet
    pub fn from_hero_packet(packet: &ObjectHero) -> (Self, SyncResult) {
        let mut obj = Self::for_hero(packet.player.object_id, packet.player.name.clone());
        let sync_result = obj.sync_from_hero_packet(packet);
        (obj, sync_result)
    }
    
    /// Create a MapObject from an ObjectMonster network packet
    pub fn from_monster_packet(packet: &ObjectMonster) -> (Self, SyncResult) {
        let mut obj = Self::for_monster(packet.object_id, packet.name.clone());
        let sync_result = obj.sync_from_monster_packet(packet);
        (obj, sync_result)
    }
    
    /// Create a MapObject from an ObjectNpc network packet
    /// C# reference: NPCObject.Load()
    pub fn from_npc_packet(packet: &ObjectNpc) -> (Self, SyncResult) {
        let mut obj = Self::for_monster(packet.object_id, packet.name.clone());
        let location = Point::new(packet.location_x, packet.location_y);
        obj.current_location = location;
        obj.map_location = location;
        obj.movement = location; // 🔧 Sync movement with current_location
        obj.direction = packet.direction;
        obj.name_colour = packet.name_colour;
        // NPCs have no animation, buffs, or special states
        (obj, SyncResult {
            buff_delta: BuffDelta::default(),
            action_before: MirAction::Standing,
            action_after: MirAction::Standing,
        })
    }

    // ========================================
    // Sync Methods - Update from Network Packets
    // ========================================
    
    /// Sync MapObject state from an ObjectPlayer network packet.
    /// This is the ONLY place where ObjectPlayer data enters the game object layer.
    /// C# reference: PlayerObject.Load() - MapObject.cs line 98
    pub fn sync_from_player_packet(&mut self, packet: &ObjectPlayer) -> SyncResult {
        let action_before = self.current_action;
        
        // Update identity and display
        self.name = packet.name.clone();
        self.name_colour = packet.name_colour;
        
        // Update position
        let location = Point::new(packet.location_x, packet.location_y);
        self.current_location = location;
        self.map_location = location;
        self.movement = location; // 🔧 Sync movement with current_location
        self.direction = packet.direction;
        
        // Update state flags
        self.dead = packet.dead;
        self.hidden = packet.hidden;
        self.poison = packet.poison;
        self.light = packet.light as i32;
        
        // Update buffs (compare old vs new)
        let buff_delta = self.update_buffs_internal(&packet.buffs);
        
        // Update animation based on state
        self.animation.update_from_state(
            packet.dead,
            packet.hidden,
            packet.fishing,
            packet.riding_mount
        );
        
        let action_after = self.animation.current_action();
        self.current_action = action_after;
        self.last_update = Instant::now();
        
        SyncResult {
            buff_delta,
            action_before,
            action_after,
        }
    }
    
    /// Sync MapObject state from an ObjectHero network packet
    pub fn sync_from_hero_packet(&mut self, packet: &ObjectHero) -> SyncResult {
        // Hero contains an ObjectPlayer, so delegate to it
        self.sync_from_player_packet(&packet.player)
    }
    
    /// Sync MapObject state from an ObjectMonster network packet
    /// C# reference: MonsterObject.Load()
    pub fn sync_from_monster_packet(&mut self, packet: &ObjectMonster) -> SyncResult {
        let action_before = self.current_action;
        
        // Update identity and display
        self.name = packet.name.clone();
        self.name_colour = packet.name_colour;
        
        // Update position
        let location = Point::new(packet.location_x, packet.location_y);
        self.current_location = location;
        self.map_location = location;
        self.movement = location; // 🔧 Sync movement with current_location
        self.direction = packet.direction;
        
        // Update state flags
        self.dead = packet.dead;
        self.hidden = packet.hidden;
        self.poison = packet.poison;
        self.ai = packet.ai;
        self.light = packet.light as i32;
        
        // Update buffs
        let buff_delta = self.update_buffs_internal(&packet.buffs);
        
        // Update animation
        let new_action = if packet.dead {
            MirAction::Dead
        } else {
            action_before
        };
        self.animation.set_action(new_action);
        
        let action_after = self.animation.current_action();
        self.current_action = action_after;
        self.last_update = Instant::now();
        
        SyncResult {
            buff_delta,
            action_before,
            action_after,
        }
    }

    // ========================================
    // Getters - Identity and Type
    // ========================================
    
    /// Get the unique object ID
    pub fn object_id(&self) -> u32 {
        self.object_id
    }

    /// Get the type of this map object
    pub fn object_type(&self) -> MapObjectType {
        self.object_type
    }
    
    // ========================================
    // Getters - Position and Direction
    // ========================================

    /// Get the current display location (for rendering with interpolation)
    /// C# reference: CurrentLocation
    pub fn current_location(&self) -> Point {
        self.current_location
    }
    
    /// Get the map grid location (server-synchronized position)
    /// C# reference: MapLocation
    pub fn map_location(&self) -> Point {
        self.map_location
    }
    
    /// Legacy compatibility - returns current_location
    pub fn location(&self) -> Point {
        self.current_location
    }

    /// Get the current facing direction
    pub fn direction(&self) -> MirDirection {
        self.direction
    }
    
    // ========================================
    // Getters - Display Information
    // ========================================
    
    /// Get the name of the object
    pub fn name(&self) -> &str {
        &self.name
    }
    
    /// Get the name color (ARGB format)
    pub fn name_colour(&self) -> i32 {
        self.name_colour
    }
    
    // Legacy compatibility method
    pub fn name_colour_argb(&self) -> i32 {
        self.name_colour
    }
    
    // ========================================
    // Getters - State
    // ========================================

    /// Check if the object is dead
    pub fn is_dead(&self) -> bool {
        self.dead
    }

    /// Check if the object is hidden
    pub fn is_hidden(&self) -> bool {
        self.hidden
    }
    
    /// Get the poison status
    pub fn poison(&self) -> PoisonType {
        self.poison
    }
    
    /// Get AI type (for monsters, 0 for players)
    pub fn ai(&self) -> u8 {
        self.ai
    }
    
    /// Get light radius
    pub fn light(&self) -> i32 {
        self.light
    }
    
    // ========================================
    // Getters - Animation
    // ========================================

    /// Get the current animation action
    pub fn current_action(&self) -> MirAction {
        self.animation.current_action()
    }
    
    // ========================================
    // Getters - Buffs
    // ========================================
    
    /// Get active buffs
    /// C# reference: public List<BuffType> Buffs
    pub fn buffs(&self) -> &[BuffType] {
        &self.buffs
    }
    
    /// Check if a specific buff is active
    pub fn has_buff(&self, buff_type: BuffType) -> bool {
        self.buffs.contains(&buff_type)
    }
    
    // ========================================
    // Getters - Health/Mana
    // ========================================
    
    /// Get health percentage (0-100)
    /// C# reference: public byte PercentHealth
    pub fn percent_health(&self) -> u8 {
        self.percent_health
    }
    
    /// Get mana percentage (0-100)
    /// C# reference: public byte PercentMana
    pub fn percent_mana(&self) -> u8 {
        self.percent_mana
    }
    
    // ========================================
    // Getters - Drawing
    // ========================================
    
    /// Get draw location (for rendering)
    /// C# reference: public Point DrawLocation
    pub fn draw_location(&self) -> Point {
        self.draw_location
    }
    
    /// Get current draw frame
    /// C# reference: public int DrawFrame
    pub fn draw_frame(&self) -> i32 {
        self.draw_frame
    }
    
    /// Get current action
    /// C# reference: public MirAction CurrentAction
    pub fn get_current_action(&self) -> MirAction {
        self.current_action
    }

    // ========================================
    // Setters - Position and Direction
    // ========================================
    
    /// Set the current display location
    pub fn set_current_location(&mut self, location: Point) {
        self.current_location = location;
        // 🔧 CRITICAL FIX: Synchronize movement with current_location
        // When not actively moving, movement should equal current_location for proper rendering
        // C# Reference: PlayerObject.cs line 838 - "Movement = CurrentLocation" in default case
        self.movement = location;
    }
    
    /// Set the map grid location
    pub fn set_map_location(&mut self, location: Point) {
        self.map_location = location;
    }
    
    /// Legacy compatibility - sets current_location
    pub fn set_location(&mut self, location: Point) {
        self.current_location = location;
        // 🔧 Sync movement with current_location (same as set_current_location)
        self.movement = location;
    }
    
    /// Set the facing direction
    pub fn set_direction(&mut self, direction: MirDirection) {
        self.direction = direction;
    }
    
    // ========================================
    // Setters - Health/Mana
    // ========================================
    
    /// Set health percentage
    /// C# reference: public virtual byte PercentHealth { set; }
    pub fn set_percent_health(&mut self, percent: u8) {
        if self.percent_health != percent {
            self.percent_health = percent;
        }
    }
    
    /// Set mana percentage
    /// C# reference: public virtual byte PercentMana { set; }
    pub fn set_percent_mana(&mut self, percent: u8) {
        if self.percent_mana != percent {
            self.percent_mana = percent;
        }
    }
    
    // ========================================
    // Setters - Display Information
    // ========================================
    
    /// Set the name of the object
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }
    
    /// Set the name color (ARGB format)
    pub fn set_name_colour(&mut self, colour: i32) {
        self.name_colour = colour;
    }
    
    // Legacy compatibility method
    pub fn set_name_colour_argb(&mut self, colour: i32) -> i32 {
        let previous = self.name_colour;
        self.name_colour = colour;
        previous
    }
    
    /// Set the level (for display purposes - actual level stored in UserInformation)
    pub fn set_level(&mut self, _level: u16) {
        // Level is stored in UserInformation/character state, not in MapObject
        // This is a stub method for UI compatibility
    }
    
    /// Set the guild name
    pub fn set_guild_name(&mut self, _guild_name: String) -> Option<String> {
        // Guild name is not currently stored in MapObject
        // This is a stub method for UI compatibility
        None
    }
    
    // ========================================
    // Setters - State
    // ========================================
    
    /// Set the dead state
    pub fn set_dead(&mut self, dead: bool) {
        self.dead = dead;
    }
    
    /// Set the hidden state
    pub fn set_hidden(&mut self, hidden: bool) {
        self.hidden = hidden;
    }
    
    /// Set the poison status
    pub fn set_poison(&mut self, poison: PoisonType) {
        self.poison = poison;
    }
    
    /// Set AI type (for monsters)
    pub fn set_ai(&mut self, ai: u8) {
        self.ai = ai;
    }
    
    /// Set light radius
    pub fn set_light(&mut self, light: i32) {
        self.light = light;
    }
    
    // ========================================
    // Animation Control
    // ========================================
    
    /// Set the current animation action
    pub fn set_action(&mut self, action: MirAction) {
        self.animation.set_action(action);
    }
    
    /// Advance animation by delta time, returns animation step info
    pub fn advance(&mut self, delta_ms: u32) -> AnimationStep {
        let step = self.animation.tick(delta_ms);
        if step.frames_advanced > 0 || step.completed_cycles > 0 {
            self.last_update = Instant::now();
        }
        step
    }
    
    // ========================================
    // Movement Control
    // ========================================
    
    /// Check if the object is currently moving
    pub fn is_moving(&self) -> bool {
        self.current_location != self.map_location
    }
    
    /// Start moving to a new location
    /// This sets up the movement interpolation
    pub fn start_move(&mut self, target: Point) {
        if self.map_location != target {
            self.current_location = self.map_location; // Start from current position
            self.map_location = target;
            
            // 🔧 CRITICAL: Movement is the rendering position, not a delta vector
            // It should be set to current_location at start, then interpolate towards map_location
            // C# Reference: PlayerObject.cs line 838 - Movement = CurrentLocation
            self.movement = self.current_location;
            
            tracing::trace!("Object {} starting move from ({},{}) to ({},{})",
                self.object_id, 
                self.current_location.x, self.current_location.y,
                target.x, target.y
            );
        }
    }
    
    /// Update movement interpolation
    /// Returns true if the object reached its destination
    /// 
    /// C# Reference: MapObject.cs ProcessMovement() and MoveTo()
    pub fn update_movement(&mut self, delta_time: f32) -> bool {
        if !self.is_moving() {
            return false;
        }
        
        // Movement speed in cells per second
        const MOVEMENT_SPEED: f32 = 4.0; // Adjust this for desired speed
        
        // Calculate distance to move this frame
        let distance_to_move = MOVEMENT_SPEED * delta_time;
        
        // Calculate total distance to target
        let dx = (self.map_location.x - self.current_location.x) as f32;
        let dy = (self.map_location.y - self.current_location.y) as f32;
        let total_distance = (dx * dx + dy * dy).sqrt();
        
        if total_distance <= distance_to_move {
            // Reached destination
            self.current_location = self.map_location;
            self.movement = self.map_location; // 🔧 Sync movement with destination
            
            tracing::trace!("Object {} reached destination ({},{})",
                self.object_id, self.map_location.x, self.map_location.y
            );
            
            true
        } else {
            // Continue interpolating
            let progress = distance_to_move / total_distance;
            let new_x = self.current_location.x as f32 + dx * progress;
            let new_y = self.current_location.y as f32 + dy * progress;
            
            let new_current = Point::new(new_x as i32, new_y as i32);
            self.current_location = new_current;
            self.movement = new_current; // 🔧 Sync movement during interpolation
            false
        }
    }
    
    /// Update draw location based on current location
    /// This should be called after movement updates to ensure correct rendering position
    /// 
    /// C# Reference: MapObject.cs SetLibraries() and DrawLocation calculation
    pub fn update_draw_location(&mut self) {
        // In the original C# code, DrawLocation is calculated based on:
        // - CurrentLocation (grid position)
        // - OffSetMove (pixel offset for smooth movement)
        // - FinalDrawLocation (adjusted for object height/size)
        
        // For now, we'll use a simple mapping from grid to pixel coordinates
        // Each grid cell is typically 48x32 pixels in isometric view
        const CELL_WIDTH: i32 = 48;
        const CELL_HEIGHT: i32 = 32;
        
        // Convert grid coordinates to isometric screen coordinates
        let screen_x = (self.current_location.x - self.current_location.y) * (CELL_WIDTH / 2);
        let screen_y = (self.current_location.x + self.current_location.y) * (CELL_HEIGHT / 2);
        
        self.draw_location = Point::new(screen_x, screen_y);
        self.final_draw_location = self.draw_location; // Will be adjusted by object-specific rendering
        
        // Update DrawY for depth sorting
        self.draw_y = screen_y;
    }
    
    /// Teleport to a location instantly (no interpolation)
    pub fn teleport_to(&mut self, location: Point) {
        self.map_location = location;
        self.current_location = location;
        self.movement = location; // 🔧 Sync movement with location
        self.update_draw_location();
        
        tracing::debug!("Object {} teleported to ({},{})",
            self.object_id, location.x, location.y
        );
    }
    
    // ========================================
    // Buff Management
    // ========================================
    
    /// Update buffs from a list, returns the delta (added/removed)
    /// C# reference: Buffs property setter (implicit comparison)
    pub fn update_buffs(&mut self, buffs: &[BuffType]) -> BuffDelta {
        self.update_buffs_internal(buffs)
    }
    
    /// Set buffs (replaces existing buffs) - legacy compatibility
    pub fn set_buffs(&mut self, buffs: Vec<BuffType>) {
        self.buffs = buffs;
    }
    
    /// Add a single buff to the object
    /// If the buff already exists, this is a no-op
    pub fn add_buff(&mut self, buff: BuffType) {
        if !self.buffs.contains(&buff) {
            self.buffs.push(buff);
            tracing::debug!("Added buff {:?} to object {}", buff, self.object_id);
        }
    }
    
    /// Remove a single buff from the object
    /// If the buff doesn't exist, this is a no-op
    pub fn remove_buff(&mut self, buff: BuffType) {
        if let Some(pos) = self.buffs.iter().position(|b| *b == buff) {
            self.buffs.remove(pos);
            tracing::debug!("Removed buff {:?} from object {}", buff, self.object_id);
        }
    }
    
    /// Clear all buffs
    pub fn clear_buffs(&mut self) {
        if !self.buffs.is_empty() {
            self.buffs.clear();
            tracing::debug!("Cleared all buffs from object {}", self.object_id);
        }
    }
    
    /// Internal method to update buffs and calculate delta
    fn update_buffs_internal(&mut self, incoming: &[BuffType]) -> BuffDelta {
        let mut added = Vec::new();
        for buff in incoming {
            if !self.buffs.contains(buff) {
                added.push(*buff);
            }
        }

        let mut removed = Vec::new();
        for buff in &self.buffs {
            if !incoming.contains(buff) {
                removed.push(*buff);
            }
        }

        self.buffs = incoming.to_vec();

        BuffDelta { added, removed }
    }

    pub fn apply_attack(
        &mut self,
        direction: MirDirection,
        location: Point,
        spell: Spell,
        level: u8,
        attack_type: u8,
    ) -> AttackOutcome {
        let action = self.attack_action_for_type(attack_type);
        let transition = self.apply_action(action, direction, location);
        AttackOutcome {
            transition,
            spell,
            level,
            attack_type,
        }
    }

    pub fn apply_struck(
        &mut self,
        direction: MirDirection,
        location: Point,
        attacker_id: u32,
    ) -> StruckOutcome {
        let transition = self.apply_action(MirAction::Struck, direction, location);
        StruckOutcome {
            transition,
            attacker_id,
        }
    }

    // ========================================
    // Lifecycle Management
    // ========================================
    
    /// C#: Remove(), lines 153-176
    /// Remove this object from the game world.
    /// 
    /// Note: The caller is responsible for:
    /// - Clearing static references (MouseObject, TargetObject, MagicObject, etc)
    /// - Removing from MapControl.Objects and MapControl.ObjectsList
    /// - Calling GameScene.Scene.MapControl.RemoveObject(this)
    /// - Clearing HeroObject if this is the hero
    /// - Clearing NPCID if this is an NPC
    /// 
    /// These responsibilities are handled by the caller in Rust (typically ObjectManager).
    pub fn remove(&mut self) {
        // Clear any pending actions (C# line 97: ActionFeed)
        self.action_feed.clear();
        
        // Clear effects (C# line 104: Effects)
        self.effects.clear();
        
        // Clear buffs (C# line 105: Buffs)
        self.buffs.clear();
        
        // Reset state
        self.dead_time = 0;
    }

    // Buff Effect Management
    // ========================================
    
    /// C#: AddBuffEffect(), lines 213-352 (140 lines!)
    /// Add visual effect for a buff type
    /// 
    /// TODO: Implement when Effect system is complete
    /// Dependencies:
    /// - Effect::new_buff() method
    /// - Libraries resource manager (Magic3, etc)
    /// - Sound system integration
    /// 
    /// C# Implementation summary:
    /// - 30+ buff types with unique effects
    /// - Each buff has specific library, frame range, duration
    /// - Some buffs trigger sounds
    /// - Some buffs modify object state (Sprint, Sneaking, etc)
    pub fn add_buff_effect(&mut self, _buff_type: BuffType) {
        // TODO: Implement
        // Example from C#:
        // case BuffType.Fury:
        //     Effects.Add(new BuffEffect(Libraries.Magic3, 190, 7, 1400, this, true, type) { Repeat = true });
        //     break;
    }
    
    /// C#: RemoveBuffEffect(), lines 353-445 (93 lines)
    /// Remove visual effect for a buff type
    /// 
    /// TODO: Implement when Effect system is complete
    /// Dependencies: Same as add_buff_effect()
    /// 
    /// C# Implementation summary:
    /// - Searches Effects list for matching BuffType
    /// - Removes matching effects
    /// - Some buffs modify object state on removal (Sprint=false, Sneaking=false, etc)
    pub fn remove_buff_effect(&mut self, _buff_type: BuffType) {
        // TODO: Implement
        // Example from C#:
        // for (int i = Effects.Count - 1; i >= 0; i--)
        // {
        //     BuffEffect effect = Effects[i] as BuffEffect;
        //     if (effect == null || effect.BuffType != type) continue;
        //     effect.Clear();
        //     Effects.RemoveAt(i);
        // }
    }

    // Action Application
    // ========================================
    
    /// Apply a generic action to the object
    /// C# reference: SetAction() and related methods
    pub fn apply_action(
        &mut self,
        action: MirAction,
        direction: MirDirection,
        location: Point,
    ) -> ActionResult {
        let action_before = self.current_action;
        let direction_before = self.direction;
        let location_before = self.current_location;
        
        let action_changed = self.animation.ensure_action(action);
        self.current_action = action;
        self.direction = direction;
        self.current_location = location;
        self.map_location = location;
        
        let action_after = self.current_action;
        self.last_update = Instant::now();

        ActionResult {
            action_before,
            action_after,
            direction_before,
            direction_after: self.direction,
            location_before,
            location_after: self.current_location,
            action_changed,
        }
    }

    /// Apply death to the object
    pub fn apply_death(&mut self, direction: MirDirection, location: Point) -> ActionResult {
        self.dead = true;
        self.apply_action(MirAction::Die, direction, location)
    }
    
    /// Get the appropriate attack action for the given attack type
    fn attack_action_for_type(&self, attack_type: u8) -> MirAction {
        match self.object_type {
            MapObjectType::User => MirAction::Attack1,
            MapObjectType::Hero => match attack_type {
                1 => MirAction::Attack2,
                2 => MirAction::Attack3,
                3 => MirAction::Attack4,
                4 => MirAction::Attack5,
                _ => MirAction::Attack1,
            },
            MapObjectType::Monster => match attack_type {
                1 => MirAction::Attack2,
                2 => MirAction::Attack3,
                3 => MirAction::Attack4,
                _ => MirAction::Attack1,
            },
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct BuffDelta {
    pub added: Vec<BuffType>,
    pub removed: Vec<BuffType>,
}

impl BuffDelta {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty()
    }
}

// Result types
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub buff_delta: BuffDelta,
    pub action_before: MirAction,
    pub action_after: MirAction,
}

impl SyncResult {
    pub fn action_changed(&self) -> bool {
        self.action_before != self.action_after
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ActionResult {
    pub action_before: MirAction,
    pub action_after: MirAction,
    pub direction_before: MirDirection,
    pub direction_after: MirDirection,
    pub location_before: Point,
    pub location_after: Point,
    pub action_changed: bool,
}

impl ActionResult {
    pub fn moved(&self) -> bool {
        self.location_before != self.location_after
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AttackOutcome {
    pub transition: ActionResult,
    pub spell: Spell,
    pub level: u8,
    pub attack_type: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct StruckOutcome {
    pub transition: ActionResult,
    pub attacker_id: u32,
}

#[derive(Debug, Clone)]
pub struct ObjectUpdateOutcome {
    pub created: bool,
    pub object_type: MapObjectType,
    pub sync: SyncResult,
}

#[derive(Debug, Clone)]
pub struct ObjectActionOutcome {
    pub object_id: u32,
    pub object_type: MapObjectType,
    pub result: ActionResult,
}

#[derive(Debug, Clone)]
pub struct ObjectAttackOutcome {
    pub object_id: u32,
    pub object_type: MapObjectType,
    pub attack: AttackOutcome,
}

#[derive(Debug, Clone)]
pub struct ObjectStruckOutcome {
    pub object_id: u32,
    pub object_type: MapObjectType,
    pub struck: StruckOutcome,
}

#[derive(Debug, Clone)]
pub struct ObjectDeathOutcome {
    pub object_id: u32,
    pub object_type: MapObjectType,
    pub death_type: u8,
    pub transition: Option<ActionResult>,
    pub removed: bool,
    pub location: Point,
    pub direction: MirDirection,
}
