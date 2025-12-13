// ============================================================================
// Character Render System - 角色渲染系统
// ============================================================================
//
// **优先级**: 610 (在地图渲染后，调试系统前)
//
// **职责**:
// - 渲染所有玩家角色（本地玩家和其他玩家）
// - 处理角色动画帧（站立、行走、跑步）
// - 处理多层渲染（身体、武器、翅膀等）
//
// **C# 参考**:
// - PlayerObject.DrawBody() - 绘制角色身体
// - PlayerObject.DrawWeapon() - 绘制武器
// - CHumEffect[class][gender] - 角色库索引
//
// **渲染流程**:
// 1. 查询所有 (Player, Position, PlayerAppearance)
// 2. 按 Y 坐标排序（深度排序）
// 3. 计算动画帧索引
// 4. 从 CHumEffect 库加载精灵图
// 5. 应用相机变换绘制到屏幕
//
// ============================================================================

use super::SpriteRenderSystem;
use crate::components::{AnimationFrame, Player, PlayerAppearance, Position, RenderPass, TimeTracker};
use crate::game::GameResult;
use crate::objects::frames::get_player_frame;
use crate::resources::LibraryName;
use macroquad::prelude::*;

/// 精灵渲染系统

impl SpriteRenderSystem {
    // 武器特效强度（DrawBlend 的 alpha 近似值）
    const WEAPON_EFFECT_ALPHA: f32 = 0.4;

    // 坐骑时，人物在坐骑上的上移（纯视觉，不影响碰撞/寻路）
    const MOUNT_RIDER_OFFSET_Y_PX: f32 = -24.0;

