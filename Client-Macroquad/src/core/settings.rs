// ============================================================================
// 游戏配置
// ============================================================================

use serde::{Deserialize, Serialize};

/// 游戏配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct GameSettings {
    /// 窗口配置
    pub window: WindowSettings,
    
    /// 渲染配置
    pub render: RenderSettings,
    
    /// 网络配置
    pub network: NetworkSettings,
}

/// 窗口配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSettings {
    pub width: i32,
    pub height: i32,
    pub title: String,
    pub resizable: bool,
    pub vsync: bool,
}

/// 渲染配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderSettings {
    /// Gamma校正值 (1.0 = 无校正, 2.2 = 标准sRGB)
    pub gamma: f32,

    /// 亮度增益 (1.0 = 原始亮度, >1.0 变亮, <1.0 变暗)
    pub brightness: f32,

    /// 对比度增益 (1.0 = 原始对比度, >1.0 增强, <1.0 降低)
    pub contrast: f32,

    /// 饱和度增益 (1.0 = 原始饱和度, 0.0 = 灰度, >1.0 过饱和)
    pub saturation: f32,

    /// 混合模式 Alpha (0.0-1.0, 控制纹理透明度)
    pub blend_alpha: f32,

    /// 是否启用色调映射 (HDR -> LDR)
    pub tone_mapping: bool,

    /// 目标最大帧率 (PR #1167: 用 config 替代硬编码 60)
    /// 默认 60 FPS,范围 30-300。
    pub max_fps: u32,
}

/// 网络配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSettings {
    pub server_address: String,
    pub server_port: u16,
    pub timeout_ms: u64,
}


impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            width: 1024,
            height: 768,
            title: "传奇2 - Macroquad".to_string(),
            resizable: true,
            vsync: true,
        }
    }
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            gamma: 1.0,
            brightness: 1.0,
            contrast: 1.0,
            saturation: 1.0,
            blend_alpha: 1.0,
            tone_mapping: false,
            max_fps: 60, // PR #1167: 默认 60 FPS
        }
    }
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            server_address: "127.0.0.1".to_string(),
            server_port: 7000,
            timeout_ms: 5000,
        }
    }
}

impl RenderSettings {
    /// 高质量预设
    pub fn preset_high_quality() -> Self {
        Self {
            gamma: 2.2,
            brightness: 1.1,
            contrast: 1.1,
            saturation: 1.05,
            blend_alpha: 1.0,
            tone_mapping: true,
            max_fps: 60,
        }
    }

    /// 低质量预设 (高性能)
    pub fn preset_low_quality() -> Self {
        Self::default()
    }

    /// 复古风格预设
    pub fn preset_retro() -> Self {
        Self {
            gamma: 1.8,
            brightness: 0.95,
            contrast: 1.2,
            saturation: 0.85,
            blend_alpha: 1.0,
            tone_mapping: false,
            max_fps: 60,
        }
    }
}
