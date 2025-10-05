// Resolution module - Screen resolution management
// Mirrors Client/Resolution/

pub mod supported_resolution;
pub mod display_resolutions;

// Re-export main types
pub use supported_resolution::SupportedResolution;
pub use display_resolutions::DisplayResolutions;