    /// 渲染单个角色
    /// 渲染单个角色（macroquad 版本）
    ///
    /// **重构说明**: 不再自己计算帧索引，改为从 AnimationFrame 组件读取
    /// AnimationSystem 负责计算并更新帧索引，渲染系统只负责读取和绘制
    #[allow(dead_code)]
    fn render_character(
        player: &Player,
        pos: &Position,
        appearance: &PlayerAppearance,
        anim_frame: &AnimationFrame,
        animation_count: i32,
        alpha: f32,
        add_blend_material: &Material,
    ) -> GameResult {
        let base_frame = anim_frame.character_frame;
        let weapon_frame = anim_frame.weapon_frame;

        let tint = Color::new(1.0, 1.0, 1.0, alpha.clamp(0.0, 1.0));

        // 性别偏移（C# HairOffSet/ArmourOffSet/WeaponOffSet）
        let body_hair_offset = if appearance.gender == crate::components::MirGender::Male {
            0
        } else {
            808
        };
        let weapon_offset = if appearance.gender == crate::components::MirGender::Male {
            0
        } else {
            416
        };

        let armour_index = appearance.armour.max(0) as usize;
        let hair_index = appearance.hair as usize;
        let weapon_index_opt = if appearance.weapon >= 0 {
            Some(appearance.weapon as usize)
        } else {
            None
        };
        let weapon_effect_index_opt = if appearance.weapon_effect > 0 {
            Some(appearance.weapon_effect as usize)
        } else {
            None
        };

        let armour_library = match appearance.class {
            crate::components::MirClass::Assassin => LibraryName::AArmours(armour_index),
            crate::components::MirClass::Archer => LibraryName::ARArmours(armour_index),
            _ => LibraryName::CArmours(armour_index),
        };
        let hair_library = match appearance.class {
            crate::components::MirClass::Assassin => LibraryName::AHair(hair_index),
            crate::components::MirClass::Archer => LibraryName::ARHair(hair_index),
            _ => LibraryName::CHair(hair_index),
        };
        let weapon_library = |idx: usize| match appearance.class {
            crate::components::MirClass::Archer => LibraryName::ARWeapons(idx),
            _ => LibraryName::CWeapons(idx),
        };

        let mut drew_any = false;

        let draw_layer = |lib: LibraryName, frame: i32, pos: &Position, tint: Color| -> bool {
            let frame_index = frame.max(0) as usize;
            let Some(info) = lib.get_texture(frame_index) else {
                return false;
            };
            let Some(tex) = info.image else {
                return false;
            };
            let draw_x = pos.x + info.offset_x as f32;
            let draw_y = pos.y + info.offset_y as f32;
            draw_texture_ex(
                &tex,
                draw_x,
                draw_y,
                tint,
                DrawTextureParams { ..Default::default() },
            );
            true
        };

        let draw_layer_additive = |material: &Material,
                                  lib: LibraryName,
                                  frame: i32,
                                  pos: &Position,
                                  tint: Color|
         -> bool {
            let frame_index = frame.max(0) as usize;
            let Some(info) = lib.get_texture(frame_index) else {
                return false;
            };
            let Some(tex) = info.image else {
                return false;
            };
            let draw_x = pos.x + info.offset_x as f32;
            let draw_y = pos.y + info.offset_y as f32;

            gl_use_material(material);
            draw_texture_ex(
                &tex,
                draw_x,
                draw_y,
                tint,
                DrawTextureParams { ..Default::default() },
            );
            gl_use_default_material();

            true
        };

        // 坐骑（当前沿用 GameScene 的 MVP 逻辑：用于视觉展示）
        const DEFAULT_MOUNT_INDEX: usize = 0;
        let mounted = true;
        let mut mount_drawn = false;
        let actor_pos;

        if mounted {
            let mount_lib = LibraryName::Mounts(DEFAULT_MOUNT_INDEX);

            // 当前 Data/Mount 资源表现为与 MirDirection 同序（Up 开始顺时针）
            let dir = player.direction as u8 as i32;
            let mount_dir_index = dir.rem_euclid(8);

            // Stand: 4 帧/方向  => base 0
            // Walk : 8 帧/方向  => base 8*4 = 32
            // Run  : 6 帧/方向  => base 32 + 8*8 = 96
            let (frames_per_dir, interval_ms, action_base) = match player.action {
                crate::components::PlayerAction::Walk => (8, 100, 8 * 4),
                crate::components::PlayerAction::Run => (6, 100, 8 * 4 + 8 * 8),
                _ => (4, 500, 0),
            };

            let animation_tick = animation_count * 100 / interval_ms;
            let current_frame = animation_tick % frames_per_dir;
            let primary_index = action_base + mount_dir_index * frames_per_dir + current_frame;

            let candidates = [
                primary_index,
                0 + mount_dir_index * 4 + (current_frame % 4),
                mount_dir_index * frames_per_dir + current_frame,
                current_frame,
                0,
                1,
                2,
            ];

            for frame_index in candidates {
                if draw_layer(mount_lib, frame_index, pos, tint) {
                    mount_drawn = true;
                    drew_any = true;
                    break;
                }
            }
        }

        // 骑乘时，人物应使用 Mount* 动作帧（否则会看到“人在坐骑上还在跑路/跑步姿势”）
        // Mount* 人物帧本身已经包含与坐骑对齐的绘制偏移。
        let mut rider_base_frame = base_frame;
        let mut rider_weapon_frame = weapon_frame;
        let mut rider_uses_mount_frames = false;
        if mounted && mount_drawn {
            let mount_action = match player.action {
                crate::components::PlayerAction::Walk => mir2_shared::enums::MirAction::MountWalking,
                crate::components::PlayerAction::Run => mir2_shared::enums::MirAction::MountRunning,
                _ => mir2_shared::enums::MirAction::MountStanding,
            };

            if let Some(frame) = get_player_frame(mount_action) {
                let dir = player.direction as u8 as i32;
                let interval = frame.interval.max(1);
                let count = frame.count.max(1);
                let tick = animation_count * 100 / interval;
                let current = tick % count;

                rider_base_frame = frame.start + (dir * frame.count) + current;
                rider_weapon_frame = rider_base_frame;
                rider_uses_mount_frames = true;
            }
        }

        // 只有真的画出了坐骑，才对人物位置做骑乘修正。
        // 若使用 Mount* 人物帧，不再额外上移；否则用旧偏移做兜底对齐。
        actor_pos = if mounted && mount_drawn {
            if rider_uses_mount_frames {
                *pos
            } else {
                Position::new(pos.x, pos.y + Self::MOUNT_RIDER_OFFSET_Y_PX)
            }
        } else {
            *pos
        };

        // 翅膀（简单实现：优先用 rider_base_frame；取不到就回退到 0 帧）
        if appearance.wing_effect > 0 {
            let wing_lib = LibraryName::Wings(appearance.wing_effect as usize);
            if !draw_layer(wing_lib, rider_base_frame, &actor_pos, tint) {
                drew_any |= draw_layer(wing_lib, 0, &actor_pos, tint);
            } else {
                drew_any = true;
            }
        }

        // 武器层前后关系（参考原版）：右侧/下方方向武器在前，其余方向武器在后
        let weapon_front = matches!(
            player.direction,
            crate::components::MirDirection::UpRight
                | crate::components::MirDirection::Right
                | crate::components::MirDirection::DownRight
                | crate::components::MirDirection::Down
        );

        // weapon behind
        if !weapon_front {
            if let Some(weapon_index) = weapon_index_opt {
                drew_any |= draw_layer(
                    weapon_library(weapon_index),
                    rider_weapon_frame + weapon_offset,
                    &actor_pos,
                    tint,
                );

                if let Some(effect_index) = weapon_effect_index_opt {
                    let effect_tint = Color::new(
                        tint.r,
                        tint.g,
                        tint.b,
                        (tint.a * Self::WEAPON_EFFECT_ALPHA).clamp(0.0, 1.0),
                    );
                    drew_any |= draw_layer_additive(
                        add_blend_material,
                        LibraryName::CWeaponEffect(effect_index),
                        rider_weapon_frame + weapon_offset,
                        &actor_pos,
                        effect_tint,
                    );
                }
            }
        }

        // body
        drew_any |= draw_layer(
            armour_library,
            rider_base_frame + body_hair_offset,
            &actor_pos,
            tint,
        );

        // hair
        drew_any |= draw_layer(
            hair_library,
            rider_base_frame + body_hair_offset,
            &actor_pos,
            tint,
        );

        // weapon front
        if weapon_front {
            if let Some(weapon_index) = weapon_index_opt {
                drew_any |= draw_layer(
                    weapon_library(weapon_index),
                    rider_weapon_frame + weapon_offset,
                    &actor_pos,
                    tint,
                );

                if let Some(effect_index) = weapon_effect_index_opt {
                    let effect_tint = Color::new(
                        tint.r,
                        tint.g,
                        tint.b,
                        (tint.a * Self::WEAPON_EFFECT_ALPHA).clamp(0.0, 1.0),
                    );
                    drew_any |= draw_layer_additive(
                        add_blend_material,
                        LibraryName::CWeaponEffect(effect_index),
                        rider_weapon_frame + weapon_offset,
                        &actor_pos,
                        effect_tint,
                    );
                }
            }
        }

        // 回退：红点
        if !drew_any {
            draw_circle(pos.x, pos.y, 6.0, Color::new(1.0, 0.0, 0.0, tint.a));
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub(super) fn draw_character(
        &self,
        world: &hecs::World,
        add_blend_material: &Material,
    ) -> GameResult {
        // tracing::info!("👤 CharacterRenderSystem::draw() 开始");
        // 获取相机变换
        let Some((_cam_x, _cam_y, _zoom)) = Self::get_camera_transform(world) else {
            // tracing::info!("⏭️  CharacterRenderSystem: 没有相机，跳过渲染");
            return Ok(());
        };

        let animation_count = world
            .query::<&TimeTracker>()
            .iter()
            .next()
            .map(|(_, tt)| tt.animation_count)
            .unwrap_or(0);

        let alpha = world
            .query::<&RenderPass>()
            .iter()
            .next()
            .map(|(_, pass)| pass.alpha)
            .unwrap_or(1.0);

        // 收集并排序角色（现在包含 AnimationFrame）
        let mut characters_to_render = Vec::new();

        for (_entity, (player, pos, appearance, anim_frame)) in world
            .query::<(&Player, &Position, &PlayerAppearance, &AnimationFrame)>()
            .iter()
        {
            characters_to_render.push((
                player.clone(),
                pos.clone(),
                appearance.clone(),
                anim_frame.clone(),
            ));
        }

        // 按 Y 坐标排序（实现深度排序）
        characters_to_render.sort_by(|a, b| {
            a.1.y
                .partial_cmp(&b.1.y)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // 渲染所有角色
        for (player, pos, appearance, anim_frame) in characters_to_render {
            Self::render_character(
                &player,
                &pos,
                &appearance,
                &anim_frame,
                animation_count,
                alpha,
                add_blend_material,
            )?;
        }

        Ok(())
    }
}
