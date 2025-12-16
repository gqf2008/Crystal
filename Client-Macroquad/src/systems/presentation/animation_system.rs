// ============================================================================
// Attack System - 攻击动画管理系统
// Priority: 400 (在 PlayerStateSystem 之后, MovementSystem 之前)
// ============================================================================
//
// **职责**:
// - 检测攻击动画完成
// - 自动移除 AttackState 组件
// - 恢复角色到 Stand 状态
//
// **ECS 设计原则**:
// - ✅ 无状态 System (所有状态存储在 AttackState Component)
// - ✅ 单一职责 (只负责攻击动画生命周期管理)
// - ✅ 组件驱动 (通过 AttackState 组件查询攻击中的实体)
//
// **数据流**:
// ```
// PlayerControlSystem (右键点击)
//     ↓ 添加 AttackState 组件
// AnimationSystem (检测动画完成)
//     ↓ 移除 AttackState 组件 + 设置 Stand
// ```
//
// ============================================================================

use crate::game::{GameResult, GameContext};
use std::time::Instant;
use crate::{
    components::{
        AnimationFrame, AttackSoundPlayed, AttackState, MountState, MountStatus, Player, PlayerAction, PlayerAppearance, SoundTrigger, SoundType, TimeTracker,
    },
    systems::LogicSystem,
};
use crate::objects::frames::{get_default_monster_frame, get_player_frame};
use crate::systems::logic::combat::CombatSystem;

#[derive(ecs_macros::LogicSystem)]
pub struct AnimationSystem;

impl AnimationSystem {
    pub fn new() -> Self {
        Self
    }
}

impl AnimationSystem {
    /// 更新所有动画帧
    /// 
    /// **职责**: 计算并更新实体的动画帧索引
    /// - 角色身体动画帧
    /// - 武器动画帧
    pub fn update_animation_frames(&mut self, ctx: &mut GameContext) -> GameResult {
        // 获取全局时间跟踪器
        let time_tracker = ctx
            .world
            .query::<&TimeTracker>()
            .iter()
            .next()
            .map(|(_, t)| t.clone())
            .unwrap_or_default();

        let now = Instant::now();

        // 更新所有角色的动画帧
        for (_entity, (player, mount_state, attack_state, anim_frame)) in ctx
            .world
            .query_mut::<(&Player, Option<&MountState>, Option<&AttackState>, &mut AnimationFrame)>()
            .into_iter()
        {
            // C# 原版核心：
            // - DrawFrame = Frame.Start + Frame.OffSet * Direction + FrameIndex
            // - DrawWingFrame = Frame.EffectStart + Frame.EffectOffSet * Direction + EffectFrameIndex
            // - 角色骑乘时 CurrentAction 会切到 Mount*，从而同时影响 DrawFrame/DrawWingFrame

            let mounted = mount_state.and_then(|m| m.mount_index).is_some();
            let (draw_frame, effect_frame, frame_index) =
                Self::calculate_frames(player, &time_tracker, mounted, now, attack_state.copied());

            anim_frame.character_frame = draw_frame;
            // 武器/武器特效：C# 用同一套 DrawFrame，只是在取纹理时叠加 WeaponOffSet
            anim_frame.weapon_frame = draw_frame;
            anim_frame.effect_frame = effect_frame;
            anim_frame.action_frame_index = frame_index;
        }

        Ok(())
    }

    /// 计算角色动画帧索引
    ///
    /// **重构说明**: 现在从 `objects/frames.rs` 的 `PLAYER_FRAMES` 读取配置
    /// 
    /// C# 逻辑参考: PlayerObject.cs DrawBody()
    /// ```csharp
    /// int index = BaseIndex + (Direction * FrameCount) + CurrentFrame
    /// ```
    fn calculate_frames(
        player: &Player,
        time_tracker: &TimeTracker,
        mounted: bool,
        now: Instant,
        attack_state: Option<AttackState>,
    ) -> (i32, i32, i32) {
        Self::calculate_frames_with_attack(player, time_tracker, mounted, now, attack_state)
    }

