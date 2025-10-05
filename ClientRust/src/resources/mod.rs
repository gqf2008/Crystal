// Resources Module - Embedded UI images and assets
// Mirrors Client/Resources/

use image::{DynamicImage, ImageError, ImageFormat};

/// UI resource images embedded at compile time
pub struct Images;

impl Images {
    /// Blue progress bar image
    pub fn blue_progress() -> &'static [u8] {
        include_bytes!("../../resources/ui/Blue Progress.png")
    }
    
    /// Checkbox base state (unchecked)
    pub fn checkf_base2() -> &'static [u8] {
        include_bytes!("../../resources/ui/CheckF_Base2.png")
    }
    
    /// Checkbox hover state
    pub fn checkf_hover() -> &'static [u8] {
        include_bytes!("../../resources/ui/CheckF_Hover.png")
    }
    
    /// Checkbox pressed state
    pub fn checkf_pressed() -> &'static [u8] {
        include_bytes!("../../resources/ui/CheckF_Pressed.png")
    }
    
    /// Config button base state
    pub fn config_base() -> &'static [u8] {
        include_bytes!("../../resources/ui/Config_Base.png")
    }
    
    /// Config button base state (variant 1)
    pub fn config_base1() -> &'static [u8] {
        include_bytes!("../../resources/ui/Config_Base1.png")
    }
    
    /// Config checkbox off state
    pub fn config_check_off1() -> &'static [u8] {
        include_bytes!("../../resources/ui/Config_Check_Off1.png")
    }
    
    /// Config checkbox on state
    pub fn config_check_on() -> &'static [u8] {
        include_bytes!("../../resources/ui/Config_Check_On.png")
    }
    
    /// Config button hover state
    pub fn config_hover() -> &'static [u8] {
        include_bytes!("../../resources/ui/Config_Hover.png")
    }
    
    /// Config button pressed state
    pub fn config_pressed() -> &'static [u8] {
        include_bytes!("../../resources/ui/Config_Pressed.png")
    }
    
    /// Config radio button on state
    pub fn config_radio_on() -> &'static [u8] {
        include_bytes!("../../resources/ui/Config_Radio_On.png")
    }
    
    /// Close/Cross button base state
    pub fn cross_base() -> &'static [u8] {
        include_bytes!("../../resources/ui/Cross_Base.png")
    }
    
    /// Close/Cross button hover state
    pub fn cross_hover() -> &'static [u8] {
        include_bytes!("../../resources/ui/Cross_Hover.png")
    }
    
    /// Close/Cross button pressed state
    pub fn cross_pressed() -> &'static [u8] {
        include_bytes!("../../resources/ui/Cross_Pressed.png")
    }
    
    /// Green progress bar image
    pub fn green_progress() -> &'static [u8] {
        include_bytes!("../../resources/ui/Green Progress.png")
    }
    
    /// Launch button base state
    pub fn launch_base() -> &'static [u8] {
        include_bytes!("../../resources/ui/Launch_Base.png")
    }
    
    /// Launch button base state (variant 1)
    pub fn launch_base1() -> &'static [u8] {
        include_bytes!("../../resources/ui/Launch_Base1.png")
    }
    
    /// Launch button hover state
    pub fn launch_hover() -> &'static [u8] {
        include_bytes!("../../resources/ui/Launch_Hover.png")
    }
    
    /// Launch button pressed state
    pub fn launch_pressed() -> &'static [u8] {
        include_bytes!("../../resources/ui/Launch_Pressed.png")
    }
    
    /// Progress bar end cap (blue)
    pub fn new_progress_end_blue() -> &'static [u8] {
        include_bytes!("../../resources/ui/NEW Progress End (Blue).png")
    }
    
    /// Progress bar end cap (green)
    pub fn new_progress_end_green() -> &'static [u8] {
        include_bytes!("../../resources/ui/NEW Progress End (Green).png")
    }
    
    /// Pfffft image
    pub fn pfffft() -> &'static [u8] {
        include_bytes!("../../resources/ui/pfffft.png")
    }
    
    /// Radio button unactive/off state
    pub fn radio_unactive() -> &'static [u8] {
        include_bytes!("../../resources/ui/Radio_Unactive.png")
    }
    
    /// Server selection base image
    pub fn server_base() -> &'static [u8] {
        include_bytes!("../../resources/ui/server_base.png")
    }
    
    /// Textbox background images
    pub fn textboxes() -> &'static [u8] {
        include_bytes!("../../resources/ui/textboxes.png")
    }
}

/// Image loading helper methods using the `image` crate
impl Images {
    /// Load any resource as a DynamicImage
    pub fn load_image(bytes: &'static [u8]) -> Result<DynamicImage, ImageError> {
        image::load_from_memory_with_format(bytes, ImageFormat::Png)
    }
    
