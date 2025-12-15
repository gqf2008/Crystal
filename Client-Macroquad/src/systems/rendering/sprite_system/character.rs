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
use crate::components::{AnimationFrame, Camera, LibrarySprite, LocalPlayer, MountState, Player, PlayerAppearance, Position, SpriteBlendMode, TimeTracker};
use crate::game::GameResult;
use crate::objects::frames::get_player_frame;
use crate::resources::LibraryName;
use macroquad::prelude::*;

/// 精灵渲染系统

impl SpriteRenderSystem {
    // 武器特效强度（DrawBlend 的 alpha 近似值）
    // C# 原版 PlayerObject.DrawWeapon(): WeaponEffectLibrary1.DrawBlend(..., rate=0.4F)
    pub(crate) const WEAPON_EFFECT_ALPHA: f32 = 0.4;

    // 坐骑时，人物在坐骑上的上移（纯视觉，不影响碰撞/寻路）
    const MOUNT_RIDER_OFFSET_Y_PX: f32 = -24.0;

    // 坐骑时翅膀的额外上移：翅膀应更贴合“人在坐骑上更高”的观感。
    // 若你觉得还不够/太多，优先调这个常量。
    const MOUNT_WING_OFFSET_Y_PX: f32 = -24.0;

    // 坐骑时翅膀的额外水平偏移：用于修正“翅膀偏左”。
    const MOUNT_WING_OFFSET_X_PX: f32 = 12.0;

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
        mount_index: Option<usize>,
        is_local: bool,
    ) -> GameResult {
        // 诊断：外观索引（LOCAL/REMOTE 各打印一次，避免只看到远程而看不到本地）
        static APPEARANCE_LOCAL_ONCE: std::sync::OnceLock<()> = std::sync::OnceLock::new();
        static APPEARANCE_REMOTE_ONCE: std::sync::OnceLock<()> = std::sync::OnceLock::new();
        if is_local {
            let _ = APPEARANCE_LOCAL_ONCE.set(()).map(|_| {
                println!(
                    "[DIAG][SpriteRenderSystem][LOCAL] appearance: class={:?} gender={:?} hair={} armour={} weapon={} weapon_effect={} wing_effect={} mount_index={:?}",
                    appearance.class,
                    appearance.gender,
                    appearance.hair,
                    appearance.armour,
                    appearance.weapon,
                    appearance.weapon_effect,
                    appearance.wing_effect,
                    mount_index
                );
            });
        } else {
            let _ = APPEARANCE_REMOTE_ONCE.set(()).map(|_| {
                println!(
                    "[DIAG][SpriteRenderSystem][REMOTE] appearance: class={:?} gender={:?} hair={} armour={} weapon={} weapon_effect={} wing_effect={} mount_index={:?}",
                    appearance.class,
                    appearance.gender,
                    appearance.hair,
                    appearance.armour,
                    appearance.weapon,
                    appearance.weapon_effect,
                    appearance.wing_effect,
                    mount_index
                );
            });
        }

        let base_frame = anim_frame.character_frame;
        let weapon_frame = anim_frame.weapon_frame;
        let effect_frame = anim_frame.effect_frame;

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

        // 坐骑（由组件驱动）
        let mounted = mount_index.is_some();
        let mut mount_drawn = false;
        let actor_pos;

        if mounted {
            let mount_lib = LibraryName::Mounts(mount_index.unwrap_or(0));

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
                // 注意：骑乘时人物身体会切换到 Mount* 帧，但武器资源未必包含同一套 Mount* 帧索引。
                // 为了更稳定地显示武器，这里保持使用 AnimationSystem 计算的 weapon_frame（与非骑乘一致）。
                rider_weapon_frame = weapon_frame;
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

        // 翅膀：本项目使用 CHumEffect 作为人物特效库。
        // 关键点：骑乘时人物身体会切换到 Mount* 帧段(416+)，但 CHumEffect 往往按普通动作帧布局。
        // 所以这里把 Mount* 帧映射回普通 Standing/Walking/Running 帧段，避免“抽到奇怪帧导致位置/效果怪”。
        if appearance.wing_effect > 0 {
            // C# 原版 SetLibraries(): WingLibrary = Libraries.CHumEffect[WingEffect - 1]
            let wing_lib_index = (appearance.wing_effect - 1).max(0) as usize;
            let wing_lib = LibraryName::CHumEffect(wing_lib_index);

            // 对齐 C# PlayerObject.SetLibraries():
            // - WingOffset: 男=0，女=840（altAnim 暂未实现，先走最常见路径）
            // 若不加这个偏移，女号会从翅膀库里取到“另一套帧段”，表现为错位/叠影。
            let wing_offset = if appearance.gender == crate::components::MirGender::Male {
                0
            } else {
                840
            };

            // 对齐 C# PlayerObject.DrawWings():
            // - DrawWingFrame 在逻辑层 AnimationSystem 里按 Frame.EffectStart/EffectOffSet/EffectInterval 统一计算。
            // 渲染层只读取 effect_frame，避免“渲染层再算一遍”导致方向步进/interval 不一致。
            let wing_primary = effect_frame;

            // 翅膀位置：
            // - 若人物已切换到 Mount* 帧段（rider_uses_mount_frames=true），库内 offset 已与坐骑对齐，
            //   再叠加人工偏移容易导致“翅膀飘/歪”。
            // - 只有在未能切到 Mount* 帧段、使用旧的 rider 偏移兜底时，才需要额外修正翅膀位置。
            let wing_pos = if mounted && mount_drawn && !rider_uses_mount_frames {
                Position::new(
                    actor_pos.x + Self::MOUNT_WING_OFFSET_X_PX,
                    actor_pos.y + Self::MOUNT_WING_OFFSET_Y_PX,
                )
            } else {
                actor_pos
            };
            let candidates = [wing_primary + wing_offset, wing_primary, 0 + wing_offset, 0];
            let mut wing_drawn = false;
            let mut wing_frame_used: Option<i32> = None;
            for f in candidates {
                // 翅膀/人物特效通常是发光效果，用 additive 混合更接近原版观感。
                if draw_layer_additive(add_blend_material, wing_lib, f, &wing_pos, tint) {
                    wing_drawn = true;
                    wing_frame_used = Some(f);
                    break;
                }
            }

            // 兜底：如果候选帧都不存在，首次扫描前 64 帧找一个“能画出来”的帧。
            if !wing_drawn {
                static WING_PROBE_ONCE: std::sync::OnceLock<Option<i32>> = std::sync::OnceLock::new();
                let probe = WING_PROBE_ONCE.get_or_init(|| {
                    // 先按当前性别的帧段（wing_offset）扫描，避免女号落到男号帧段。
                    for i in 0..64 {
                        let idx = (i as i32 + wing_offset) as usize;
                        if wing_lib.get_texture(idx).and_then(|info| info.image).is_some() {
                            return Some(i as i32 + wing_offset);
                        }
                    }
                    // 再兜底扫描低位帧段
                    for i in 0..64 {
                        if wing_lib.get_texture(i).and_then(|info| info.image).is_some() {
                            return Some(i as i32);
                        }
                    }
                    None
                });

                if let Some(f) = *probe {
                    if draw_layer_additive(add_blend_material, wing_lib, f, &wing_pos, tint) {
                        wing_drawn = true;
                        wing_frame_used = Some(f);
                    }
                }
            }

            static WING_DRAW_LOCAL_ONCE: std::sync::OnceLock<()> = std::sync::OnceLock::new();
            static WING_DRAW_REMOTE_ONCE: std::sync::OnceLock<()> = std::sync::OnceLock::new();
            if is_local {
                let _ = WING_DRAW_LOCAL_ONCE.set(()).map(|_| {
                    println!(
                        "[DIAG][SpriteRenderSystem][LOCAL] wing_drawn={} wing_effect={} frame_used={:?} (using CHumEffect/{:02}) mounted={} mount_drawn={}",
                        wing_drawn,
                        appearance.wing_effect,
                        wing_frame_used,
                        appearance.wing_effect,
                        mounted,
                        mount_drawn
                    );
                });
            } else {
                let _ = WING_DRAW_REMOTE_ONCE.set(()).map(|_| {
                    println!(
                        "[DIAG][SpriteRenderSystem][REMOTE] wing_drawn={} wing_effect={} frame_used={:?} (using CHumEffect/{:02}) mounted={} mount_drawn={}",
                        wing_drawn,
                        appearance.wing_effect,
                        wing_frame_used,
                        appearance.wing_effect,
                        mounted,
                        mount_drawn
                    );
                });
            }
            drew_any |= wing_drawn;
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
                let weapon_lib = weapon_library(weapon_index);
                let candidates = [
                    rider_base_frame + weapon_offset,
                    rider_weapon_frame + weapon_offset,
                    rider_weapon_frame,
                    rider_base_frame,
                    0 + weapon_offset,
                    0,
                ];
                drew_any |= Self::draw_weapon_with_effect(
                    &draw_layer,
                    &draw_layer_additive,
                    add_blend_material,
                    &actor_pos,
                    tint,
                    weapon_lib,
                    weapon_index,
                    weapon_effect_index_opt,
                    candidates,
                );
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
                let weapon_lib = weapon_library(weapon_index);
                let candidates = [
                    rider_base_frame + weapon_offset,
                    rider_weapon_frame + weapon_offset,
                    rider_weapon_frame,
                    rider_base_frame,
                    0 + weapon_offset,
                    0,
                ];
                drew_any |= Self::draw_weapon_with_effect(
                    &draw_layer,
                    &draw_layer_additive,
                    add_blend_material,
                    &actor_pos,
                    tint,
                    weapon_lib,
                    weapon_index,
                    weapon_effect_index_opt,
                    candidates,
                );
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
        alpha: f32,
        local_only: bool,
    ) -> GameResult {
        // 诊断：确认 PostFront 的 ghost 绘制路径确实进入了 draw_character。
        // 只打印一次，避免刷屏。
        static DRAW_CHAR_DIAG_ONCE: std::sync::OnceLock<()> = std::sync::OnceLock::new();

        // 相机（用于裁剪；真正的坐标变换由 GameScene 的 macroquad Camera2D 完成）
        let (cam_x, cam_y, cam_zoom, sw, sh) = world
            .query::<(&Camera, &Position)>()
            .iter()
            .next()
            .map(|(_, (c, p))| (p.x, p.y, c.zoom, c.screen_width, c.screen_height))
            .unwrap_or((0.0, 0.0, 1.0, screen_width(), screen_height()));

        let zoom = cam_zoom.max(0.0001);
        let half_w = sw.max(1.0) * 0.5 / zoom;
        let half_h = sh.max(1.0) * 0.5 / zoom;
        let view_min_x = cam_x - half_w;
        let view_max_x = cam_x + half_w;
        let view_min_y = cam_y - half_h;
        let view_max_y = cam_y + half_h;
        const CULL_MARGIN: f32 = 200.0;

        let stage = world
            .query::<&crate::components::RenderPass>()
            .iter()
            .next()
            .map(|(_, p)| p.stage)
            .unwrap_or(crate::components::RenderStage::Normal);

        if stage == crate::components::RenderStage::PostFront && local_only {
            let local_player_count = world
                .query::<&crate::components::LocalPlayer>()
                .iter()
                .count();
            let _ = DRAW_CHAR_DIAG_ONCE.set(()).map(|_| {
                println!(
                    "[DIAG][SpriteRenderSystem] draw_character(PostFront, local_only): locals={} cam=({:.1},{:.1}) zoom={:.3} view=({:.1},{:.1})-({:.1},{:.1})",
                    local_player_count,
                    cam_x,
                    cam_y,
                    cam_zoom,
                    view_min_x,
                    view_min_y,
                    view_max_x,
                    view_max_y
                );
            });
        }

        let animation_count = world
            .query::<&TimeTracker>()
            .iter()
            .next()
            .map(|(_, tt)| tt.animation_count)
            .unwrap_or(0);

        // 收集并排序可渲染对象（玩家/怪物/NPC）
        #[derive(Clone)]
        enum Renderable {
            Player {
                entity: hecs::Entity,
                player: Player,
                pos: Position,
                appearance: PlayerAppearance,
                anim_frame: AnimationFrame,
            },
            LibrarySprite {
                spr: LibrarySprite,
                pos: Position,
                kind_order: i32,
            },
        }

        fn in_view(pos: &Position, min_x: f32, min_y: f32, max_x: f32, max_y: f32, margin: f32) -> bool {
            pos.x >= min_x - margin && pos.x <= max_x + margin && pos.y >= min_y - margin && pos.y <= max_y + margin
        }

        let mut renderables: Vec<(f32, i32, Renderable)> = Vec::new();

        // Players
        if local_only {
            for (entity, (_local, player, pos, appearance, anim_frame)) in world
                .query::<(&LocalPlayer, &Player, &Position, &PlayerAppearance, &AnimationFrame)>()
                .iter()
            {
                if !in_view(pos, view_min_x, view_min_y, view_max_x, view_max_y, CULL_MARGIN) {
                    continue;
                }
                // kind order: NPC(0) < Monster(1) < Player(2)
                renderables.push((pos.y, 2, Renderable::Player {
                    entity,
                    player: player.clone(),
                    pos: *pos,
                    appearance: appearance.clone(),
                    anim_frame: *anim_frame,
                }));
            }
        } else {
            for (entity, (player, pos, appearance, anim_frame)) in world
                .query::<(&Player, &Position, &PlayerAppearance, &AnimationFrame)>()
                .iter()
            {
                if !in_view(pos, view_min_x, view_min_y, view_max_x, view_max_y, CULL_MARGIN) {
                    continue;
                }
                renderables.push((pos.y, 2, Renderable::Player {
                    entity,
                    player: player.clone(),
                    pos: *pos,
                    appearance: appearance.clone(),
                    anim_frame: *anim_frame,
                }));
            }
        }

        // NPC/Monster：当前先用占位绘制（后续接入真实库映射/动画组件）
        if !local_only {
            use crate::components::{NetworkObjectType, NetworkSync};
            for (_entity, (sync, spr, pos)) in world.query::<(&NetworkSync, &LibrarySprite, &Position)>().iter() {
                if sync.object_type != NetworkObjectType::NPC && sync.object_type != NetworkObjectType::Monster {
                    continue;
                }
                if !matches!(spr.blend_mode, SpriteBlendMode::Alpha) {
                    continue;
                }
                if !in_view(pos, view_min_x, view_min_y, view_max_x, view_max_y, CULL_MARGIN) {
                    continue;
                }

                // kind order: NPC(0) < Monster(1) < Player(2)
                let kind_order = if sync.object_type == NetworkObjectType::NPC { 0 } else { 1 };
                renderables.push((
                    pos.y,
                    kind_order,
                    Renderable::LibrarySprite {
                        spr: *spr,
                        pos: *pos,
                        kind_order,
                    },
                ));
            }
        }

        // 深度排序：先按 y（越靠下越后绘制），再按类型优先级
        renderables.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.cmp(&b.1))
        });

        // 渲染
        for (_y, _kind, item) in renderables {
            match item {
                Renderable::Player { entity, player, pos, appearance, anim_frame } => {
                    let mount_index = world
                        .get::<&MountState>(entity)
                        .ok()
                        .and_then(|m| m.mount_index);
                    let is_local = world.get::<&LocalPlayer>(entity).is_ok();
                    Self::render_character(
                        &player,
                        &pos,
                        &appearance,
                        &anim_frame,
                        animation_count,
                        alpha,
                        add_blend_material,
                        mount_index,
                        is_local,
                    )?;
                }
                Renderable::LibrarySprite { spr, pos, kind_order: _ } => {
                    let tint = Color::new(1.0, 1.0, 1.0, alpha.clamp(0.0, 1.0));
                    let Some(info) = spr.library.get_texture(spr.texture_index()) else {
                        continue;
                    };
                    let Some(tex) = info.image else {
                        continue;
                    };

                    let draw_x = pos.x + info.offset_x as f32;
                    let draw_y = pos.y + info.offset_y as f32;
                    draw_texture_ex(&tex, draw_x, draw_y, tint, DrawTextureParams { ..Default::default() });
                }
            }
        }

        Ok(())
    }
}