    fn calculate_frames_with_attack(
        player: &Player,
        time_tracker: &TimeTracker,
        mounted: bool,
        now: Instant,
        attack_state: Option<AttackState>,
    ) -> (i32, i32, i32) {
        // 选择 MirAction（对齐 C#：骑乘时使用 Mount*）
        let mir_action = if mounted {
            match player.action {
                PlayerAction::Walk => mir2_shared::enums::MirAction::MountWalking,
                PlayerAction::Run => mir2_shared::enums::MirAction::MountRunning,
                // 骑乘攻击：使用 MountAttack（原版就是一套骑乘攻击动作）
                PlayerAction::Attack1 | PlayerAction::Attack2 | PlayerAction::Attack3 => {
                    mir2_shared::enums::MirAction::MountAttack
                }
                // 先覆盖最常见的几种；受击/死亡等后续再扩展
                _ => mir2_shared::enums::MirAction::MountStanding,
            }
        } else {
            player.action.to_mir_action()
        };

        let Some(frame) = get_player_frame(mir_action) else {
            tracing::warn!("⚠️ 未找到动画配置: {:?}, 使用默认值", mir_action);
            let fallback = player.direction as u8 as i32 * 4;
            return (fallback, 0, 0);
        };

        let dir = player.direction as u8 as i32;

        // body: DrawFrame
        let interval = frame.interval.max(1);
        let mut frame_index = if attack_state.is_some() {
            let elapsed_ms = now
                .duration_since(attack_state.unwrap().start_time)
                .as_millis() as i32;
            (elapsed_ms / interval).rem_euclid(frame.count.max(1))
        } else {
            // 旧实现：使用全局 animation_count 作为时间基准
            let animation_tick = (time_tracker.animation_count as i32) * 100 / interval;
            animation_tick.rem_euclid(frame.count.max(1))
        };
        if frame.reverse {
            frame_index = (frame.count.max(1) - 1) - frame_index;
        }
        let draw_frame = frame.start + (dir * frame.offset()) + frame_index;

        // effect: DrawWingFrame
        let effect_frame = if frame.effect_count > 0 {
            let effect_interval = frame.effect_interval.max(1);
            let mut effect_index = if attack_state.is_some() {
                let elapsed_ms = now
                    .duration_since(attack_state.unwrap().start_time)
                    .as_millis() as i32;
                (elapsed_ms / effect_interval).rem_euclid(frame.effect_count.max(1))
            } else {
                let effect_tick = (time_tracker.animation_count as i32) * 100 / effect_interval;
                effect_tick.rem_euclid(frame.effect_count.max(1))
            };
            if frame.reverse {
                effect_index = (frame.effect_count.max(1) - 1) - effect_index;
            }
            frame.effect_start + (dir * frame.effect_offset()) + effect_index
        } else {
            // 没有 effect_* 配置时，退回到身体帧（比固定 0 更接近“跟随动作帧”）
            draw_frame
        };

        (draw_frame, effect_frame, frame_index)
    }

    /// 按攻击动作帧触发玩家攻击音效（对齐 C#：在 Attack 动画的命中帧播放 Swing/坐骑攻击音效）
    pub fn update_attack_sounds(&mut self, ctx: &mut GameContext) -> GameResult {
        const TRIGGER_FRAME: i32 = 1;

        let mut triggers: Vec<(hecs::Entity, Instant, Option<i32>)> = Vec::new();

        for (entity, (attack_state, anim, _appearance, _mount_status, _mount_state, played)) in ctx
            .world
            .query::<(
                &AttackState,
                &AnimationFrame,
                &PlayerAppearance,
                Option<&MountStatus>,
                Option<&MountState>,
                Option<&AttackSoundPlayed>,
            )>()
            .iter()
        {
            if anim.action_frame_index != TRIGGER_FRAME {
                continue;
            }

            if let Some(p) = played {
                if p.attack_start_time == attack_state.start_time {
                    continue;
                }
            }

            // 复用对齐后的音效选择规则（包含坐骑随机区间、刺客/弓手分支、武器 Swing 分组）
            let sound_id = CombatSystem::choose_player_attack_sound_id(&ctx.world, entity);
            triggers.push((entity, attack_state.start_time, sound_id));
        }

        for (entity, start_time, sound_id) in triggers {
            if ctx
                .world
                .insert_one(entity, AttackSoundPlayed { attack_start_time: start_time })
                .is_err()
            {
                if let Ok(mut played) = ctx.world.get::<&mut AttackSoundPlayed>(entity) {
                    played.attack_start_time = start_time;
                }
            }

            if let Some(id) = sound_id {
                let _ = ctx.world.insert_one(
                    entity,
                    SoundTrigger::once(id.to_string(), SoundType::CharacterAction),
                );
            }
        }

        Ok(())
    }