    /// Load blue progress bar as image
    pub fn load_blue_progress() -> Result<DynamicImage, ImageError> {
        Self::load_image(Self::blue_progress())
    }
    
    /// Load green progress bar as image
    pub fn load_green_progress() -> Result<DynamicImage, ImageError> {
        Self::load_image(Self::green_progress())
    }
    
    /// Load launch button (base state) as image
    pub fn load_launch_base() -> Result<DynamicImage, ImageError> {
        Self::load_image(Self::launch_base())
    }
    
    /// Load launch button (hover state) as image
    pub fn load_launch_hover() -> Result<DynamicImage, ImageError> {
        Self::load_image(Self::launch_hover())
    }
    
    /// Load launch button (pressed state) as image
    pub fn load_launch_pressed() -> Result<DynamicImage, ImageError> {
        Self::load_image(Self::launch_pressed())
    }
    
    /// Load config button (base state) as image
    pub fn load_config_base() -> Result<DynamicImage, ImageError> {
        Self::load_image(Self::config_base())
    }
    
    /// Load config button (hover state) as image
    pub fn load_config_hover() -> Result<DynamicImage, ImageError> {
        Self::load_image(Self::config_hover())
    }
    
    /// Load config button (pressed state) as image
    pub fn load_config_pressed() -> Result<DynamicImage, ImageError> {
        Self::load_image(Self::config_pressed())
    }
    
    /// Load close/cross button (base state) as image
    pub fn load_cross_base() -> Result<DynamicImage, ImageError> {
        Self::load_image(Self::cross_base())
    }
    
    /// Load close/cross button (hover state) as image
    pub fn load_cross_hover() -> Result<DynamicImage, ImageError> {
        Self::load_image(Self::cross_hover())
    }
    
    /// Load close/cross button (pressed state) as image
    pub fn load_cross_pressed() -> Result<DynamicImage, ImageError> {
        Self::load_image(Self::cross_pressed())
    }
    
    /// Load server background as image
    pub fn load_server_base() -> Result<DynamicImage, ImageError> {
        Self::load_image(Self::server_base())
    }
    
    /// Load checkbox (base state) as image
    pub fn load_checkf_base2() -> Result<DynamicImage, ImageError> {
        Self::load_image(Self::checkf_base2())
    }
    
    /// Load checkbox (hover state) as image
    pub fn load_checkf_hover() -> Result<DynamicImage, ImageError> {
        Self::load_image(Self::checkf_hover())
    }
    
    /// Load checkbox (pressed state) as image
    pub fn load_checkf_pressed() -> Result<DynamicImage, ImageError> {
        Self::load_image(Self::checkf_pressed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_resources_not_empty() {
        // Test that resources are properly embedded
        assert!(!Images::blue_progress().is_empty());
        assert!(!Images::green_progress().is_empty());
        assert!(!Images::launch_base().is_empty());
    }
    
    #[test]
    fn test_all_resources_exist() {
        // Verify all resource getters work
        let _ = Images::blue_progress();
        let _ = Images::checkf_base2();
        let _ = Images::checkf_hover();
        let _ = Images::checkf_pressed();
        let _ = Images::config_base();
        let _ = Images::config_base1();
        let _ = Images::config_check_off1();
        let _ = Images::config_check_on();
        let _ = Images::config_hover();
        let _ = Images::config_pressed();
        let _ = Images::config_radio_on();
        let _ = Images::cross_base();
        let _ = Images::cross_hover();
        let _ = Images::cross_pressed();
        let _ = Images::green_progress();
        let _ = Images::launch_base();
        let _ = Images::launch_base1();
        let _ = Images::launch_hover();
        let _ = Images::launch_pressed();
        let _ = Images::new_progress_end_blue();
        let _ = Images::new_progress_end_green();
        let _ = Images::pfffft();
        let _ = Images::radio_unactive();
        let _ = Images::server_base();
        let _ = Images::textboxes();
    }
    
    #[test]
    fn test_load_images_with_image_crate() {
        // Test loading with image crate
        let img = Images::load_blue_progress().expect("Failed to load blue progress");
        assert!(img.width() > 0);
        assert!(img.height() > 0);
        
        let img = Images::load_launch_base().expect("Failed to load launch button");
        assert!(img.width() > 0);
        assert!(img.height() > 0);
        
        let img = Images::load_server_base().expect("Failed to load server background");
        assert!(img.width() > 0);
        assert!(img.height() > 0);
    }
    
    #[test]
    fn test_image_dimensions() {
        // Verify images have expected properties
        use image::GenericImageView;
        
        let img = Images::load_blue_progress().unwrap();
        let (width, height) = img.dimensions();
        println!("Blue progress: {}x{}", width, height);
        assert!(width > 0 && height > 0);
        
        let img = Images::load_launch_base().unwrap();
        let (width, height) = img.dimensions();
        println!("Launch button: {}x{}", width, height);
        assert!(width > 0 && height > 0);
    }
}
