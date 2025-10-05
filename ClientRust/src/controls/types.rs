// Shared types for MirControls
// Mirrors common types from C# (Point, Size, Rectangle, Color, etc.)

use mir2_shared::Color as SharedColor;

// Re-export Point from SharedRust
pub use mir2_shared::Point;

// Color wrapper to provide client-specific methods while using SharedRust's Color
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    inner: SharedColor,
}

impl Color {
    #[inline]
    pub fn from_argb(a: u8, r: u8, g: u8, b: u8) -> Self {
        Self {
            inner: SharedColor::new(a, r, g, b),
        }
    }

    #[inline]
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self::from_argb(255, r, g, b)
    }

    #[inline]
    pub fn to_argb(&self) -> u32 {
        self.inner.to_argb() as u32
    }

    #[inline]
    pub fn from_argb_u32(argb: u32) -> Self {
        Self {
            inner: SharedColor::from_argb(argb as i32),
        }
    }

    #[inline]
    pub fn a(&self) -> u8 { self.inner.alpha() }
    
    #[inline]
    pub fn r(&self) -> u8 { self.inner.red() }
    
    #[inline]
    pub fn g(&self) -> u8 { self.inner.green() }
    
    #[inline]
    pub fn b(&self) -> u8 { self.inner.blue() }

    // Common colors
    #[inline]
    pub fn white() -> Self { Self::from_rgb(255, 255, 255) }
    
    #[inline]
    pub fn black() -> Self { Self::from_rgb(0, 0, 0) }
    
    #[inline]
    pub fn transparent() -> Self { Self::from_argb(0, 0, 0, 0) }
    
    #[inline]
    pub fn red() -> Self { Self::from_rgb(255, 0, 0) }
    
    #[inline]
    pub fn green() -> Self { Self::from_rgb(0, 255, 0) }
    
    #[inline]
    pub fn blue() -> Self { Self::from_rgb(0, 0, 255) }
    
    #[inline]
    pub fn yellow() -> Self { Self::from_rgb(255, 255, 0) }
    
    #[inline]
    pub fn magenta() -> Self { Self::from_rgb(255, 0, 255) }
    
    #[inline]
    pub fn cyan() -> Self { Self::from_rgb(0, 255, 255) }
}

impl Default for Color {
    fn default() -> Self {
        Self::white()
    }
}

/// Size - Dimension (Client-specific)
/// Mirrors System.Drawing.Size
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Size {
    pub width: i32,
    pub height: i32,
}

impl Size {
    #[inline]
    pub const fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }

    #[inline]
    pub const fn zero() -> Self {
        Self { width: 0, height: 0 }
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }
}

/// Rectangle - Rectangular area (Client-specific)
/// Mirrors System.Drawing.Rectangle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rectangle {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rectangle {
    #[inline]
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self { x, y, width, height }
    }

    #[inline]
    pub fn from_location_size(location: Point, size: Size) -> Self {
        Self {
            x: location.x,
            y: location.y,
            width: size.width,
            height: size.height,
        }
    }

    #[inline]
    pub const fn left(&self) -> i32 { self.x }
    
    #[inline]
    pub const fn top(&self) -> i32 { self.y }
    
    #[inline]
    pub const fn right(&self) -> i32 { self.x + self.width }
    
    #[inline]
    pub const fn bottom(&self) -> i32 { self.y + self.height }

    #[inline]
    pub fn location(&self) -> Point {
        Point::new(self.x, self.y)
    }

    #[inline]
    pub const fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }

    /// Check if this rectangle contains a point
    #[inline]
    pub const fn contains(&self, point: Point) -> bool {
        point.x >= self.left() && point.x < self.right() &&
        point.y >= self.top() && point.y < self.bottom()
    }

    /// Check if this rectangle contains another rectangle
    pub fn contains_rect(&self, rect: Rectangle) -> bool {
        self.x <= rect.x &&
        self.y <= rect.y &&
        self.right() >= rect.right() &&
        self.bottom() >= rect.bottom()
    }

    /// Check if this rectangle intersects with another
    pub fn intersects(&self, rect: Rectangle) -> bool {
        rect.left() < self.right() &&
        self.left() < rect.right() &&
        rect.top() < self.bottom() &&
        self.top() < rect.bottom()
    }
}



