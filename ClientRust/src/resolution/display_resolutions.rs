// Display Resolutions - Query system display modes
// Mirrors Client.Resolution.DisplayResolutions

use std::sync::Mutex;
use once_cell::sync::Lazy;

use super::supported_resolution::SupportedResolution;

/// Global list of supported display resolutions detected on this system
static DISPLAY_SUPPORTED_RESOLUTIONS: Lazy<Mutex<Vec<SupportedResolution>>> = 
    Lazy::new(|| Mutex::new(Vec::new()));

/// Display resolutions manager
pub struct DisplayResolutions;

impl DisplayResolutions {
    /// Get display resolutions supported by the system
    /// 
    /// # Returns
    /// * `true` if any supported resolutions were detected
    /// * `false` if detection failed or no supported resolutions found
    pub fn get_display_resolutions() -> bool {
        let detected_resolutions;
        
        #[cfg(target_os = "windows")]
        {
            detected_resolutions = Self::get_windows_resolutions();
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            // For non-Windows platforms, assume all resolutions are available
            // This can be enhanced with platform-specific detection later
            log::warn!("Display resolution detection not implemented for this platform");
            detected_resolutions = SupportedResolution::all().to_vec();
        }
        
        if detected_resolutions.is_empty() {
            return false;
        }
        
        // Update global list
        if let Ok(mut list) = DISPLAY_SUPPORTED_RESOLUTIONS.lock() {
            *list = detected_resolutions;
            true
        } else {
            false
        }
    }
    
    /// Get list of detected supported resolutions
    pub fn get_supported_resolutions() -> Vec<SupportedResolution> {
        DISPLAY_SUPPORTED_RESOLUTIONS
            .lock()
            .map(|list| list.clone())
            .unwrap_or_default()
    }
    
    /// Check if a resolution is supported (by width)
    pub fn is_supported(resolution: u32) -> bool {
        Self::is_supported_str(&resolution.to_string())
    }
    
    /// Check if a resolution is supported (by string)
    pub fn is_supported_str(resolution: &str) -> bool {
        if let Some(res) = SupportedResolution::from_string(resolution) {
            // Check if it's a valid enum value
            SupportedResolution::all().contains(&res)
        } else {
            false
        }
    }
    
    /// Check if a specific resolution is available on this system
    pub fn is_available(resolution: SupportedResolution) -> bool {
        DISPLAY_SUPPORTED_RESOLUTIONS
            .lock()
            .map(|list| list.contains(&resolution))
            .unwrap_or(false)
    }
    
    #[cfg(target_os = "windows")]
    fn get_windows_resolutions() -> Vec<SupportedResolution> {
        use winapi::um::winuser::EnumDisplaySettingsW;
        use winapi::um::wingdi::DEVMODEW;
        use std::mem;
        use std::ptr;
        
        let mut detected = Vec::new();
        let mut seen = std::collections::HashSet::new();
        
        unsafe {
            let mut dev_mode: DEVMODEW = mem::zeroed();
            dev_mode.dmSize = mem::size_of::<DEVMODEW>() as u16;
            
            let mut mode_num: u32 = 0;
            
            // Enumerate all display modes
            while EnumDisplaySettingsW(
                ptr::null(),
                mode_num,
                &mut dev_mode,
            ) != 0
            {
                let width = dev_mode.dmPelsWidth;
                let height = dev_mode.dmPelsHeight;
                
                // Check if this is a supported resolution
                if let Some(res) = SupportedResolution::from_dimensions(width, height) {
                    // Avoid duplicates
                    if seen.insert((width, height)) {
                        detected.push(res);
                    }
                }
                
                mode_num += 1;
            }
        }
        
        // Sort by width
        detected.sort_by_key(|r| r.width());
        detected
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_is_supported_str() {
        assert!(DisplayResolutions::is_supported_str("1920"));
        assert!(DisplayResolutions::is_supported_str("1920x1080"));
        assert!(DisplayResolutions::is_supported_str("w1920h1080"));
        assert!(!DisplayResolutions::is_supported_str("invalid"));
        assert!(!DisplayResolutions::is_supported_str("999"));
    }
    
    #[test]
    fn test_is_supported() {
        assert!(DisplayResolutions::is_supported(1920));
        assert!(DisplayResolutions::is_supported(1024));
        assert!(!DisplayResolutions::is_supported(999));
    }
    
    #[test]
    #[ignore] // This test requires actual display enumeration
    fn test_get_display_resolutions() {
        let result = DisplayResolutions::get_display_resolutions();
        // Should succeed on any system
        assert!(result || cfg!(not(target_os = "windows")));
        
        let resolutions = DisplayResolutions::get_supported_resolutions();
        // Should have at least one resolution
        if cfg!(target_os = "windows") {
            assert!(!resolutions.is_empty());
        }
    }
}
