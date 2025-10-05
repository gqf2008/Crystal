// MirControl - Base control class
// Mirrors Client/MirControls/MirControl.cs

use super::types::*;
use std::any::Any;

/// Control trait - Base interface for all UI controls
/// 
/// This trait mirrors the C# MirControl class and provides the foundation
/// for all UI controls in the game.
/// 
/// # Design Notes
/// - Uses trait objects for polymorphism (similar to C# inheritance)
/// - Children are stored as Box<dyn Control> for dynamic dispatch
/// - Events are handled through callback methods (on_mouse_down, etc.)
pub trait Control {
    // === Identity ===
    
    /// Get control name (for debugging)
    fn name(&self) -> &str {
        "Control"
    }
    
    // === Position and Size ===
    
    /// Get location relative to parent
    fn location(&self) -> Point;
    
    /// Set location relative to parent
    fn set_location(&mut self, location: Point);
    
    /// Get control size
    fn size(&self) -> Size;
    
    /// Set control size
    fn set_size(&mut self, size: Size);
    
    /// Get absolute display location
    /// Note: Without parent tracking, this returns local location.
    /// Dialogs/scenes should track absolute positions when needed.
    fn display_location(&self) -> Point {
        self.location()
    }
    
    /// Get display rectangle (absolute position + size)
    /// Mirrors C# `public Rectangle DisplayRectangle`
    fn display_rectangle(&self) -> Rectangle {
        Rectangle::from_location_size(self.display_location(), self.size())
    }
    
    // === Visibility and State ===
    
    /// Get visibility (local state)
    fn visible(&self) -> bool;
    
    /// Set visibility
    fn set_visible(&mut self, visible: bool);
    
    /// Get enabled state (local)
    fn enabled(&self) -> bool;
    
    /// Set enabled state
    fn set_enabled(&mut self, enabled: bool);
    
    /// Check if control is really visible
    /// Note: Without parent tracking, this returns local visibility.
    /// Dialogs/scenes should manage visibility hierarchies.
    fn is_really_visible(&self) -> bool {
        self.visible()
    }
    
    /// Check if control is really enabled
    /// Note: Without parent tracking, this returns local enabled state.
    /// Dialogs/scenes should manage enabled hierarchies.
    fn is_really_enabled(&self) -> bool {
        self.enabled()
    }
    
    // === Colors ===
    
    /// Get background color
    fn back_color(&self) -> Color;
    
    /// Set background color
    fn set_back_color(&mut self, color: Color);
    
    /// Get foreground color (for text)
    fn fore_color(&self) -> Color;
    
    /// Set foreground color
    fn set_fore_color(&mut self, color: Color);
    
    /// Get border color
    fn border_color(&self) -> Color;
    
    /// Set border color
    fn set_border_color(&mut self, color: Color);
    
    // === Border ===
    
    /// Get border visibility
    fn border(&self) -> bool;
    
    /// Set border visibility
    fn set_border(&mut self, border: bool);
    
    // === Visual Effects ===
    
    /// Get grayscale effect state
    fn gray_scale(&self) -> bool;
    
    /// Set grayscale effect
    fn set_gray_scale(&mut self, gray_scale: bool);
    
    /// Get blending state
    fn blending(&self) -> bool;
    
    /// Set blending
    fn set_blending(&mut self, blending: bool);
    
    /// Get blending rate (0.0 - 1.0)
    fn blending_rate(&self) -> f32;
    
    /// Set blending rate
    fn set_blending_rate(&mut self, rate: f32);
    
    /// Get blend mode
    fn blend_mode(&self) -> BlendMode;
    
    /// Set blend mode
    fn set_blend_mode(&mut self, mode: BlendMode);
    
    // === Hierarchy ===
    // Note: Child management methods are removed from trait to maintain object safety.
    // Concrete types should implement their own child management if needed.
    
    // === Lifecycle ===
    
    /// Initialize control (called once when first shown)
    fn initialize(&mut self) {}
    
    /// Update control logic (called every frame)
    /// Note: Implementations should update their children
    fn update(&mut self, _delta_time: f32) {}
    
    /// Draw control and children
    /// Mirrors C# Draw() method
    fn draw(&self) {
        if !self.visible() {
            return;
        }
        
        self.on_before_draw();
        self.draw_control();
        self.draw_children();
        self.on_after_draw();
    }
    
