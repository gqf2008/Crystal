// SpellObject.rs - Flying spell projectiles
// Mirrors Client/MirObjects/SpellObject.cs

use mir2_shared::{
    enums::{MirDirection, Spell, SpellEffect},
    Point,
};

use super::map_object::MapObject;
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
            map_object: MapObject::new_spell(object_id),
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

    /// Calculate velocity vector from start to target
    fn calculate_velocity(&mut self) {
        let dx = self.target_location.x - self.start_location.x;
        let dy = self.target_location.y - self.start_location.y;
        let distance = ((dx * dx + dy * dy) as f32).sqrt();
        
        if distance > 0.0 {
            self.velocity.x = (dx as f32 / distance * self.speed as f32) as i32;
            self.velocity.y = (dy as f32 / distance * self.speed as f32) as i32;
        }
    }

    /// Configure spell-specific parameters
    fn configure_spell(&mut self) {
        match self.spell {
            Spell::FireBall => {
                self.speed = 10;
                self.frame_count = 10;
                self.frame_interval = 50;
                self.repeat = true;
            }
            Spell::GreatFireBall => {
                self.speed = 8;
                self.frame_count = 20;
                self.frame_interval = 60;
                self.repeat = true;
            }
            Spell::ThunderBolt => {
                self.speed = 20;
                self.frame_count = 5;
                self.frame_interval = 30;
                self.repeat = false;
            }
            Spell::Lightning => {
                self.speed = 25;
                self.frame_count = 10;
                self.frame_interval = 20;
                self.repeat = false;
            }
            Spell::IceThrust => {
                self.speed = 15;
                self.frame_count = 8;
                self.frame_interval = 40;
                self.repeat = true;
            }
            _ => {
                self.speed = 10;
                self.frame_count = 10;
                self.frame_interval = 50;
                self.repeat = false;
            }
        }
    }

    /// Update spell position
    pub fn update_position(&mut self, current_time: i64) {
        if self.expired {
            return;
        }

        // Move towards target
        self.map_object.current_location.x += self.velocity.x;
        self.map_object.current_location.y += self.velocity.y;

        // Check if reached target
        if self.has_reached_target() {
            self.on_hit(current_time);
        }

        // Update animation frame
        self.update_frame(current_time);
    }

    /// Check if spell has reached target location
    fn has_reached_target(&self) -> bool {
        let dx = (self.map_object.current_location.x - self.target_location.x).abs();
        let dy = (self.map_object.current_location.y - self.target_location.y).abs();
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
        Point::new(
            self.map_object.current_location.x * 48,
            self.map_object.current_location.y * 32,
        )
    }

    /// Check collision with target object
    pub fn check_collision(&self, target_location: Point) -> bool {
        if self.expired {
            return false;
        }
        
        let dx = (self.map_object.current_location.x - target_location.x).abs();
        let dy = (self.map_object.current_location.y - target_location.y).abs();
        
        // Within 1 tile range
        dx <= 1 && dy <= 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spell_object_creation() {
        let spell = SpellObject::new(1, Spell::FireBall);
        assert_eq!(spell.map_object.object_id, 1);
        assert_eq!(spell.spell, Spell::FireBall);
        assert!(!spell.expired);
    }

    #[test]
    fn test_spell_velocity_calculation() {
        let mut spell = SpellObject::new(1, Spell::FireBall);
        spell.start_location = Point::new(0, 0);
        spell.target_location = Point::new(10, 10);
        spell.speed = 10;
        
        spell.calculate_velocity();
        
        // Velocity should point towards target (normalized and scaled)
        assert!(spell.velocity.x > 0);
        assert!(spell.velocity.y > 0);
    }

    #[test]
    fn test_spell_configuration() {
        let mut spell = SpellObject::new(1, Spell::FireBall);
        spell.configure_spell();
        assert_eq!(spell.speed, 10);
        assert!(spell.repeat);
        
        let mut lightning = SpellObject::new(2, Spell::Lightning);
        lightning.configure_spell();
        assert_eq!(lightning.speed, 25);
        assert!(!lightning.repeat);
    }

    #[test]
    fn test_spell_expiration() {
        let mut spell = SpellObject::new(1, Spell::FireBall);
        let current_time = 1000;
        
        assert!(!spell.should_remove(current_time));
        
        spell.expired = true;
        spell.hit_time = current_time;
        
        assert!(!spell.should_remove(current_time));
        assert!(spell.should_remove(current_time + 1500)); // After 1s
    }
}