/// MouseButton - Mouse button enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    None,
    Left,
    Right,
    Middle,
}

impl Default for MouseButton {
    fn default() -> Self {
        MouseButton::None
    }
}

/// KeyCode - Keyboard key codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    // Alphabet keys
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    
    // Number keys (top row)
    Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9,
    
    // Numpad keys
    Numpad0, Numpad1, Numpad2, Numpad3, Numpad4,
    Numpad5, Numpad6, Numpad7, Numpad8, Numpad9,
    
    // Function keys
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    
    // Arrow keys
    Up, Down, Left, Right,
    
    // Control keys
    Enter, Escape, Tab, Space, Backspace, Delete, Insert,
    Home, End, PageUp, PageDown,
    
    // Modifier keys
    Shift, Control, Alt,
    LeftShift, RightShift,
    LeftControl, RightControl,
    LeftAlt, RightAlt,
    
    // Punctuation and symbols
    Minus, Equals, LeftBracket, RightBracket,
    Semicolon, Quote, Comma, Period, Slash, Backslash,
    Tilde,
    
    // Lock keys
    CapsLock, NumLock, ScrollLock,
    
    // Other
    Unknown,
}

/// BlendMode - Blending mode for rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    None,
    Normal,
    Additive,
    Multiply,
    Screen,
}

impl Default for BlendMode {
    fn default() -> Self {
        BlendMode::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point() {
        let p1 = Point::new(10, 20);
        let p2 = Point::new(5, 15);
        
        // Test operators (inherited from SharedRust)
        assert_eq!(p1 + p2, Point::new(15, 35));
        assert_eq!(p1 - p2, Point::new(5, 5));
    }

    #[test]
    fn test_size() {
        let s = Size::new(100, 200);
        assert_eq!(s.width, 100);
        assert_eq!(s.height, 200);
        assert!(!s.is_empty());
        
        let empty = Size::zero();
        assert!(empty.is_empty());
    }

    #[test]
    fn test_rectangle() {
        let rect = Rectangle::new(10, 20, 100, 50);
        
        assert_eq!(rect.left(), 10);
        assert_eq!(rect.top(), 20);
        assert_eq!(rect.right(), 110);
        assert_eq!(rect.bottom(), 70);
        
        assert!(rect.contains(Point::new(50, 40)));
        assert!(!rect.contains(Point::new(5, 5)));
        assert!(!rect.contains(Point::new(120, 40)));
    }

    #[test]
    fn test_rectangle_intersection() {
        let r1 = Rectangle::new(0, 0, 100, 100);
        let r2 = Rectangle::new(50, 50, 100, 100);
        let r3 = Rectangle::new(200, 200, 50, 50);
        
        assert!(r1.intersects(r2));
        assert!(!r1.intersects(r3));
    }

    #[test]
    fn test_color() {
        let white = Color::white();
        assert_eq!(white.r(), 255);
        assert_eq!(white.g(), 255);
        assert_eq!(white.b(), 255);
        assert_eq!(white.a(), 255);
        
        let transparent = Color::transparent();
        assert_eq!(transparent.a(), 0);
        
        // Test ARGB conversion
        let color = Color::from_argb(128, 255, 128, 64);
        let argb = color.to_argb();
        let color2 = Color::from_argb_u32(argb);
        assert_eq!(color, color2);
    }

    #[test]
    fn test_color_common() {
        assert_eq!(Color::red(), Color::from_rgb(255, 0, 0));
        assert_eq!(Color::green(), Color::from_rgb(0, 255, 0));
        assert_eq!(Color::blue(), Color::from_rgb(0, 0, 255));
    }
}