    /// Draw control itself (without children)
    /// Override this in derived controls
    fn draw_control(&self);
    
    /// Draw all children
    /// Note: Implementations should draw their children
    fn draw_children(&self) {}
    
    // === Event Handlers ===
    
    /// Handle mouse move event
    /// Note: Implementations should propagate to children
    fn on_mouse_move(&mut self, _x: i32, _y: i32) {}
    
    /// Handle mouse button down event
    /// Note: Implementations should propagate to children
    fn on_mouse_down(&mut self, _x: i32, _y: i32, _button: MouseButton) {}
    
    /// Handle mouse button up event
    /// Note: Implementations should propagate to children
    fn on_mouse_up(&mut self, _x: i32, _y: i32, _button: MouseButton) {}
    
    /// Handle mouse click event
    fn on_click(&mut self, _x: i32, _y: i32, _button: MouseButton) {}
    
    /// Handle mouse double-click event
    fn on_double_click(&mut self, _x: i32, _y: i32, _button: MouseButton) {}
    
    /// Handle mouse wheel event
    fn on_mouse_wheel(&mut self, _delta: i32) {}
    
    /// Handle key down event
    fn on_key_down(&mut self, _key: KeyCode) {}
    
    /// Handle key up event
    fn on_key_up(&mut self, _key: KeyCode) {}
    
    /// Handle character input event
    fn on_key_press(&mut self, _ch: char) {}
    
    // === Event Callbacks (Virtual Methods) ===
    
    /// Called before drawing
    fn on_before_draw(&self) {}
    
    /// Called after drawing
    fn on_after_draw(&self) {}
    
    /// Called when control is first shown
    fn on_shown(&mut self) {}
    
    /// Called when location changes
    fn on_location_changed(&mut self) {
        self.invalidate();
    }
    
    /// Called when size changes
    fn on_size_changed(&mut self) {
        self.invalidate();
    }
    
    /// Called when enabled state changes
    fn on_enabled_changed(&mut self) {
        self.invalidate();
    }
    
    /// Called when visibility changes
    fn on_visible_changed(&mut self) {
        self.invalidate();
    }
    
    /// Called when a child is added
    fn on_child_added(&mut self) {}
    
    /// Called when a child is removed
    fn on_child_removed(&mut self) {}
    
    /// Called when back color changes
    fn on_back_color_changed(&mut self) {
        self.invalidate();
    }
    
    /// Called when fore color changes
    fn on_fore_color_changed(&mut self) {
        self.invalidate();
    }
    
    // === Utility Methods ===
    
    /// Mark control as needing redraw
    /// Mirrors C# Redraw() method
    fn invalidate(&mut self);
    
    /// Force immediate redraw
    fn redraw(&mut self) {
        self.invalidate();
    }
    
    /// Dispose of control resources
    /// Note: Implementations should dispose their children
    fn dispose(&mut self) {}
    
    /// Type casting helper (downcast)
    fn as_any(&self) -> &dyn Any;
    
    /// Type casting helper (downcast mutable)
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// MirControl - Default implementation of Control trait
/// 
/// This is the base control class that all other controls inherit from.
/// Mirrors C# `public class MirControl : IDisposable`
pub struct MirControl {
    // Basic properties
    location: Point,
    size: Size,
    visible: bool,
    enabled: bool,
    
    // Colors
    back_color: Color,
    fore_color: Color,
    border_color: Color,
    
    // Border
    border: bool,
    
    // Visual effects
    gray_scale: bool,
    blending: bool,
    blending_rate: f32,
    blend_mode: BlendMode,
    
    // Hierarchy
    children: Vec<MirControl>,
    
    // Rendering state
    texture_valid: bool,
    needs_redraw: bool,
    
    // Text and tooltip
    hint: String,
    
    // Control texture state (for caching)
    draw_control_texture: bool,
}

impl MirControl {
    /// Create a new MirControl with default values
    pub fn new() -> Self {
        Self {
            location: Point::new(0, 0),
            size: Size::zero(),
            visible: true,
            enabled: true,
            back_color: Color::transparent(),
            fore_color: Color::white(),
            border_color: Color::black(),
            border: false,
            gray_scale: false,
            blending: false,
            blending_rate: 1.0,
            blend_mode: BlendMode::default(),
            children: Vec::new(),
            texture_valid: false,
            needs_redraw: true,
            hint: String::new(),
            draw_control_texture: false,
        }
    }
    
