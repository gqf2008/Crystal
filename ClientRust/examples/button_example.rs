// Example: Button state management with resources
// Demonstrates how to use embedded resources for UI buttons

use mir2_client::resources::Images;
use image::DynamicImage;

/// Represents the visual states of a button
pub struct ButtonTextures {
    pub base: DynamicImage,
    pub hover: DynamicImage,
    pub pressed: DynamicImage,
}

impl ButtonTextures {
    /// Create textures for the launch button
    pub fn launch_button() -> Result<Self, image::ImageError> {
        Ok(Self {
            base: Images::load_launch_base()?,
            hover: Images::load_launch_hover()?,
            pressed: Images::load_launch_pressed()?,
        })
    }
    
    /// Create textures for the config button
    pub fn config_button() -> Result<Self, image::ImageError> {
        Ok(Self {
            base: Images::load_config_base()?,
            hover: Images::load_config_hover()?,
            pressed: Images::load_config_pressed()?,
        })
    }
    
    /// Create textures for the close button
    pub fn close_button() -> Result<Self, image::ImageError> {
        Ok(Self {
            base: Images::load_cross_base()?,
            hover: Images::load_cross_hover()?,
            pressed: Images::load_cross_pressed()?,
        })
    }
    
    /// Create textures for checkbox
    pub fn checkbox() -> Result<Self, image::ImageError> {
        Ok(Self {
            base: Images::load_checkf_base2()?,
            hover: Images::load_checkf_hover()?,
            pressed: Images::load_checkf_pressed()?,
        })
    }
}

/// Button state enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    Normal,
    Hover,
    Pressed,
}

/// A UI button with state management
pub struct Button {
    textures: ButtonTextures,
    state: ButtonState,
    x: i32,
    y: i32,
}

impl Button {
    pub fn new(textures: ButtonTextures, x: i32, y: i32) -> Self {
        Self {
            textures,
            state: ButtonState::Normal,
            x,
            y,
        }
    }
    
    /// Get the current texture based on state
    pub fn current_texture(&self) -> &DynamicImage {
        match self.state {
            ButtonState::Normal => &self.textures.base,
            ButtonState::Hover => &self.textures.hover,
            ButtonState::Pressed => &self.textures.pressed,
        }
    }
    
    /// Update button state (e.g., on mouse events)
    pub fn set_state(&mut self, state: ButtonState) {
        self.state = state;
    }
    
    /// Check if point is inside button bounds
    pub fn contains_point(&self, px: i32, py: i32) -> bool {
        use image::GenericImageView;
        let (width, height) = self.textures.base.dimensions();
        px >= self.x && px < self.x + width as i32 &&
        py >= self.y && py < self.y + height as i32
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Button State Management Example ===\n");
    
    // Create buttons
    println!("Creating buttons...");
    let launch_textures = ButtonTextures::launch_button()?;
    let config_textures = ButtonTextures::config_button()?;
    let close_textures = ButtonTextures::close_button()?;
    
    let mut launch_btn = Button::new(launch_textures, 100, 200);
    let mut config_btn = Button::new(config_textures, 250, 200);
    let mut close_btn = Button::new(close_textures, 400, 200);
    
    println!("✅ Launch button created at (100, 200)");
    println!("✅ Config button created at (250, 200)");
    println!("✅ Close button created at (400, 200)");
    
    // Simulate state changes
    println!("\n=== Simulating Mouse Events ===\n");
    
    println!("1. Mouse enters launch button:");
    launch_btn.set_state(ButtonState::Hover);
    print_button_info("Launch", &launch_btn);
    
    println!("\n2. Mouse clicks launch button:");
    launch_btn.set_state(ButtonState::Pressed);
    print_button_info("Launch", &launch_btn);
    
    println!("\n3. Mouse releases, button returns to normal:");
    launch_btn.set_state(ButtonState::Normal);
    print_button_info("Launch", &launch_btn);
    
    println!("\n4. Mouse enters config button:");
    config_btn.set_state(ButtonState::Hover);
    print_button_info("Config", &config_btn);
    
    // Test hit detection
    println!("\n=== Hit Detection Test ===\n");
    let test_points = [
        (110, 210, "inside launch button"),
        (260, 210, "inside config button"),
        (50, 50, "outside all buttons"),
    ];
    
    for (x, y, desc) in test_points {
        println!("Point ({}, {}) - {}:", x, y, desc);
        println!("  Launch: {}", if launch_btn.contains_point(x, y) { "HIT" } else { "miss" });
        println!("  Config: {}", if config_btn.contains_point(x, y) { "HIT" } else { "miss" });
        println!("  Close:  {}", if close_btn.contains_point(x, y) { "HIT" } else { "miss" });
    }
    
    println!("\n✅ Example completed successfully!");
    
    Ok(())
}

fn print_button_info(name: &str, button: &Button) {
    use image::GenericImageView;
    let texture = button.current_texture();
    let (w, h) = texture.dimensions();
    println!("  {} button state: {:?}", name, button.state);
    println!("  Current texture: {}x{} pixels", w, h);
}