    /// 更新怪物/NPC 的 LibrarySprite 动画帧（最小集：怪物 DefaultMonster）
    pub fn update_library_sprite_animations(&mut self, ctx: &mut GameContext) -> GameResult {
        use crate::components::{LibrarySprite, Monster, MonsterAnimState};

        for (_entity, (monster, state, spr)) in ctx
            .world
            .query_mut::<(&Monster, &MonsterAnimState, &mut LibrarySprite)>()
            .into_iter()
        {
            let action = state.action;
            let frame = crate::objects::frames::get_monster_frame(
                monster.monster_type,
                action,
                state.direction,
                monster.stage,
            )
            .or_else(|| {
                // 兜底：DefaultMonster 只有 Attack1
                get_default_monster_frame(mir2_shared::enums::MirAction::Attack1)
            });
            let Some(frame) = frame else {
                continue;
            };

            let dir = state.direction as u8 as i32;
            let interval = frame.interval.max(1);

            let elapsed_ms = Instant::now().duration_since(state.start_time).as_millis() as i32;
            let mut frame_index = (elapsed_ms / interval).rem_euclid(frame.count.max(1));
            if frame.reverse {
                frame_index = (frame.count.max(1) - 1) - frame_index;
            }

            // DrawFrame = Frame.Start + (Direction * Frame.OffSet) + FrameIndex
            spr.index = frame.start + (dir * frame.offset());
            spr.frame = frame_index;

            // quiet unused warning guard
            let _ = monster.monster_type;
        }

        Ok(())
    }

    /// 怪物 SwingSound（BaseSound+4）按攻击动作帧触发
    pub fn update_monster_swing_sounds(&mut self, ctx: &mut GameContext) -> GameResult {
        use crate::components::{AttackState, LibrarySprite, Monster, MonsterAnimState, SoundTrigger, SoundType, SwingSoundPlayed};

        // 对齐 C# 原版（Client/MirObjects/MonsterObject.cs）：
        // - 近战 Attack1/Attack2：FrameIndex == 3 时 PlaySwingSound()
        // - 远程 AttackRange1：FrameIndex == 2 时 PlaySwingSound()
        // 这里先做最小可用映射；更细分（特殊怪物/特殊动作）后续再补。
        fn swing_trigger_frame(action: crate::components::MirAction) -> Option<i32> {
            use crate::components::MirAction;
            match action {
                MirAction::Attack1
                | MirAction::Attack2
                | MirAction::Attack3
                | MirAction::Attack4
                | MirAction::MountAttack => Some(3),
                MirAction::AttackRange1 | MirAction::AttackRange2 => Some(2),
                _ => None,
            }
        }

        let mut triggers: Vec<(hecs::Entity, Instant, i32)> = Vec::new();

        for (entity, (monster, anim, attack_state, spr, played)) in ctx
            .world
            .query::<(
                &Monster,
                &MonsterAnimState,
                &AttackState,
                &LibrarySprite,
                Option<&SwingSoundPlayed>,
            )>()
            .iter()
        {
            // 对齐 C# MonsterObject.PlaySwingSound：部分怪物不播放 SwingSound
            // - DarkCaptain / EvilMir / DragonStatue: return
            let mt = monster.monster_type;
            if mt == (mir2_shared::enums::Monster::DarkCaptain as u16)
                || mt == (mir2_shared::enums::Monster::EvilMir as u16)
                || mt == (mir2_shared::enums::Monster::DragonStatue as u16)
            {
                continue;
            }

            let Some(trigger_frame) = swing_trigger_frame(anim.action) else {
                continue;
            };

            if spr.frame != trigger_frame {
                continue;
            }

            if let Some(p) = played {
                if p.attack_start_time == attack_state.start_time {
                    continue;
                }
            }

            let base = monster.monster_type as i32 * 10;
            triggers.push((entity, attack_state.start_time, base + 4));
        }

        for (entity, start_time, sound_id) in triggers {
            if ctx
                .world
                .insert_one(entity, SwingSoundPlayed { attack_start_time: start_time })
                .is_err()
            {
                if let Ok(mut played) = ctx.world.get::<&mut SwingSoundPlayed>(entity) {
                    played.attack_start_time = start_time;
                }
            }

            let _ = ctx.world.insert_one(
                entity,
                SoundTrigger::once(sound_id.to_string(), SoundType::CharacterAction),
            );
        }

        Ok(())
    }

