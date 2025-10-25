// Effect.rs - Visual effects (explosions, auras, buffs)
// Mirrors Client/MirObjects/Effect.cs

use mir2_shared::{enums::SpellEffect, Point};

/// Effect layer for rendering order
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EffectLayer {
    BelowObject = 0,  // Ground effects (traps, magic circles)
    OnObject = 1,      // Effects on character (buffs, auras)
    AboveObject = 2,   // Effects above character (healing, level up)
    Front = 3,         // Front effects (explosions, hits)
}

/// Effect blend mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpriteBlendMode {
    None,
    Additive,      // Bright, glowing effects
    Alpha,         // Semi-transparent
    Multiply,      // Darkening effects
}

/// Effect object - represents visual effects (explosions, auras, buffs)
#[derive(Debug, Clone)]
pub struct Effect {
    // Visual properties
    pub effect_type: SpellEffect,
    pub library_index: u32,    // Which library to load from
    pub start_frame: u32,      // Starting frame index
    pub frame_count: u32,      // Number of frames
    pub frame_interval: i32,   // Milliseconds between frames
    pub current_frame: u32,    // Current frame being displayed
    
    // Position
    pub location: Point,
    pub offset: Point,         // Pixel offset from location
    
    // Timing
    pub start_time: i64,
    pub last_frame_time: i64,
    pub duration: i64,         // Total duration in ms (0 = until animation ends)
    
    // Flags
    pub repeat: bool,          // Loop animation
    pub repeat_until: i64,     // Repeat until this time (0 = infinite)
    pub blend: bool,           // Use blend mode
    pub blend_mode: SpriteBlendMode,
    pub light: i32,            // Light intensity (0-10)
    pub layer: EffectLayer,
    
    // State
    pub completed: bool,
    pub owner_id: u32,         // Object that owns this effect (for tracking)
}

impl Effect {
    /// Create a new effect
    pub fn new(
        effect_type: SpellEffect,
        location: Point,
        start_frame: u32,
        frame_count: u32,
        frame_interval: i32,
    ) -> Self {
        let current_time = get_current_time();
        Self {
            effect_type,
            library_index: 0,
            start_frame,
            frame_count,
            frame_interval,
            current_frame: 0,
            location,
            offset: Point::new(0, 0),
            start_time: current_time,
            last_frame_time: current_time,
            duration: 0,
            repeat: false,
            repeat_until: 0,
            blend: false,
            blend_mode: SpriteBlendMode::None,
            light: 0,
            layer: EffectLayer::OnObject,
            completed: false,
            owner_id: 0,
        }
    }

    /// Create an explosion effect
    pub fn explosion(location: Point, size: u32) -> Self {
        let mut effect = Self::new(
            SpellEffect::DelayedExplosion,
            location,
            100, // Start frame
            10,  // Frame count
            50,  // 50ms per frame
        );
        effect.blend = true;
        effect.blend_mode = SpriteBlendMode::Additive;
        effect.light = 5;
        effect.layer = EffectLayer::Front;
        
        // Scale based on size
        effect.frame_count = size * 5;
        effect
    }

    /// Create a buff aura effect
    pub fn buff_aura(location: Point, buff_type: SpellEffect) -> Self {
        let mut effect = Self::new(
            buff_type,
            location,
            200, // Start frame
            20,  // Frame count
            100, // 100ms per frame
        );
        effect.repeat = true;
        effect.blend = true;
        effect.blend_mode = SpriteBlendMode::Alpha;
        effect.light = 2;
        effect.layer = EffectLayer::OnObject;
        effect
    }

    /// Create a healing effect
    pub fn healing(location: Point) -> Self {
        let mut effect = Self::new(
            SpellEffect::Healing,
            location,
            300, // Start frame
            10,  // Frame count
            80,  // 80ms per frame
        );
        effect.blend = true;
        effect.blend_mode = SpriteBlendMode::Additive;
        effect.light = 3;
        effect.layer = EffectLayer::AboveObject;
        effect.offset = Point::new(0, -20); // Float above character
        effect
    }

    /// Update effect (call every frame)
    pub fn update(&mut self, current_time: i64) -> bool {
        if self.completed {
            return false;
        }

        // Check if effect should expire by duration
        if self.duration > 0 && current_time - self.start_time >= self.duration {
            self.completed = true;
            return false;
        }

        // Check if effect should expire by repeat_until
        if self.repeat_until > 0 && current_time >= self.repeat_until {
            self.completed = true;
            return false;
        }

        // Update frame
        if current_time - self.last_frame_time >= self.frame_interval as i64 {
            self.last_frame_time = current_time;
            self.current_frame += 1;

            if self.current_frame >= self.frame_count {
                if self.repeat && (self.repeat_until == 0 || current_time < self.repeat_until) {
                    self.current_frame = 0; // Loop
                } else {
                    self.completed = true;
                    return false;
                }
            }
        }

        true // Still active
    }

