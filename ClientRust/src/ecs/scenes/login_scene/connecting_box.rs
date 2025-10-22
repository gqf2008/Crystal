// ConnectingBox - "Connecting to server..." dialog with Cancel button
// Mirrors C# _connectBox in LoginScene.cs

/// Connecting dialog shown while connecting to server
#[derive(Debug)]
pub struct ConnectingBox {
    pub visible: bool,
    pub message: String,
    pub attempt_count: u32,
    
    // Button state
    pub cancel_button_hovered: bool,
}

impl ConnectingBox {
    /// Create new connecting box
    pub fn new() -> Self {
        Self {
            visible: false,
            message: "Attempting to connect to the server.".to_string(),
            attempt_count: 0,
            cancel_button_hovered: false,
        }
    }
    
    /// Show the connecting box
    pub fn show(&mut self) {
        self.visible = true;
        self.attempt_count = 0;
    }
    
    /// Hide the connecting box
    pub fn hide(&mut self) {
        self.visible = false;
    }
    
    /// Update connection attempt message
    pub fn update_message(&mut self, attempt: u32) {
        self.attempt_count = attempt;
        self.message = format!("Attempting to connect to the server.\n\nAttempt: {}", attempt);
    }
    
    /// Check if Cancel button is at position
    pub fn is_cancel_button_at(&self, x: f32, y: f32, dialog_x: f32, dialog_y: f32) -> bool {
        if !self.visible {
            return false;
        }
        
        // Cancel button position (center bottom)
        let button_x = dialog_x + 70.0;  // Centered in 250px wide dialog
        let button_y = dialog_y + 120.0; // Near bottom
        let button_w = 110.0;
        let button_h = 25.0;
        
        x >= button_x && x < button_x + button_w &&
        y >= button_y && y < button_y + button_h
    }
    
    /// Update button hover state
    pub fn update_button_hover(&mut self, x: f32, y: f32, dialog_x: f32, dialog_y: f32) {
        self.cancel_button_hovered = self.is_cancel_button_at(x, y, dialog_x, dialog_y);
    }
    
    /// Handle click - returns true if Cancel button was clicked
    pub fn handle_click(&mut self, x: f32, y: f32, dialog_x: f32, dialog_y: f32) -> bool {
        if !self.visible {
            return false;
        }
        
        if self.is_cancel_button_at(x, y, dialog_x, dialog_y) {
            return true; // Cancel clicked
        }
        
        false
    }
}

impl Default for ConnectingBox {
    fn default() -> Self {
        Self::new()
    }
}