    /// 怪物 AttackSound（BaseSound+1/+6..+9）按“动作切换瞬间”触发
    ///
    /// 对齐 C#：MonsterObject.SetAction() 进入 Attack*/AttackRange* 时立即播放对应音效。
    /// 这里不依赖怪物帧表（DefaultMonster 只有 Attack1），避免特殊怪物/帧集导致音效不触发。
    pub fn update_monster_attack_sounds(&mut self, ctx: &mut GameContext) -> GameResult {
        use crate::components::{AttackSoundPlayed, AttackState, Monster, MonsterAnimState, SoundTrigger, SoundType};
        use mir2_shared::enums::{MirAction, Monster as MonsterKind};

        fn third_attack_offset(monster_type: u16) -> Option<i32> {
            // C# MonsterObject.PlayThirdAttackSound():
            // DarkCaptain/HornedSorceror/HornedCommander 不播放
            if monster_type == (MonsterKind::DarkCaptain as u16)
                || monster_type == (MonsterKind::HornedSorceror as u16)
                || monster_type == (MonsterKind::HornedCommander as u16)
            {
                None
            } else {
                Some(7)
            }
        }

        fn fourth_attack_offset(monster_type: u16) -> Option<i32> {
            // C# MonsterObject.PlayFourthAttackSound():
            // HornedCommander 不播放；SnowWolfKing 用 +5；默认 +8
            if monster_type == (MonsterKind::HornedCommander as u16) {
                None
            } else if monster_type == (MonsterKind::SnowWolfKing as u16) {
                Some(5)
            } else {
                Some(8)
            }
        }

        fn range_offset_stage0(monster_type: u16) -> Option<i32> {
            // C# MonsterObject.PlayRangeSound()
            // 多数特例直接用 +5 / +7 / +8 / none；默认回退到 PlayAttackSound(+1)
            if monster_type == (MonsterKind::TucsonGeneral as u16) {
                return None;
            }
            if monster_type == (MonsterKind::AncientBringer as u16)
                || monster_type == (MonsterKind::SeedingsGeneral as u16)
            {
                return Some(7);
            }
            if monster_type == (MonsterKind::RestlessJar as u16) {
                return Some(8);
            }

            match MonsterKind::try_from(monster_type) {
                Ok(
                    MonsterKind::FrozenZombie
                    | MonsterKind::UndeadWolf
                    | MonsterKind::CatShaman
                    | MonsterKind::CannibalTentacles
                    | MonsterKind::SwampWarrior
                    | MonsterKind::GeneralMeowMeow
                    | MonsterKind::RhinoPriest
                    | MonsterKind::HardenRhino
                    | MonsterKind::TreeGuardian
                    | MonsterKind::OmaCannibal
                    | MonsterKind::OmaMage
                    | MonsterKind::OmaWitchDoctor
                    | MonsterKind::CreeperPlant
                    | MonsterKind::AvengingSpirit
                    | MonsterKind::AvengingWarrior
                    | MonsterKind::PeacockSpider
                    | MonsterKind::FlamingMutant
                    | MonsterKind::KingHydrax
                    | MonsterKind::DarkCaptain
                    | MonsterKind::DarkOmaKing
                    | MonsterKind::HornedMage
                    | MonsterKind::FrozenKnight
                    | MonsterKind::IcePhantom
                    | MonsterKind::WaterDragon
                    | MonsterKind::BlackTortoise
                    | MonsterKind::EvilMir
                    | MonsterKind::DragonStatue,
                ) => Some(5),
                _ => Some(1),
            }
        }

        fn range_offset_stage1(monster_type: u16) -> Option<i32> {
            // C# MonsterObject.PlaySecondRangeSound()
            if monster_type == (MonsterKind::TucsonGeneral as u16) {
                return Some(5);
            }
            if monster_type == (MonsterKind::TurtleKing as u16) {
                return None;
            }
            if monster_type == (MonsterKind::KingGuard as u16)
                || monster_type == (MonsterKind::TreeGuardian as u16)
                || monster_type == (MonsterKind::DarkCaptain as u16)
                || monster_type == (MonsterKind::HornedCommander as u16)
            {
                return Some(7);
            }
            if monster_type == (MonsterKind::AncientBringer as u16)
                || monster_type == (MonsterKind::SeedingsGeneral as u16)
            {
                return Some(8);
            }
            Some(6)
        }

        fn range_offset_stage2(monster_type: u16) -> Option<i32> {
            // C# MonsterObject.PlayThirdRangeSound():
            // TucsonGeneral 用 +7；默认 PlayThirdAttackSound()
            if monster_type == (MonsterKind::TucsonGeneral as u16) {
                return Some(7);
            }
            third_attack_offset(monster_type)
        }

        fn melee_attack_offset(monster_type: u16, stage: u8) -> Option<i32> {
            // stage 对齐网络包 attack_type：0..4
            match stage {
                1 => Some(6),
                2 => third_attack_offset(monster_type),
                3 => fourth_attack_offset(monster_type),
                4 => Some(9),
                _ => Some(1),
            }
        }

        fn range_attack_offset(monster_type: u16, stage: u8) -> Option<i32> {
            match stage {
                1 => range_offset_stage1(monster_type),
                2 => range_offset_stage2(monster_type),
                // 第 0 段：RangeSound
                _ => range_offset_stage0(monster_type),
            }
        }

        let mut triggers: Vec<(hecs::Entity, std::time::Instant, i32)> = Vec::new();

        for (entity, (monster, anim, attack_state, played)) in ctx
            .world
            .query::<(
                &Monster,
                &MonsterAnimState,
                &AttackState,
                Option<&AttackSoundPlayed>,
            )>()
            .iter()
        {
            // 仅在攻击动作期间触发
            let is_attack_action = matches!(
                anim.action,
                MirAction::Attack1
                    | MirAction::Attack2
                    | MirAction::Attack3
                    | MirAction::Attack4
                    | MirAction::Attack5
                    | MirAction::AttackRange1
                    | MirAction::AttackRange2
                    | MirAction::AttackRange3
            );
            if !is_attack_action {
                continue;
            }

            if let Some(p) = played {
                if p.attack_start_time == attack_state.start_time {
                    continue;
                }
            }

            let base = monster.monster_type as i32 * 10;
            let stage = attack_state.server_attack_type;
            let is_ranged = matches!(
                anim.action,
                MirAction::AttackRange1 | MirAction::AttackRange2 | MirAction::AttackRange3
            );

            let offset = if is_ranged {
                range_attack_offset(monster.monster_type, stage)
            } else {
                melee_attack_offset(monster.monster_type, stage)
            };

            let Some(offset) = offset else {
                continue;
            };

            triggers.push((entity, attack_state.start_time, base + offset));
        }

        for (entity, start_time, sound_id) in triggers {
            if ctx
                .world
                .insert_one(entity, AttackSoundPlayed { attack_start_time: start_time })
                .is_err()
            {
                if let Ok(mut played) = ctx.world.get::<&mut AttackSoundPlayed>(entity) {
                    played.attack_start_time = start_time;
                }
            }

            let _ = ctx.world.insert_one(
                entity,
                SoundTrigger::once(sound_id.to_string(), SoundType::CharacterAction),
            );
        }

        Ok(())
    }

