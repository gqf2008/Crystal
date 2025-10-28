// ============================================================================
// Sound Trigger System - 音效触发系统
// ============================================================================
//
// 🎯 Layer 3 - Presentation Decision Layer（表现层决策）
//
// 职责：
// - 监听游戏事件（攻击、受击、死亡、技能释放等）
// - 决定应该播放什么音效
// - 为实体添加SoundTriggerComponent
//
// 不负责：
// - 实际播放音效（由Layer 4的SoundPlaybackSystem负责）
// - 音效资源加载和管理
//
// ============================================================================

use hecs::{CommandBuffer, Entity, World};
// use crate::game_event::GameEvent;  // TODO: 等待GameEvent系统实现后启用
use crate::ecs::components::{SoundTriggerComponent, SoundType};

/// 音效触发系统（Layer 3）
/// 
/// # 设计原则
/// - 仅负责"决定播放什么音效"
/// - 不负责实际播放（交给Layer 4）
/// - 通过添加SoundTriggerComponent传递决策
pub struct SoundTriggerSystem;

impl SoundTriggerSystem {
    /// 处理游戏事件，决定触发哪些音效
    /// 
    /// # 参数
    /// - `world`: ECS世界
    /// - `cmd`: 命令缓冲区（用于添加组件）
    /// - `events`: 本帧的游戏事件列表
    /// 
    /// # TODO
    /// 等待GameEvent系统实现后，此方法将处理各种游戏事件
    pub fn process_events(
        _world: &World,
        _cmd: &mut CommandBuffer,
        _events: &[String], // 暂时用String代替GameEvent
    ) {
        // TODO: 实现事件处理逻辑
        // 示例代码（已注释）：
        /*
        for event in events {
            match event {
                GameEvent::PlayerAttack { entity, .. } => {
                    Self::trigger_attack_sound(cmd, *entity);
                }
                _ => {}
            }
        }
        */
    }
    
    // ========================================================================
    // 私有辅助方法 - 各种音效触发（待GameEvent系统完成后启用）
    // ========================================================================
    
    #[allow(dead_code)]
    /// 触发攻击音效
    fn trigger_attack_sound(cmd: &mut CommandBuffer, entity: Entity) {
        let sound = SoundTriggerComponent::once("attack.wav", SoundType::CharacterAction);
        cmd.insert(entity, (sound,));
    }
    
    #[allow(dead_code)]
    /// 触发受击音效（根据伤害大小选择不同音效）
    fn trigger_hit_sound(cmd: &mut CommandBuffer, entity: Entity, damage: i32) {
        let sound_file = if damage > 100 {
            "hit_heavy.wav"
        } else if damage > 50 {
            "hit_medium.wav"
        } else {
            "hit_light.wav"
        };
        
        let sound = SoundTriggerComponent::once(sound_file, SoundType::CharacterAction);
        cmd.insert(entity, (sound,));
    }
    
    #[allow(dead_code)]
    /// 触发死亡音效
    fn trigger_death_sound(cmd: &mut CommandBuffer, entity: Entity) {
        let sound = SoundTriggerComponent::once("death.wav", SoundType::CharacterAction);
        cmd.insert(entity, (sound,));
    }
    
    #[allow(dead_code)]
    /// 触发技能释放音效
    fn trigger_spell_sound(cmd: &mut CommandBuffer, entity: Entity, spell_id: u32) {
        // 根据技能ID选择不同音效
        let sound_file = match spell_id {
            1 => "spell_fireball.wav",
            2 => "spell_heal.wav",
            3 => "spell_lightning.wav",
            _ => "spell_generic.wav",
        };
        
        let sound = SoundTriggerComponent::once(sound_file, SoundType::Spell);
        cmd.insert(entity, (sound,));
    }
    
    #[allow(dead_code)]
    /// 触发技能命中音效
    fn trigger_spell_hit_sound(cmd: &mut CommandBuffer, entity: Entity) {
        let sound = SoundTriggerComponent::once("spell_impact.wav", SoundType::Spell);
        cmd.insert(entity, (sound,));
    }
    
    #[allow(dead_code)]
    /// 触发拾取音效
    fn trigger_pickup_sound(cmd: &mut CommandBuffer, entity: Entity) {
        let sound = SoundTriggerComponent::once("item_pickup.wav", SoundType::Item);
        cmd.insert(entity, (sound,));
    }
    
    #[allow(dead_code)]
    /// 触发物品使用音效
    fn trigger_item_use_sound(cmd: &mut CommandBuffer, entity: Entity) {
        let sound = SoundTriggerComponent::once("item_use.wav", SoundType::Item);
        cmd.insert(entity, (sound,));
    }
    
    #[allow(dead_code)]
    /// 触发装备音效
    fn trigger_equip_sound(cmd: &mut CommandBuffer, entity: Entity) {
        let sound = SoundTriggerComponent::once("item_equip.wav", SoundType::Item);
        cmd.insert(entity, (sound,));
    }
    
    #[allow(dead_code)]
    /// 触发UI音效（创建临时实体）
    fn trigger_ui_sound(cmd: &mut CommandBuffer, sound_file: &str) {
        let sound = SoundTriggerComponent::once(sound_file, SoundType::UI);
        // UI音效不绑定到任何实体，创建临时实体
        cmd.spawn((sound,));
    }
    
    #[allow(dead_code)]
    /// 触发升级音效
    fn trigger_levelup_sound(cmd: &mut CommandBuffer, entity: Entity) {
        let sound = SoundTriggerComponent::once("levelup.wav", SoundType::System)
            .with_volume(0.8);
        cmd.insert(entity, (sound,));
    }
    
    #[allow(dead_code)]
    /// 触发任务完成音效
    fn trigger_quest_complete_sound(cmd: &mut CommandBuffer) {
        let sound = SoundTriggerComponent::once("quest_complete.wav", SoundType::System);
        cmd.spawn((sound,));
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sound_trigger_system() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        
        let player = world.spawn(());
        
        let events = vec![
            GameEvent::PlayerAttack { entity: player, target: None },
        ];
        
        SoundTriggerSystem::process_events(&world, &mut cmd, &events);
        cmd.run_on(&mut world);
        
        // 验证音效组件已添加
        assert!(world.get::<SoundTriggerComponent>(player).is_ok());
    }
}
