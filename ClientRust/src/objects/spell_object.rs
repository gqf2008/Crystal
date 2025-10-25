// SpellObject.rs - Flying spell projectiles
// Mirrors Client/MirObjects/SpellObject.cs

use ggez::{Context, GameResult};
use ggez::graphics::Canvas;
use mir2_shared::{
    enums::{Spell, SpellEffect},
    Point,
};

use super::map_object::MapObject;
use super::drawable::DrawableMapObject;
// TODO: ObjectSpell not yet implemented in protocol.rs
// use crate::network::protocol::ObjectSpell;

// TODO: FrameSet not yet implemented
// use super::frames::FrameSet;

/// Spell object - represents flying spell projectiles (fireballs, lightning, etc.)
#[derive(Debug, Clone)]
pub struct SpellObject {
    // Inherited from MapObject
    pub map_object: MapObject,
    
    // Spell specific fields
    pub spell: Spell,
    pub effect: SpellEffect,
    pub caster_id: u32,
    pub target_id: u32,
    pub target_location: Point,
    
    // Movement
    pub start_location: Point,
    pub velocity: Point, // Speed in x/y directions
    pub speed: i32,      // Pixels per frame
    pub frame_interval: i32,
    pub frame_index: i32,
    
    // Animation
    // TODO: FrameSet not yet implemented
    // pub frames: FrameSet,
    pub frame_count: i32,
    pub repeat: bool,
    
    // State
    pub expired: bool,
    pub hit_time: i64,
}

impl SpellObject {
    /// Create a new spell object
    pub fn new(object_id: u32, spell: Spell) -> Self {
        Self {
            map_object: MapObject::for_monster(object_id, String::new()), // Spells don't need full MapObject
            spell,
            effect: SpellEffect::None,
            caster_id: 0,
            target_id: 0,
            target_location: Point::new(0, 0),
            start_location: Point::new(0, 0),
            velocity: Point::new(0, 0),
            speed: 10,
            frame_interval: 100,
            frame_index: 0,
            // frames: FrameSet::default(), // TODO: FrameSet not yet implemented
            frame_count: 0,
            repeat: false,
            expired: false,
            hit_time: 0,
        }
    }

    /// Load spell information from server
    // TODO: ObjectSpell not yet implemented
    /*
    pub fn load(&mut self, info: &ObjectSpell) {
        self.spell = info.spell;
        self.map_object.current_location = info.location;
        self.map_object.map_location = info.location;
        self.start_location = info.location;
        self.target_location = info.target;
        self.map_object.direction = info.direction;
        self.caster_id = info.caster_id;
        self.target_id = info.target_id;
        
        // Calculate velocity based on direction
        self.calculate_velocity();
        
        // Set spell-specific parameters
        self.configure_spell();
    }
    */

    /// Update spell position
    pub fn update_position(&mut self, current_time: i64) {
        if self.expired {
            return;
        }

        // Move towards target
        let mut loc = self.map_object.location();
        loc.x += self.velocity.x;
        loc.y += self.velocity.y;
        self.map_object.set_location(loc);

        // Check if reached target
        if self.has_reached_target() {
            self.on_hit(current_time);
        }

        // Update animation frame
        self.update_frame(current_time);
    }

    /// Check if spell has reached target location
    fn has_reached_target(&self) -> bool {
        let loc = self.map_object.location();
        let dx = (loc.x - self.target_location.x).abs();
        let dy = (loc.y - self.target_location.y).abs();
        dx <= self.speed && dy <= self.speed
    }

    /// Handle spell hit
    fn on_hit(&mut self, current_time: i64) {
        self.expired = true;
        self.hit_time = current_time;
        
        // TODO: Create explosion effect at target location
        // TODO: Play hit sound
    }

    /// Update animation frame
    fn update_frame(&mut self, _current_time: i64) {
        self.frame_index += 1;
        
        if self.frame_index >= self.frame_count {
            if self.repeat {
                self.frame_index = 0;
            } else {
                self.expired = true;
            }
        }
    }

    /// Check if spell should be removed
    pub fn should_remove(&self, current_time: i64) -> bool {
        if !self.expired {
            return false;
        }
        
        // Remove after 1 second of expiring
        current_time - self.hit_time > 1000
    }

    /// Get spell's current screen position (in pixels)
    pub fn get_screen_position(&self) -> Point {
        // Convert tile position to pixel position
        // Assuming 48x32 tile size
        let loc = self.map_object.location();
        Point::new(
            loc.x * 48,
            loc.y * 32,
        )
    }

    /// Check collision with target object
    pub fn check_collision(&self, target_location: Point) -> bool {
        if self.expired {
            return false;
        }
        
        let loc = self.map_object.location();
        let dx = (loc.x - target_location.x).abs();
        let dy = (loc.y - target_location.y).abs();
        
        // Within 1 tile range
        dx <= 1 && dy <= 1
    }
}

// Implement DrawableMapObject trait for SpellObject
impl DrawableMapObject for SpellObject {
    fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas, _draw_location: Point) -> GameResult {
        // TODO: Implement spell drawing
        // C# Reference: Client/MirObjects/SpellObject.cs Draw() method
        // Need to:
        // 1. Get spell effect texture from library
        // 2. Apply animation frame
        // 3. Draw at current position
        Ok(())
    }
    
    fn object_id(&self) -> u32 {
        self.map_object.object_id()
    }
    
    fn is_dead(&self) -> bool {
        self.expired
    }
    
    fn is_hidden(&self) -> bool {
        self.map_object.is_hidden()
    }
    
    fn draw_priority(&self) -> i32 {
        1 // Spells draw after items but before creatures
    }
}