    // 注意：武器帧独立逻辑暂时保留在 WeaponState/WeaponAnimation（用于未来的“挥砍特效触发帧”等）。
    // 但渲染使用的 weapon_frame 目前直接跟随 character_frame，避免与资源布局不一致导致“取不到纹理”。

    pub fn update_attack_animation(&mut self, ctx: &mut GameContext, _dt: f32) -> GameResult {
        let now = Instant::now();
        
        // 收集需要移除 AttackState 的实体
        let mut finished_attacks = Vec::new();
        
        for (entity, (attack_state, mount_state, monster)) in ctx
            .world
            .query_mut::<(&AttackState, Option<&MountState>, Option<&crate::components::Monster>)>()
            .into_iter()
        {
            let mounted = mount_state.and_then(|m| m.mount_index).is_some();
            let is_monster = monster.is_some();

            // 从 FrameSet 获取攻击动画时长
            let duration_ms = if is_monster {
                // DefaultMonster 只有 Attack1，服务器多段攻击这里只做最小对齐
                get_default_monster_frame(mir2_shared::enums::MirAction::Attack1)
                    .map(|frame| (frame.count * frame.interval) as u64)
                    .unwrap_or(600)
            } else if mounted {
                get_player_frame(mir2_shared::enums::MirAction::MountAttack)
                    .map(|frame| (frame.count * frame.interval) as u64)
                    .unwrap_or_else(|| {
                        tracing::warn!("⚠️ 未找到骑乘攻击动画配置: MountAttack, 使用默认时长");
                        600
                    })
            } else if let Some(frame) = get_player_frame(attack_state.attack_type.to_mir_action()) {
                (frame.count * frame.interval) as u64
            } else {
                // 后备：默认600ms (6帧 * 100ms)
                tracing::warn!("⚠️ 未找到攻击动画配置: {:?}, 使用默认时长", attack_state.attack_type);
                600
            };
            
            let elapsed = now.duration_since(attack_state.start_time).as_millis() as u64;
            
            if elapsed >= duration_ms {
                finished_attacks.push(entity);
                tracing::debug!(
                    "⚔️ 攻击动画完成: {:?} (耗时 {}ms)",
                    attack_state.attack_type,
                    elapsed
                );
            }
        }
        
        // 移除完成的攻击状态并恢复 Stand
        for entity in finished_attacks {
            // 移除 AttackState 组件
            let _ = ctx.world.remove_one::<AttackState>(entity);
            let _ = ctx.world.remove_one::<AttackSoundPlayed>(entity);
            let _ = ctx.world.remove_one::<crate::components::SwingSoundPlayed>(entity);
            
            // 恢复到站立状态
            if let Ok(player) = ctx.world.query_one_mut::<&mut Player>(entity) {
                player.action = PlayerAction::Stand;
                tracing::info!("✅ 攻击完成，返回站立状态");
            }

            // Monster：攻击结束回到 Standing
            if let Ok(mut s) = ctx.world.get::<&mut crate::components::MonsterAnimState>(entity) {
                s.action = crate::components::MirAction::Standing;
                s.start_time = Instant::now();
            }
        }
        
        Ok(())
    }
}

