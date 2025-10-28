// ============================================================================
// Sound Playback System - 音效播放系统
// ============================================================================
//
// 🎯 Layer 4 - Rendering & Playback Layer（渲染与播放层）
//
// 职责：
// - 读取Layer 3创建的SoundTriggerComponent
// - 实际播放音效
// - 管理音效实例的生命周期
// - 播放完成后移除SoundTriggerComponent
//
// 不负责：
// - 决定播放什么音效（由Layer 3负责）
// - 游戏逻辑
//
// ============================================================================

use hecs::{CommandBuffer, Entity, World};
use ggez::{Context, GameResult};
// use ggez::audio::{Source, SoundSource};  // TODO: GGEZ音频API待确认
use std::collections::HashMap;
use crate::ecs::components::{SoundTriggerComponent, PersistentSoundComponent, SoundType};

/// 音效播放系统（Layer 4）
/// 
/// # 设计原则
/// - 仅负责"实际播放音效"
/// - 不决定播放什么（由Layer 3决定）
/// - 读取SoundTriggerComponent并执行播放
/// 
/// # TODO
/// 等待GGEZ音频API确认后实现实际播放逻辑
pub struct SoundPlaybackSystem {
    /// 音效资源缓存（路径到音效数据的映射）
    sound_cache: HashMap<String, Vec<u8>>,  // 暂时用Vec<u8>代替Source
    
    /// 正在播放的持续音效
    playing_sounds: HashMap<Entity, String>,  // 暂时只存储文件名
    
    /// 全局音量设置
    master_volume: f32,
    bgm_volume: f32,
    sfx_volume: f32,
}

impl SoundPlaybackSystem {
    /// 创建新的音效播放系统
    pub fn new() -> Self {
        Self {
            sound_cache: HashMap::new(),
            playing_sounds: HashMap::new(),
            master_volume: 1.0,
            bgm_volume: 0.7,
            sfx_volume: 1.0,
        }
    }
    
    /// 更新系统，处理所有音效触发
    /// 
    /// # 参数
    /// - `ctx`: ggez上下文（用于音频播放）
    /// - `world`: ECS世界
    /// - `cmd`: 命令缓冲区（用于移除已播放的触发组件）
    /// 
    /// # TODO
    /// 等待GGEZ音频API确认后实现
    pub fn update(&mut self, _ctx: &mut Context, world: &World, cmd: &mut CommandBuffer) -> GameResult {
        // TODO: 处理一次性触发音效
        // self.process_sound_triggers(ctx, world, cmd)?;
        
        // TODO: 处理持续音效
        // self.process_persistent_sounds(ctx, world)?;
        
        // 暂时只移除SoundTriggerComponent（避免累积）
        let mut entities_to_remove = Vec::new();
        for (entity, _trigger) in world.query::<&SoundTriggerComponent>().iter() {
            entities_to_remove.push(entity);
        }
        for entity in entities_to_remove {
            cmd.remove::<(SoundTriggerComponent,)>(entity);
        }
        
        Ok(())
    }
    
    // ========================================================================
    // 一次性音效处理（待GGEZ音频API确认后实现）
    // ========================================================================
    
    #[allow(dead_code)]
    /// 处理所有SoundTriggerComponent
    fn process_sound_triggers(
        &mut self,
        _ctx: &mut Context,
        world: &World,
        cmd: &mut CommandBuffer,
    ) -> GameResult {
        let mut entities_to_remove = Vec::new();
        
        // 查询所有带有SoundTriggerComponent的实体
        for (entity, _trigger) in world.query::<&SoundTriggerComponent>().iter() {
            // TODO: 播放音效
            // self.play_sound(ctx, trigger)?;
            
            // 记录需要移除的实体（播放完成后移除触发组件）
            entities_to_remove.push(entity);
        }
        
        // 移除已播放的触发组件
        for entity in entities_to_remove {
            cmd.remove::<(SoundTriggerComponent,)>(entity);
        }
        
        Ok(())
    }
    
