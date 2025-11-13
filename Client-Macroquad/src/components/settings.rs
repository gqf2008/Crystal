// ============================================================================
// 游戏设置组件
// ============================================================================

/// 游戏设置（全局配置）
#[derive(Debug, Clone)]
pub struct Settings {
    pub sound_enabled: bool,
    pub music_enabled: bool,
    pub volume: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sound_enabled: true,
            music_enabled: true,
            volume: 1.0,
        }
    }
}