impl LogicSystem for AnimationSystem {
    fn update(&mut self, ctx: &mut GameContext, dt: f32) -> GameResult {
        // 1. 更新所有动画帧（逻辑层职责）
        self.update_animation_frames(ctx)?;

        // 2. 按攻击动作帧触发攻击音效
        self.update_attack_sounds(ctx)?;

        // 3. 推进怪物 LibrarySprite 动画帧
        self.update_library_sprite_animations(ctx)?;

        // 3.1 怪物 AttackSound（对齐 C# SetAction：动作切换瞬间）
        self.update_monster_attack_sounds(ctx)?;

        // 4. 怪物 SwingSound 按帧触发
        self.update_monster_swing_sounds(ctx)?;
        
        // 5. 检测攻击动画完成
        self.update_attack_animation(ctx, dt)?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{MirDirection, Player};

    #[test]
    fn effect_frame_matches_csharp_drawwingframe_formula_for_mount_standing() {
        let player = Player {
            direction: MirDirection::Down,
            action: PlayerAction::Stand,
        };
        let time_tracker = TimeTracker::default();

        // mounted=true 会将 Stand 映射为 MountStanding
        let (draw_frame, effect_frame, _idx) =
            AnimationSystem::calculate_frames(&player, &time_tracker, true, Instant::now(), None);
        let frame = get_player_frame(mir2_shared::enums::MirAction::MountStanding).expect("mount standing frame");

        let dir = player.direction as u8 as i32;
        let expected_draw = frame.start + dir * frame.offset();
        let expected_effect = frame.effect_start + dir * frame.effect_offset();

        assert_eq!(draw_frame, expected_draw);
        assert_eq!(effect_frame, expected_effect);
        assert!(frame.effect_count > 0);
    }
}

