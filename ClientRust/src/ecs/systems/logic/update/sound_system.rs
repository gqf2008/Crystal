// ============================================================================
// Layer 5: State Update - SoundSystem
// Priority: 520
// ============================================================================
//
// **职责**：
// - 音效触发管理
// - 3D音效位置计算
// - 音量控制
//
// **逻辑来源**：
// - C# SoundManager.PlaySound(): 播放音效
// - 根据距离调整音量
//
// ============================================================================

use hecs::World;
use ggez::GameResult;
use crate::ecs::{GameContext, systems::{System, priority}};

/// 音效系统
pub struct SoundSystem {
    /// 听者位置(通常是摄像机/玩家位置)
    listener_pos: (f32, f32),
    /// 最大听音距离
    max_distance: f32,
}

impl SoundSystem {
    pub fn new() -> Self {
        Self {
            listener_pos: (0.0, 0.0),
            max_distance: 1000.0,
        }
    }

    /// 计算3D音效音量(基于距离衰减)
    #[allow(dead_code)]
    fn calculate_volume(&self, sound_pos: (f32, f32)) -> f32 {
        let dx = sound_pos.0 - self.listener_pos.0;
        let dy = sound_pos.1 - self.listener_pos.1;
        let distance = (dx * dx + dy * dy).sqrt();

        if distance >= self.max_distance {
            0.0
        } else {
            (1.0 - distance / self.max_distance).max(0.0)
        }
    }
}

impl Default for SoundSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl System for SoundSystem {
    fn priority(&self) -> u32 {
        priority::SOUND
    }

    fn update(&mut self, _ctx: &mut GameContext, _delay_time: f32) -> GameResult {
        // TODO: 实现音效触发和3D音量计算
        // 需要先定义SoundSource组件
        Ok(())
    }
}
