// Supported Resolution Enum
// Mirrors Client.Resolution.eSupportedResolution

use std::fmt;
use serde::{Deserialize, Serialize};

/// Supported game resolutions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SupportedResolution {
    /// 1024x768 (4:3)
    W1024H768 = 1024,
    /// 1280x720 (16:9, 720p)
    W1280H720 = 1280,
    /// 1366x768 (16:9, common laptop)
    W1366H768 = 1366,
    /// 1920x1080 (16:9, 1080p)
    W1920H1080 = 1920,
}

impl SupportedResolution {
    /// Get width of this resolution
    pub fn width(&self) -> u32 {
        match self {
            SupportedResolution::W1024H768 => 1024,
            SupportedResolution::W1280H720 => 1280,
            SupportedResolution::W1366H768 => 1366,
            SupportedResolution::W1920H1080 => 1920,
        }
    }
    
    /// Get height of this resolution
    pub fn height(&self) -> u32 {
        match self {
            SupportedResolution::W1024H768 => 768,
            SupportedResolution::W1280H720 => 720,
            SupportedResolution::W1366H768 => 768,
            SupportedResolution::W1920H1080 => 1080,
        }
    }
    
    /// Get (width, height) tuple
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width(), self.height())
    }
    
    /// Get aspect ratio as a float
    pub fn aspect_ratio(&self) -> f32 {
        self.width() as f32 / self.height() as f32
    }
    
    /// Parse from width value
    pub fn from_width(width: u32) -> Option<Self> {
        match width {
            1024 => Some(SupportedResolution::W1024H768),
            1280 => Some(SupportedResolution::W1280H720),
            1366 => Some(SupportedResolution::W1366H768),
            1920 => Some(SupportedResolution::W1920H1080),
            _ => None,
        }
    }
    
    /// Parse from string (e.g., "w1920h1080", "1920x1080", "1920")
    pub fn from_string(s: &str) -> Option<Self> {
        let s_lower = s.to_lowercase();
        
        // Try format: "w1920h1080"
        if s_lower.starts_with('w') && s_lower.contains('h') {
            if s_lower == "w1024h768" {
                return Some(SupportedResolution::W1024H768);
            } else if s_lower == "w1280h720" {
                return Some(SupportedResolution::W1280H720);
            } else if s_lower == "w1366h768" {
                return Some(SupportedResolution::W1366H768);
            } else if s_lower == "w1920h1080" {
                return Some(SupportedResolution::W1920H1080);
            }
        }
        
        // Try format: "1920x1080"
        if s.contains('x') || s.contains('X') {
            let parts: Vec<&str> = s.split(['x', 'X']).collect();
            if parts.len() == 2 {
                if let (Ok(w), Ok(h)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                    return Self::from_dimensions(w, h);
                }
            }
        }
        
        // Try just width
        if let Ok(width) = s.parse::<u32>() {
            return Self::from_width(width);
        }
        
        None
    }
    
    /// Parse from dimensions
    pub fn from_dimensions(width: u32, height: u32) -> Option<Self> {
        match (width, height) {
            (1024, 768) => Some(SupportedResolution::W1024H768),
            (1280, 720) => Some(SupportedResolution::W1280H720),
            (1366, 768) => Some(SupportedResolution::W1366H768),
            (1920, 1080) => Some(SupportedResolution::W1920H1080),
            _ => None,
        }
    }
    
    /// Get all supported resolutions
    pub fn all() -> &'static [SupportedResolution] {
        &[
            SupportedResolution::W1024H768,
            SupportedResolution::W1280H720,
            SupportedResolution::W1366H768,
            SupportedResolution::W1920H1080,
        ]
    }
    
    /// Check if a resolution is supported
    pub fn is_supported(width: u32, height: u32) -> bool {
        Self::from_dimensions(width, height).is_some()
    }
}

impl fmt::Display for SupportedResolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}x{}", self.width(), self.height())
    }
}

impl Default for SupportedResolution {
    fn default() -> Self {
        SupportedResolution::W1024H768
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dimensions() {
        assert_eq!(SupportedResolution::W1920H1080.dimensions(), (1920, 1080));
        assert_eq!(SupportedResolution::W1024H768.dimensions(), (1024, 768));
    }
    
    #[test]
    fn test_aspect_ratio() {
        let res = SupportedResolution::W1920H1080;
        assert!((res.aspect_ratio() - 16.0 / 9.0).abs() < 0.01);
        
        let res = SupportedResolution::W1024H768;
        assert!((res.aspect_ratio() - 4.0 / 3.0).abs() < 0.01);
    }
    
    #[test]
    fn test_from_width() {
        assert_eq!(SupportedResolution::from_width(1920), Some(SupportedResolution::W1920H1080));
        assert_eq!(SupportedResolution::from_width(1024), Some(SupportedResolution::W1024H768));
        assert_eq!(SupportedResolution::from_width(999), None);
    }
    
    #[test]
    fn test_from_string() {
        assert_eq!(
            SupportedResolution::from_string("w1920h1080"),
            Some(SupportedResolution::W1920H1080)
        );
        assert_eq!(
            SupportedResolution::from_string("W1920H1080"),
            Some(SupportedResolution::W1920H1080)
        );
        assert_eq!(
            SupportedResolution::from_string("1920x1080"),
            Some(SupportedResolution::W1920H1080)
        );
        assert_eq!(
            SupportedResolution::from_string("1920"),
            Some(SupportedResolution::W1920H1080)
        );
        assert_eq!(SupportedResolution::from_string("invalid"), None);
    }
    
    #[test]
    fn test_from_dimensions() {
        assert_eq!(
            SupportedResolution::from_dimensions(1920, 1080),
            Some(SupportedResolution::W1920H1080)
        );
        assert_eq!(
            SupportedResolution::from_dimensions(999, 999),
            None
        );
    }
    
    #[test]
    fn test_display() {
        assert_eq!(format!("{}", SupportedResolution::W1920H1080), "1920x1080");
        assert_eq!(format!("{}", SupportedResolution::W1024H768), "1024x768");
    }
    
    #[test]
    fn test_is_supported() {
        assert!(SupportedResolution::is_supported(1920, 1080));
        assert!(SupportedResolution::is_supported(1024, 768));
        assert!(!SupportedResolution::is_supported(999, 999));
    }
}