    /// Builder: Set location
    pub fn with_location(mut self, location: Point) -> Self {
        self.location = location;
        self
    }
    
    /// Builder: Set size
    pub fn with_size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }
    
    /// Builder: Set back color
    pub fn with_back_color(mut self, color: Color) -> Self {
        self.back_color = color;
        self
    }
    
    /// Builder: Set fore color
    pub fn with_fore_color(mut self, color: Color) -> Self {
        self.fore_color = color;
        self
    }
    
    /// Builder: Set visible
    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }
    
    /// Builder: Set enabled
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    
    /// Builder: Set border
    pub fn with_border(mut self, border: bool) -> Self {
        self.border = border;
        self
    }
    
    /// Get hint text
    pub fn hint(&self) -> &str {
        &self.hint
    }
    
    /// Set hint text
    pub fn set_hint(&mut self, hint: String) {
        self.hint = hint;
    }
    
    /// Get draw control texture flag
    pub fn draw_control_texture(&self) -> bool {
        self.draw_control_texture
    }
    
    /// Set draw control texture flag
    pub fn set_draw_control_texture(&mut self, draw: bool) {
        self.draw_control_texture = draw;
        self.invalidate();
    }
    
    // === Child Management ===
    
    /// Get children (immutable)
    pub fn children(&self) -> &[MirControl] {
        &self.children
    }
    
    /// Get children (mutable)
    pub fn children_mut(&mut self) -> &mut Vec<MirControl> {
        &mut self.children
    }
    
    /// Add child control
    pub fn add_child(&mut self, child: MirControl) {
        self.children.push(child);
        self.needs_redraw = true;
    }
    
    /// Remove child control by index
    pub fn remove_child(&mut self, index: usize) -> Option<MirControl> {
        if index < self.children.len() {
            let child = self.children.remove(index);
            self.needs_redraw = true;
            Some(child)
        } else {
            None
        }
    }
    
    /// Find child by predicate
    pub fn find_child<F>(&self, predicate: F) -> Option<&MirControl>
    where
        F: Fn(&MirControl) -> bool,
    {
        self.children.iter().find(|child| predicate(*child))
    }
}

impl Control for MirControl {
    fn name(&self) -> &str {
        "MirControl"
    }
    
    // === Position and Size ===
    
    fn location(&self) -> Point {
        self.location
    }
    
    fn set_location(&mut self, location: Point) {
        if self.location != location {
            self.location = location;
            self.on_location_changed();
        }
    }
    
    fn size(&self) -> Size {
        self.size
    }
    
    fn set_size(&mut self, size: Size) {
        if self.size != size {
            self.size = size;
            self.texture_valid = false;
            self.on_size_changed();
        }
    }
    
    // === Visibility and State ===
    
    fn visible(&self) -> bool {
        self.visible
    }
    
    fn set_visible(&mut self, visible: bool) {
        if self.visible != visible {
            self.visible = visible;
            self.on_visible_changed();
        }
    }
    
    fn enabled(&self) -> bool {
        self.enabled
    }
    
    fn set_enabled(&mut self, enabled: bool) {
        if self.enabled != enabled {
            self.enabled = enabled;
            self.on_enabled_changed();
        }
    }
    
    // === Colors ===
    
    fn back_color(&self) -> Color {
        self.back_color
    }
    
    fn set_back_color(&mut self, color: Color) {
        if self.back_color != color {
            self.back_color = color;
            self.texture_valid = false;
            self.on_back_color_changed();
        }
    }
    
    fn fore_color(&self) -> Color {
        self.fore_color
    }
    
    fn set_fore_color(&mut self, color: Color) {
        if self.fore_color != color {
            self.fore_color = color;
            self.on_fore_color_changed();
        }
    }
    
    fn border_color(&self) -> Color {
        self.border_color
    }
    
    fn set_border_color(&mut self, color: Color) {
        if self.border_color != color {
            self.border_color = color;
            self.redraw();
        }
    }
    
    // === Border ===
    
    fn border(&self) -> bool {
        self.border
    }
    
    fn set_border(&mut self, border: bool) {
        if self.border != border {
            self.border = border;
            self.redraw();
        }
    }
    
    // === Visual Effects ===
    
