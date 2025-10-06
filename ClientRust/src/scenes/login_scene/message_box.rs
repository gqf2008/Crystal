// MessageBox - Simple message display dialog
// Mirrors Client/MirControls/MirMessageBox.cs

/// Simple message box for displaying text messages
#[derive(Debug)]
pub struct MessageBox {
    pub visible: bool,
    pub message: String,
    pub title: String,
    
    // Button state
    pub ok_button_hovered: bool,
    
    // Auto-close timer (optional)
    pub auto_close_time: Option<f32>,
    pub timer: f32,
}

impl MessageBox {
    /// Create new message box
    pub fn new(message: String) -> Self {
        Self {
            visible: false,
            message,
            title: "Message".to_string(),
            ok_button_hovered: false,
            auto_close_time: None,
            timer: 0.0,
        }
    }
    
    /// Create message box with custom title
    pub fn with_title(message: String, title: String) -> Self {
        Self {
            visible: false,
            message,
            title,
            ok_button_hovered: false,
            auto_close_time: None,
            timer: 0.0,
        }
    }
    
    /// Show the message box
    pub fn show(&mut self) {
        self.visible = true;
        self.timer = 0.0;
    }
    
    /// Hide the message box
    pub fn hide(&mut self) {
        self.visible = false;
    }
    
    /// Update (for auto-close)
    pub fn update(&mut self, delta_time: f32) -> bool {
        if !self.visible {
            return false;
        }
        
        if let Some(auto_close) = self.auto_close_time {
            self.timer += delta_time;
            if self.timer >= auto_close {
                self.hide();
                return true; // Closed
            }
        }
        
        false
    }
    
    /// Check if OK button is hovered at position
    pub fn is_ok_button_at(&self, x: f32, y: f32, dialog_x: f32, dialog_y: f32) -> bool {
        if !self.visible {
            return false;
        }
        
        // OK button position (center bottom)
        let button_x = dialog_x + 95.0;  // Centered in 250px wide dialog
        let button_y = dialog_y + 140.0; // Near bottom
        let button_w = 60.0;
        let button_h = 20.0;
        
        x >= button_x && x < button_x + button_w &&
        y >= button_y && y < button_y + button_h
    }
    
    /// Update button hover state
    pub fn update_button_hover(&mut self, x: f32, y: f32, dialog_x: f32, dialog_y: f32) {
        self.ok_button_hovered = self.is_ok_button_at(x, y, dialog_x, dialog_y);
    }
    
    /// Handle click - returns true if OK button was clicked
    pub fn handle_click(&mut self, x: f32, y: f32, dialog_x: f32, dialog_y: f32) -> bool {
        if !self.visible {
            return false;
        }
        
        if self.is_ok_button_at(x, y, dialog_x, dialog_y) {
            self.hide();
            return true;
        }
        
        false
    }
}

impl Default for MessageBox {
    fn default() -> Self {
        Self::new(String::new())
    }
}
