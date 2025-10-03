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
/// This is a flattened structure containing only common fields shared by all object types.
/// Type-specific data (e.g., player class, monster image) is stored in the wrapping objects
/// (UserObject, HeroObject, MonsterObject, etc.).
#[derive(Debug, Clone)]
pub struct MapObject {
    // === Identity ===
    object_id: u32,
    object_type: MapObjectType,
    
    // === Position and Direction ===
    location: Point,
    direction: MirDirection,
    
    // === Display Information ===
    name: String,
    name_colour: i32,  // ARGB color
    
    // === State Flags (common to all objects) ===
    dead: bool,
    hidden: bool,
    poison: PoisonType,
    
    // === Monster/NPC specific (default for players) ===
    ai: u8,      // AI type for monsters, 0 for players
    light: u8,   // Light radius
    
    // === Private State ===
    buffs: BuffState,
    animation: AnimationState,
    last_update: Instant,
}

impl MapObject {
    // ========================================
    // Factory Methods
    // ========================================
    
    /// Create a new user (player) map object with basic initialization
    pub fn for_user(object_id: u32, name: String) -> Self {
        Self {
            object_id,
            object_type: MapObjectType::User,
            location: Point::new(0, 0),
            direction: MirDirection::Up,
            name,
            name_colour: 0xFFFFFF_u32 as i32, // White by default
            dead: false,
            hidden: false,
            poison: PoisonType::empty(),
            ai: 0,
            light: 0,
            buffs: BuffState::default(),
            animation: AnimationState::default(),
            last_update: Instant::now(),
        }
    }
    
    /// Create a new hero map object with basic initialization
    pub fn for_hero(object_id: u32, name: String) -> Self {
        Self {
            object_id,
            object_type: MapObjectType::Hero,
            location: Point::new(0, 0),
            direction: MirDirection::Up,
            name,
            name_colour: 0xFFFFFF_u32 as i32,
            dead: false,
            hidden: false,
            poison: PoisonType::empty(),
            ai: 0,
            light: 0,
            buffs: BuffState::default(),
            animation: AnimationState::default(),
            last_update: Instant::now(),
        }
    }
    
    /// Create a new monster/NPC map object with basic initialization
    pub fn for_monster(object_id: u32, name: String) -> Self {
        Self {
            object_id,
            object_type: MapObjectType::Monster,
            location: Point::new(0, 0),
            direction: MirDirection::Up,
            name,
            name_colour: 0xFFFFFF_u32 as i32,
            dead: false,
            hidden: false,
            poison: PoisonType::empty(),
            ai: 0,
            light: 0,
            buffs: BuffState::default(),
            animation: AnimationState::default(),
            last_update: Instant::now(),
        }
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
    pub fn from_npc_packet(packet: &ObjectNpc) -> (Self, SyncResult) {
        let mut obj = Self::for_monster(packet.object_id, packet.name.clone());
        obj.location = Point::new(packet.location_x, packet.location_y);
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
    pub fn sync_from_player_packet(&mut self, packet: &ObjectPlayer) -> SyncResult {
        let action_before = self.animation.current_action();
        
        // Update basic fields
        self.location = Point::new(packet.location_x, packet.location_y);
        self.direction = packet.direction;
        self.name = packet.name.clone();
        self.name_colour = packet.name_colour;
        self.dead = packet.dead;
        self.hidden = packet.hidden;
        self.poison = packet.poison;
        self.light = packet.light;
        
        // Update buffs
        let buff_delta = self.buffs.replace(&packet.buffs);
        
        // Update animation based on state
        self.animation.update_from_state(
            packet.dead,
            packet.hidden,
            packet.fishing,
            packet.riding_mount
        );
        
        let action_after = self.animation.current_action();
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
    pub fn sync_from_monster_packet(&mut self, packet: &ObjectMonster) -> SyncResult {
        let action_before = self.animation.current_action();
        
        // Update basic fields
        self.location = Point::new(packet.location_x, packet.location_y);
        self.direction = packet.direction;
        self.name = packet.name.clone();
        self.name_colour = packet.name_colour;
        self.dead = packet.dead;
        self.hidden = packet.hidden;
        self.poison = packet.poison;
        self.ai = packet.ai;
        self.light = packet.light;
        
        // Update buffs
        let buff_delta = self.buffs.replace(&packet.buffs);
        
        // Update animation
        let new_action = if packet.dead {
            MirAction::Dead
        } else {
            action_before
        };
        self.animation.set_action(new_action);
        
        let action_after = self.animation.current_action();
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

    /// Get the current location on the map
    pub fn location(&self) -> Point {
        self.location
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
    pub fn light(&self) -> u8 {
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
    pub fn buffs(&self) -> &[BuffType] {
        &self.buffs.active
    }
    
    /// Check if a specific buff is active
    pub fn has_buff(&self, buff_type: BuffType) -> bool {
        self.buffs.active.contains(&buff_type)
    }

    // ========================================
    // Setters - Position and Direction
    // ========================================
    
    /// Set the location on the map
    pub fn set_location(&mut self, location: Point) {
        self.location = location;
    }
    
    /// Set the facing direction
    pub fn set_direction(&mut self, direction: MirDirection) {
        self.direction = direction;
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
    pub fn set_light(&mut self, light: u8) {
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
    // Buff Management
    // ========================================
    
    /// Update buffs from a list, returns the delta (added/removed)
    pub fn update_buffs(&mut self, buffs: &[BuffType]) -> BuffDelta {
        self.buffs.replace(buffs)
    }
    
    /// Set buffs (replaces existing buffs) - legacy compatibility
    pub fn set_buffs(&mut self, buffs: Vec<BuffType>) {
        self.buffs.replace(&buffs);
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
    // Action Application
    // ========================================
    
    /// Apply a generic action to the object
    pub fn apply_action(
        &mut self,
        action: MirAction,
        direction: MirDirection,
        location: Point,
    ) -> ActionResult {
        let action_before = self.animation.current_action();
        let direction_before = self.direction;
        let location_before = self.location;
        
        let action_changed = self.animation.ensure_action(action);
        self.direction = direction;
        self.location = location;
        
        let action_after = self.animation.current_action();
        self.last_update = Instant::now();

        ActionResult {
            action_before,
            action_after,
            direction_before,
            direction_after: self.direction,
            location_before,
            location_after: self.location,
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

// Buff management
#[derive(Debug, Clone, Default)]
struct BuffState {
    active: Vec<BuffType>,
}

impl BuffState {
    fn replace(&mut self, incoming: &[BuffType]) -> BuffDelta {
        let mut added = Vec::new();
        for buff in incoming {
            if !self.active.contains(buff) {
                added.push(*buff);
            }
        }

        let mut removed = Vec::new();
        for buff in &self.active {
            if !incoming.contains(buff) {
                removed.push(*buff);
            }
        }

        self.active = incoming.to_vec();

        BuffDelta { added, removed }
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