    /// Check if effect is finished
    pub fn is_finished(&self) -> bool {
        self.completed
    }

    /// Get current frame index for rendering
    pub fn get_current_frame_index(&self) -> u32 {
        self.start_frame + self.current_frame
    }

    /// Get screen position (in pixels)
    pub fn get_screen_position(&self) -> Point {
        // Convert tile position to pixel position and apply offset
        // Assuming 48x32 tile size
        Point::new(
            self.location.x * 48 + self.offset.x,
            self.location.y * 32 + self.offset.y,
        )
    }

    /// Get draw priority (for sorting)
    pub fn get_draw_priority(&self) -> i32 {
        // Priority = layer * 10000 + y_position
        (self.layer as i32) * 10000 + self.location.y
    }

    /// Set to repeat until a specific time
    pub fn repeat_until_time(&mut self, until_time: i64) {
        self.repeat = true;
        self.repeat_until = until_time;
    }

    /// Set duration
    pub fn with_duration(mut self, duration: i64) -> Self {
        self.duration = duration;
        self
    }

    /// Set light intensity
    pub fn with_light(mut self, light: i32) -> Self {
        self.light = light;
        self
    }

    /// Set blend mode
    pub fn with_blend(mut self, blend_mode: SpriteBlendMode) -> Self {
        self.blend = true;
        self.blend_mode = blend_mode;
        self
    }

    /// Set layer
    pub fn with_layer(mut self, layer: EffectLayer) -> Self {
        self.layer = layer;
        self
    }
}

/// Get current time in milliseconds
fn get_current_time() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effect_creation() {
        let effect = Effect::new(
            SpellEffect::DelayedExplosion,
            Point::new(10, 10),
            100,
            10,
            50,
        );
        assert_eq!(effect.current_frame, 0);
        assert!(!effect.completed);
    }

    #[test]
    fn test_effect_update() {
        let mut effect = Effect::new(
            SpellEffect::DelayedExplosion,
            Point::new(10, 10),
            100,
            5,
            100, // 100ms per frame
        );
        
        let start_time = effect.start_time;
        
        // Update at 50ms - should not advance frame
        assert!(effect.update(start_time + 50));
        assert_eq!(effect.current_frame, 0);
        
        // Update at 100ms - should advance frame
        assert!(effect.update(start_time + 100));
        assert_eq!(effect.current_frame, 1);
        
        // Update until completion (5 frames * 100ms = 500ms)
        assert!(effect.update(start_time + 200));
        assert!(effect.update(start_time + 300));
        assert!(effect.update(start_time + 400));
        assert!(!effect.update(start_time + 500)); // Should complete
        
        assert!(effect.is_finished());
    }

    #[test]
    fn test_effect_repeat() {
        let mut effect = Effect::new(
            SpellEffect::None,
            Point::new(10, 10),
            100,
            3,
            100,
        );
        effect.repeat = true;
        
        let start_time = effect.start_time;
        
        // Go through all frames
        effect.update(start_time + 100);
        effect.update(start_time + 200);
        effect.update(start_time + 300);
        
        // Should loop back to frame 0
        assert_eq!(effect.current_frame, 0);
        assert!(!effect.is_finished());
    }

    #[test]
    fn test_effect_duration() {
        let mut effect = Effect::new(
            SpellEffect::None,
            Point::new(10, 10),
            100,
            10,
            100,
        );
        effect.duration = 500; // 500ms total
        effect.repeat = true;
        
        let start_time = effect.start_time;
        
        // Should expire after duration even though repeat is true
        assert!(effect.update(start_time + 100));
        assert!(effect.update(start_time + 200));
        assert!(!effect.update(start_time + 600)); // After duration
        
        assert!(effect.is_finished());
    }

    #[test]
    fn test_effect_helpers() {
        let explosion = Effect::explosion(Point::new(10, 10), 2);
        assert_eq!(explosion.effect_type, SpellEffect::DelayedExplosion);
        assert!(explosion.blend);
        assert_eq!(explosion.layer, EffectLayer::Front);
        
        let healing = Effect::healing(Point::new(5, 5));
        assert_eq!(healing.effect_type, SpellEffect::Healing);
        assert_eq!(healing.layer, EffectLayer::AboveObject);
        assert_eq!(healing.offset.y, -20);
    }
}