    #[allow(dead_code)]
    /// 播放单个音效
    fn play_sound(&mut self, _ctx: &mut Context, trigger: &SoundTriggerComponent) -> GameResult {
        // 计算最终音量
        let _final_volume = self.calculate_volume(trigger);
        
        // TODO: 实际播放逻辑
        
        Ok(())
    }
    
    #[allow(dead_code)]
    /// 从缓存获取音效，如果不存在则加载
    fn get_or_load_sound(&mut self, _ctx: &mut Context, sound_file: &str) -> GameResult<Vec<u8>> {
        // 如果已缓存，克隆返回
        if let Some(cached) = self.sound_cache.get(sound_file) {
            return Ok(cached.clone());
        }
        
        // TODO: 加载新音效
        let sound_data = Vec::new();
        self.sound_cache.insert(sound_file.to_string(), sound_data.clone());
        
        Ok(sound_data)
    }
    
    // ========================================================================
    // 持续音效处理（待GGEZ音频API确认后实现）
    // ========================================================================
    
    #[allow(dead_code)]
    /// 处理持续音效（背景音乐、环境音）
    fn process_persistent_sounds(&mut self, _ctx: &mut Context, world: &World) -> GameResult {
        for (entity, persistent) in world.query::<&PersistentSoundComponent>().iter() {
            if persistent.is_playing {
                // 检查是否已在播放
                if !self.playing_sounds.contains_key(&entity) {
                    // TODO: 开始播放
                    self.playing_sounds.insert(entity, persistent.sound_file.clone());
                }
            } else {
                // 停止播放
                if let Some(_sound) = self.playing_sounds.remove(&entity) {
                    // TODO: 停止播放
                }
            }
        }
        
        Ok(())
    }
    
    // ========================================================================
    // 音量计算
    // ========================================================================
    
    /// 计算最终音量（考虑音效类型和全局设置）
    fn calculate_volume(&self, trigger: &SoundTriggerComponent) -> f32 {
        let type_volume = match trigger.sound_type {
            SoundType::BackgroundMusic => self.bgm_volume,
            _ => self.sfx_volume,
        };
        
        trigger.volume * type_volume * self.master_volume
    }
    
    /// 计算持续音效的音量
    fn calculate_persistent_volume(&self, persistent: &PersistentSoundComponent) -> f32 {
        let type_volume = match persistent.sound_type {
            SoundType::BackgroundMusic => self.bgm_volume,
            _ => self.sfx_volume,
        };
        
        persistent.volume * type_volume * self.master_volume
    }
    
    // ========================================================================
    // 公共API - 音量控制
    // ========================================================================
    
    /// 设置主音量
    pub fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = volume.clamp(0.0, 1.0);
    }
    
    /// 设置背景音乐音量
    pub fn set_bgm_volume(&mut self, volume: f32) {
        self.bgm_volume = volume.clamp(0.0, 1.0);
    }
    
    /// 设置音效音量
    pub fn set_sfx_volume(&mut self, volume: f32) {
        self.sfx_volume = volume.clamp(0.0, 1.0);
    }
    
    /// 停止所有音效
    pub fn stop_all(&mut self, _ctx: &mut Context) -> GameResult {
        // TODO: 实际停止逻辑
        self.playing_sounds.clear();
        Ok(())
    }
}

impl Default for SoundPlaybackSystem {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_calculation() {
        let system = SoundPlaybackSystem::new();
        
        let trigger = SoundTriggerComponent {
            sound_file: "test.wav".to_string(),
            sound_type: SoundType::BackgroundMusic,
            volume: 0.5,
            looping: false,
        };
        
        let volume = system.calculate_volume(&trigger);
        
        // master(1.0) * bgm(0.7) * trigger(0.5) = 0.35
        assert!((volume - 0.35).abs() < 0.01);
    }
}
