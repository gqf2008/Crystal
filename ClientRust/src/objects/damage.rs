// Damage.rs - Floating damage numbers
// Mirrors Client/MirObjects/Damage.cs

use mir2_shared::Point;

/// Damage type for color coding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageType {
    Physical,     // White/Yellow
    Magic,        // Blue/Cyan
    Poison,       // Green
    Critical,     // Red (critical hit)
    Miss,         // Gray (miss)
    Block,        // Orange (blocked)
    Heal,         // Green (healing)
    Mana,         // Blue (mana)
}

/// RGB Color
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub fn with_alpha(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

/// Damage display - floating damage numbers above characters
#[derive(Debug, Clone)]
pub struct Damage {
    // Damage info
    pub amount: i32,
    pub damage_type: DamageType,
    pub text: String,
    
    // Position
    pub location: Point,     // World location
    pub offset: Point,       // Pixel offset (for floating animation)
    
    // Timing
    pub start_time: i64,
    pub duration: i64,       // How long to display (ms)
    pub fade_start: i64,     // When to start fading out
    
    // Animation
    pub rise_speed: f32,     // Pixels per second
    pub current_alpha: u8,   // Current alpha value (0-255)
    
    // State
    pub completed: bool,
}

impl Damage {
    /// Create a new damage display
    pub fn new(amount: i32, damage_type: DamageType, location: Point) -> Self {
        let current_time = get_current_time();
        let text = match damage_type {
            DamageType::Miss => "Miss".to_string(),
            DamageType::Block => "Block".to_string(),
            _ => amount.to_string(),
        };
        
        Self {
            amount,
            damage_type,
            text,
            location,
            offset: Point::new(0, 0),
            start_time: current_time,
            duration: 1500,      // 1.5 seconds
            fade_start: current_time + 1000, // Start fading after 1 second
            rise_speed: 30.0,    // 30 pixels per second
            current_alpha: 255,
            completed: false,
        }
    }

    /// Create physical damage display
    pub fn physical(amount: i32, location: Point, critical: bool) -> Self {
        let damage_type = if critical {
            DamageType::Critical
        } else {
            DamageType::Physical
        };
        Self::new(amount, damage_type, location)
    }

    /// Create magic damage display
    pub fn magic(amount: i32, location: Point) -> Self {
        Self::new(amount, DamageType::Magic, location)
    }

    /// Create poison damage display
    pub fn poison(amount: i32, location: Point) -> Self {
        Self::new(amount, DamageType::Poison, location)
    }

    /// Create healing display
    pub fn heal(amount: i32, location: Point) -> Self {
        Self::new(amount, DamageType::Heal, location)
    }

    /// Create mana restore display
    pub fn mana(amount: i32, location: Point) -> Self {
        Self::new(amount, DamageType::Mana, location)
    }

    /// Create miss display
    pub fn miss(location: Point) -> Self {
        Self::new(0, DamageType::Miss, location)
    }

    /// Create block display
    pub fn block(location: Point) -> Self {
        Self::new(0, DamageType::Block, location)
    }

    /// Update damage display (call every frame)
    pub fn update(&mut self, current_time: i64, delta_time: f32) -> bool {
        if self.completed {
            return false;
        }

        // Check if expired
        if current_time - self.start_time >= self.duration {
            self.completed = true;
            return false;
        }

        // Update position (float upward)
        let rise_delta = (self.rise_speed * delta_time) as i32;
        self.offset.y -= rise_delta;

        // Update alpha (fade out)
        if current_time >= self.fade_start {
            let elapsed = (current_time - self.fade_start) as f32;
            let fade_duration = (self.duration - 1000) as f32; // Duration after fade starts
            let fade_progress = (elapsed / fade_duration).min(1.0);
            self.current_alpha = (255.0 * (1.0 - fade_progress)) as u8;
        }

        true // Still active
    }

    /// Check if damage display is finished
    pub fn is_finished(&self) -> bool {
        self.completed
    }

    /// Get screen position (in pixels)
    pub fn get_screen_position(&self) -> Point {
        // Convert tile position to pixel position and apply offset
        // Assuming 48x32 tile size
        Point::new(
            self.location.x * 48 + self.offset.x,
            self.location.y * 32 + self.offset.y - 40, // Start 40 pixels above character
        )
    }

    /// Get color based on damage type
    pub fn get_color(&self) -> Color {
        let alpha = self.current_alpha;
        match self.damage_type {
            DamageType::Physical => Color::with_alpha(255, 255, 255, alpha),    // White
            DamageType::Magic => Color::with_alpha(100, 200, 255, alpha),       // Cyan
            DamageType::Poison => Color::with_alpha(100, 255, 100, alpha),      // Green
            DamageType::Critical => Color::with_alpha(255, 50, 50, alpha),      // Red
            DamageType::Miss => Color::with_alpha(150, 150, 150, alpha),        // Gray
            DamageType::Block => Color::with_alpha(255, 150, 50, alpha),        // Orange
            DamageType::Heal => Color::with_alpha(50, 255, 50, alpha),          // Bright Green
            DamageType::Mana => Color::with_alpha(50, 100, 255, alpha),         // Blue
        }
    }

    /// Get font size multiplier (critical hits are larger)
    pub fn get_font_scale(&self) -> f32 {
        match self.damage_type {
            DamageType::Critical => 1.5, // 50% larger
            _ => 1.0,
        }
    }

    /// Get text to display
    pub fn get_display_text(&self) -> &str {
        &self.text
    }

    /// Get outline color (for text readability)
    pub fn get_outline_color(&self) -> Color {
        Color::with_alpha(0, 0, 0, self.current_alpha) // Black outline
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
    fn test_damage_creation() {
        let damage = Damage::physical(100, Point::new(10, 10), false);
        assert_eq!(damage.amount, 100);
        assert_eq!(damage.damage_type, DamageType::Physical);
        assert_eq!(damage.text, "100");
        assert!(!damage.completed);
    }

    #[test]
    fn test_damage_types() {
        let physical = Damage::physical(50, Point::new(0, 0), false);
        assert_eq!(physical.damage_type, DamageType::Physical);
        
        let critical = Damage::physical(100, Point::new(0, 0), true);
        assert_eq!(critical.damage_type, DamageType::Critical);
        assert_eq!(critical.get_font_scale(), 1.5);
        
        let magic = Damage::magic(80, Point::new(0, 0));
        assert_eq!(magic.damage_type, DamageType::Magic);
        
        let heal = Damage::heal(30, Point::new(0, 0));
        assert_eq!(heal.damage_type, DamageType::Heal);
        
        let miss = Damage::miss(Point::new(0, 0));
        assert_eq!(miss.text, "Miss");
    }

    #[test]
    fn test_damage_update() {
        let mut damage = Damage::physical(100, Point::new(10, 10), false);
        let start_time = damage.start_time;
        
        // Initial position
        assert_eq!(damage.offset.y, 0);
        assert_eq!(damage.current_alpha, 255);
        
        // Update at 0.5s (delta_time = 0.016 for 60fps)
        for _ in 0..30 {
            damage.update(start_time + 500, 0.016);
        }
        
        // Should have floated upward
        assert!(damage.offset.y < 0);
        
        // Update at 1.5s (should be expired)
        assert!(!damage.update(start_time + 1500, 0.016));
        assert!(damage.is_finished());
    }

    #[test]
    fn test_damage_fade() {
        let mut damage = Damage::physical(100, Point::new(10, 10), false);
        damage.fade_start = damage.start_time;
        damage.duration = 1000;
        
        let start_time = damage.start_time;
        
        // At start, full opacity
        damage.update(start_time, 0.016);
        assert_eq!(damage.current_alpha, 255);
        
        // At halfway through fade, 50% opacity
        damage.update(start_time + 500, 0.016);
        assert!(damage.current_alpha < 255);
        assert!(damage.current_alpha > 0);
    }

    #[test]
    fn test_damage_colors() {
        let physical = Damage::physical(100, Point::new(0, 0), false);
        let color = physical.get_color();
        assert_eq!(color.r, 255); // White
        
        let critical = Damage::physical(100, Point::new(0, 0), true);
        let color = critical.get_color();
        assert_eq!(color.r, 255); // Red
        assert!(color.g < 100);
        
        let heal = Damage::heal(50, Point::new(0, 0));
        let color = heal.get_color();
        assert_eq!(color.g, 255); // Green
    }
}