    fn gray_scale(&self) -> bool {
        self.gray_scale
    }
    
    fn set_gray_scale(&mut self, gray_scale: bool) {
        self.gray_scale = gray_scale;
        self.redraw();
    }
    
    fn blending(&self) -> bool {
        self.blending
    }
    
    fn set_blending(&mut self, blending: bool) {
        self.blending = blending;
        self.redraw();
    }
    
    fn blending_rate(&self) -> f32 {
        self.blending_rate
    }
    
    fn set_blending_rate(&mut self, rate: f32) {
        self.blending_rate = rate.clamp(0.0, 1.0);
        self.redraw();
    }
    
    fn blend_mode(&self) -> BlendMode {
        self.blend_mode
    }
    
    fn set_blend_mode(&mut self, mode: BlendMode) {
        self.blend_mode = mode;
        self.redraw();
    }
    
    // === Hierarchy ===
    // Child management is implemented directly on MirControl
    // (not through trait to maintain object safety)
    
    // === Drawing ===
    
    fn draw_control(&self) {
        // Base implementation: draw background and border
        
        // Draw background if not transparent
        if self.back_color.a() > 0 {
            // TODO: Call rendering system to draw filled rectangle
            // render::draw_filled_rect(self.display_rectangle(), self.back_color);
        }
        
        // Draw border if enabled
        if self.border {
            // TODO: Call rendering system to draw border
            // render::draw_rect(self.display_rectangle(), self.border_color);
        }
    }
    
    // === Utility ===
    
    fn invalidate(&mut self) {
        self.needs_redraw = true;
        self.texture_valid = false;
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Default for MirControl {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_creation() {
        let control = MirControl::new();
        assert_eq!(control.location(), Point::new(0, 0));
        assert_eq!(control.size(), Size::zero());
        assert!(control.visible());
        assert!(control.enabled());
    }

    #[test]
    fn test_control_builder() {
        let control = MirControl::new()
            .with_location(Point::new(10, 20))
            .with_size(Size::new(100, 50))
            .with_back_color(Color::red())
            .with_visible(false);
        
        assert_eq!(control.location(), Point::new(10, 20));
        assert_eq!(control.size(), Size::new(100, 50));
        assert_eq!(control.back_color(), Color::red());
        assert!(!control.visible());
    }

    #[test]
    fn test_control_properties() {
        let mut control = MirControl::new();
        
        control.set_location(Point::new(50, 100));
        assert_eq!(control.location(), Point::new(50, 100));
        
        control.set_size(Size::new(200, 150));
        assert_eq!(control.size(), Size::new(200, 150));
        
        control.set_visible(false);
        assert!(!control.visible());
        
        control.set_enabled(false);
        assert!(!control.enabled());
    }

    #[test]
    fn test_control_colors() {
        let mut control = MirControl::new();
        
        control.set_back_color(Color::blue());
        assert_eq!(control.back_color(), Color::blue());
        
        control.set_fore_color(Color::yellow());
        assert_eq!(control.fore_color(), Color::yellow());
        
        control.set_border_color(Color::green());
        assert_eq!(control.border_color(), Color::green());
    }

    #[test]
    fn test_display_rectangle() {
        let control = MirControl::new()
            .with_location(Point::new(10, 20))
            .with_size(Size::new(100, 50));
        
        let rect = control.display_rectangle();
        assert_eq!(rect.left(), 10);
        assert_eq!(rect.top(), 20);
        assert_eq!(rect.right(), 110);
        assert_eq!(rect.bottom(), 70);
    }

    #[test]
    fn test_children() {
        let mut parent = MirControl::new();
        let child1 = MirControl::new();
        let child2 = MirControl::new();
        
        parent.add_child(child1);
        parent.add_child(child2);
        
        assert_eq!(parent.children().len(), 2);
        
        parent.remove_child(0);
        assert_eq!(parent.children().len(), 1);
    }

    #[test]
    fn test_visual_effects() {
        let mut control = MirControl::new();
        
        control.set_gray_scale(true);
        assert!(control.gray_scale());
        
        control.set_blending(true);
        assert!(control.blending());
        
        control.set_blending_rate(0.5);
        assert_eq!(control.blending_rate(), 0.5);
        
        control.set_blend_mode(BlendMode::Additive);
        assert_eq!(control.blend_mode(), BlendMode::Additive);
    }
}
