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
use crate::components::{AnimationFrame, Camera, Health, LibrarySprite, LocalPlayer, Monster, MountState, OtherPlayer, Player, PlayerAppearance, Position, SpriteBlendMode, TimeTracker};
use crate::game::GameResult;
use crate::objects::frames::get_player_frame;
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use macroquad::prelude::*;

/// 精灵渲染系统
impl SpriteRenderSystem {
    // 武器特效强度（DrawBlend 的 alpha 近似值）
    // C# 原版 PlayerObject.DrawWeapon(): WeaponEffectLibrary1.DrawBlend(..., rate=0.4F)
    pub(crate) const WEAPON_EFFECT_ALPHA: f32 = 0.4;

    // 坐骑时，人物在坐骑上的上移（纯视觉，不影响碰撞/寻路）
    const MOUNT_RIDER_OFFSET_Y_PX: f32 = -24.0;

    // 坐骑时翅膀的额外上移：翅膀应更贴合"人在坐骑上更高"的观感。
    // 若你觉得还不够/太多，优先调这个常量。
    const MOUNT_WING_OFFSET_Y_PX: f32 = -24.0;

    // 坐骑时翅膀的额外水平偏移：用于修正"翅膀偏左"。
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
        if Self::sprite_diag_enabled() {
            if is_local {
                let _ = APPEARANCE_LOCAL_ONCE.set(()).map(|_| {
                    println!(
                        "[DIAG][SpriteRenderSystem][LOCAL] appearance: class={:?} gender={:?} hair={} armour={} weapon={} weapon_effect={} wing_effect={} mount_index={:?} alpha={:.2}",
                        appearance.class,
                        appearance.gender,
                        appearance.hair,
                        appearance.armour,
                        appearance.weapon,
                        appearance.weapon_effect,
                        appearance.wing_effect,
                        mount_index,
                        alpha
                    );
                });
            } else {
                let _ = APPEARANCE_REMOTE_ONCE.set(()).map(|_| {
                    println!(
                        "[DIAG][SpriteRenderSystem][REMOTE] appearance: class={:?} gender={:?} hair={} armour={} weapon={} weapon_effect={} wing_effect={} mount_index={:?} alpha={:.2}",
                        appearance.class,
                        appearance.gender,
                        appearance.hair,
                        appearance.armour,
                        appearance.weapon,
                        appearance.weapon_effect,
                        appearance.wing_effect,
                        mount_index,
                        alpha
                    );
                });
            }
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

        // 防御性检查：armour 索引越界时回退到 0（CArmours 一般有 59 个库，索引 0-58）
        // 这里使用保守上限 58，避免越界导致身体层完全不绘制。
        const MAX_ARMOUR_INDEX: usize = 58;
        let raw_armour_index = appearance.armour.max(0) as usize;
        let armour_index = if raw_armour_index > MAX_ARMOUR_INDEX {
            static ARMOUR_OOB_ONCE: std::sync::OnceLock<()> = std::sync::OnceLock::new();
            let _ = ARMOUR_OOB_ONCE.set(()).map(|_| {
                eprintln!(
                    "[WARN][SpriteRenderSystem] armour index {} out of bounds (max={}), fallback to 0",
                    raw_armour_index, MAX_ARMOUR_INDEX
                );
            });
            0
        } else {
            raw_armour_index
        };

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

        let layer_has_texture = |lib: LibraryName, frame: i32| -> bool {
            let frame_index = frame.max(0) as usize;
            lib.get_texture(frame_index)
                .and_then(|info| info.image)
                .is_some()
        };
        
        // 检查帧是否存在且尺寸合理（用于骑乘帧检测，避免把小特效图当成身体帧）
        let layer_has_valid_body_texture = |lib: LibraryName, frame: i32| -> bool {
            const MIN_BODY_SIZE: i16 = 40; // 身体帧至少应该 40x40 像素
            let frame_index = frame.max(0) as usize;
            lib.get_texture(frame_index)
                .map(|info| {
                    info.image.is_some() && info.width >= MIN_BODY_SIZE && info.height >= MIN_BODY_SIZE
                })
                .unwrap_or(false)
        };

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
                (mount_dir_index * 4) + (current_frame % 4),
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

        // 骑乘时，人物应使用 Mount* 动作帧（否则会看到"人在坐骑上还在跑路/跑步姿势"）
        // Mount* 人物帧本身已经包含与坐骑对齐的绘制偏移。
        let mut rider_base_frame = base_frame;
        let mut rider_weapon_frame = weapon_frame;
        let mut rider_uses_mount_frames = false;
        if mounted {
            let mount_action = match player.action {
                crate::components::PlayerAction::Walk => mir2_shared::enums::MirAction::MountWalking,
                crate::components::PlayerAction::Run => mir2_shared::enums::MirAction::MountRunning,
                crate::components::PlayerAction::Attack1
                | crate::components::PlayerAction::Attack2
                | crate::components::PlayerAction::Attack3 => mir2_shared::enums::MirAction::MountAttack,
                _ => mir2_shared::enums::MirAction::MountStanding,
            };

            let normal_action = match player.action {
                crate::components::PlayerAction::Walk => mir2_shared::enums::MirAction::Walking,
                crate::components::PlayerAction::Run => mir2_shared::enums::MirAction::Running,
                crate::components::PlayerAction::Attack1
                | crate::components::PlayerAction::Attack2
                | crate::components::PlayerAction::Attack3 => mir2_shared::enums::MirAction::Attack1,
                _ => mir2_shared::enums::MirAction::Standing,
            };

            // 关键：不要用"本帧是否成功画出坐骑(mount_drawn)"来决定人物是否骑乘。
            // mount_drawn 可能因资源缺帧而抖动，导致人物一会儿上、一会儿下。
            // 这里用一个"稳定采样帧(当前方向 idx=0)"来判断该 armour 是否真的有 Mount* 身体帧段；
            // 若没有，则整段动作都回退到普通帧，避免随着 current 帧索引变化而来回切换。
            let dir = player.direction as u8 as i32;

            // 默认：先尝试 Mount* 人物帧
            let mut mount_base: Option<i32> = None;
            let mut mount_sample_ok = false;
            if let Some(frame) = get_player_frame(mount_action) {
                let interval = frame.interval.max(1);
                let count = frame.count.max(1);
                let tick = animation_count * 100 / interval;
                let current = tick % count;

                let base = frame.start + (dir * frame.offset()) + current;
                let sample_base = frame.start; // dir=0, idx=0 for stable detection
                mount_base = Some(base);
                mount_sample_ok = layer_has_valid_body_texture(armour_library, sample_base + body_hair_offset);
            }

            if let Some(base) = mount_base {
                if mount_sample_ok {
                    rider_base_frame = base;
                    rider_weapon_frame = weapon_frame;
                    rider_uses_mount_frames = true;
                } else {
                    // 回退到普通动作帧（并选一个能画出来的身体帧，避免"有坐骑但没身体/外观"）
                    let mut fallback_base = base_frame;
                    if let Some(frame) = get_player_frame(normal_action) {
                        let interval = frame.interval.max(1);
                        let count = frame.count.max(1);
                        let tick = animation_count * 100 / interval;
                        let current = tick % count;
                        fallback_base = frame.start + (dir * frame.offset()) + current;
                    }

                    let candidates = [fallback_base, 0, 1, 2, base_frame];
                    let mut chosen: Option<i32> = None;
                    for b in candidates {
                        if layer_has_texture(armour_library, b + body_hair_offset) {
                            chosen = Some(b);
                            break;
                        }
                    }
                    rider_base_frame = chosen.unwrap_or(fallback_base);

                    rider_weapon_frame = weapon_frame;
                    rider_uses_mount_frames = false;

                    static MOUNT_ARMOUR_MISSING_ONCE: std::sync::OnceLock<()> = std::sync::OnceLock::new();
                    let _ = MOUNT_ARMOUR_MISSING_ONCE.set(()).map(|_| {
                        eprintln!(
                            "[WARN][SpriteRenderSystem] mounted armour mount-frames unavailable: armour_lib={:?} mount_sample_ok={} chosen_base={:?} (fallback to normal frames)",
                            armour_library,
                            mount_sample_ok,
                            chosen
                        );
                    });
                }
            }
        }

        // 人物位置的骑乘修正必须仅依赖"是否骑乘(mounted)"，不能依赖 mount_drawn。
        // 否则坐骑资源缺帧时会导致人物一会儿上、一会儿下。
        // 若使用 Mount* 人物帧，不再额外上移；否则用旧偏移做兜底对齐。
        let actor_pos = if mounted {
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
        // 所以这里把 Mount* 帧映射回普通 Standing/Walking/Running 帧段，避免"抽到奇怪帧导致位置/效果怪"。
        if appearance.wing_effect > 0 {
            // C# 原版 SetLibraries(): WingLibrary = Libraries.CHumEffect[WingEffect - 1]
            let wing_lib_index = (appearance.wing_effect - 1) as usize;
            let wing_lib = LibraryName::CHumEffect(wing_lib_index);

            // 对齐 C# PlayerObject.SetLibraries():
            // - WingOffset: 男=0，女=840（altAnim 暂未实现，先走最常见路径）
            // 若不加这个偏移，女号会从翅膀库里取到"另一套帧段"，表现为错位/叠影。
            let wing_offset = if appearance.gender == crate::components::MirGender::Male {
                0
            } else {
                840
            };

            // 对齐 C# PlayerObject.DrawWings():
            // 骑乘时 AnimationSystem 使用 MountStanding 的 effect_start（如 448），但 CHumEffect 库的翅膀帧
            // 通常在 0-200 范围内。所以骑乘时需要用普通 Standing 的 effect 配置重新计算翅膀帧。
            let wing_primary = if mounted {
                // 骑乘时：用普通 Standing 的 effect 配置
                let normal_action = match player.action {
                    crate::components::PlayerAction::Walk => mir2_shared::enums::MirAction::Walking,
                    crate::components::PlayerAction::Run => mir2_shared::enums::MirAction::Running,
                    crate::components::PlayerAction::Attack1
                    | crate::components::PlayerAction::Attack2
                    | crate::components::PlayerAction::Attack3 => mir2_shared::enums::MirAction::Attack1,
                    _ => mir2_shared::enums::MirAction::Standing,
                };
                
                if let Some(frame) = get_player_frame(normal_action) {
                    if frame.effect_count > 0 {
                        let dir = player.direction as u8 as i32;
                        let effect_interval = frame.effect_interval.max(1);
                        let ecount = frame.effect_count.max(1);
                        let effect_tick = (animation_count * 100) / effect_interval;
                        let effect_index = effect_tick.rem_euclid(ecount);
                        frame.effect_start + (dir * frame.effect_offset()) + effect_index
                    } else {
                        effect_frame // 没有 effect 配置，使用原值
                    }
                } else {
                    effect_frame
                }
            } else {
                effect_frame // 非骑乘时直接使用 AnimationSystem 计算的值
            };

            // 翅膀位置：
            // - 若人物已切换到 Mount* 帧段（rider_uses_mount_frames=true），库内 offset 已与坐骑对齐，
            //   再叠加人工偏移容易导致"翅膀飘/歪"。
            // - 只有在未能切到 Mount* 帧段、使用旧的 rider 偏移兜底时，才需要额外修正翅膀位置。
            let wing_pos = if mounted && !rider_uses_mount_frames {
                Position::new(
                    actor_pos.x + Self::MOUNT_WING_OFFSET_X_PX,
                    actor_pos.y + Self::MOUNT_WING_OFFSET_Y_PX,
                )
            } else {
                actor_pos
            };
            let candidates = [wing_primary + wing_offset, wing_primary, wing_offset, 0];
            let mut wing_drawn = false;
            let mut wing_frame_used: Option<i32> = None;
            
            // 诊断：每帧打印翅膀候选帧的取帧情况（只打印一次）
            static WING_CANDIDATE_DIAG: std::sync::OnceLock<()> = std::sync::OnceLock::new();
            if is_local {
                let _ = WING_CANDIDATE_DIAG.set(()).map(|_| {
                    println!(
                        "[DIAG][Wing] wing_primary(effect_frame)={} wing_offset={} pos=({:.1},{:.1})",
                        wing_primary, wing_offset, wing_pos.x, wing_pos.y
                    );
                    for f in candidates {
                        let tex_info = wing_lib.get_texture(f.max(0) as usize);
                        let (has_tex, w, h, ox, oy) = match tex_info {
                            Some(info) => (
                                info.image.is_some(),
                                info.width,
                                info.height,
                                info.offset_x,
                                info.offset_y,
                            ),
                            None => (false, 0, 0, 0, 0),
                        };
                        println!(
                            "[DIAG][Wing] candidate frame={} lib={:?} has_texture={} size={}x{} offset=({},{})",
                            f, wing_lib, has_tex, w, h, ox, oy
                        );
                    }
                });
            }
            
            for f in candidates {
                // 翅膀/人物特效通常是发光效果，用 additive 混合更接近原版观感。
                if draw_layer_additive(add_blend_material, wing_lib, f, &wing_pos, tint) {
                    wing_drawn = true;
                    wing_frame_used = Some(f);
                    break;
                }
            }

            // 兜底：如果候选帧都不存在，首次扫描前 64 帧找一个"能画出来"的帧。
            if !wing_drawn {
                static WING_PROBE_ONCE: std::sync::OnceLock<Option<i32>> = std::sync::OnceLock::new();
                let probe = WING_PROBE_ONCE.get_or_init(|| {
                    // 先按当前性别的帧段（wing_offset）扫描，避免女号落到男号帧段。
                    for i in 0..64 {
                        let idx = (i + wing_offset) as usize;
                        if wing_lib.get_texture(idx).and_then(|info| info.image).is_some() {
                            return Some(i + wing_offset);
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
                    weapon_offset,
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
        let body_frame = rider_base_frame + body_hair_offset;
        let body_drew = draw_layer(
            armour_library,
            body_frame,
            &actor_pos,
            tint,
        );
        drew_any |= body_drew;
        
        // 诊断：身体层绘制情况（只打印一次）
        static BODY_DRAW_DIAG: std::sync::OnceLock<()> = std::sync::OnceLock::new();
        if is_local {
            let _ = BODY_DRAW_DIAG.set(()).map(|_| {
                let tex_info = armour_library.get_texture(body_frame.max(0) as usize);
                let (has_tex, w, h, ox, oy) = match tex_info {
                    Some(info) => (
                        info.image.is_some(),
                        info.width,
                        info.height,
                        info.offset_x,
                        info.offset_y,
                    ),
                    None => (false, 0, 0, 0, 0),
                };
                println!(
                    "[DIAG][Body] armour_lib={:?} frame={} has_texture={} size={}x{} offset=({},{}) pos=({:.1},{:.1}) drew={}",
                    armour_library, body_frame, has_tex, w, h, ox, oy, actor_pos.x, actor_pos.y, body_drew
                );
            });
        }

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
                    weapon_offset,
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
            .map(|(c, p)| (p.x, p.y, c.zoom, c.screen_width, c.screen_height))
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
            .map(|p| p.stage)
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
            .map(|tt| tt.animation_count)
            .unwrap_or(0);

        // 收集并排序可渲染对象（玩家/怪物/NPC/地面物品）
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
                name: Option<String>,
                hp_current: Option<i32>,
                hp_max: Option<i32>,
                has_interaction_hint: bool,
            },
            GroundItem {
                item: crate::components::GroundItem,
                pos: Position,
            },
        }

        fn in_view(pos: &Position, min_x: f32, min_y: f32, max_x: f32, max_y: f32, margin: f32) -> bool {
            pos.x >= min_x - margin && pos.x <= max_x + margin && pos.y >= min_y - margin && pos.y <= max_y + margin
        }

        let mut renderables: Vec<(f32, i32, Renderable)> = Vec::new();

        // Players
        if local_only {
            for eref in world.iter() {
                let Some(_local) = eref.get::<&LocalPlayer>() else { continue };
                let Some(player) = eref.get::<&Player>() else { continue };
                let Some(pos) = eref.get::<&Position>() else { continue };
                let Some(appearance) = eref.get::<&PlayerAppearance>() else { continue };
                let Some(anim_frame) = eref.get::<&AnimationFrame>() else { continue };
                let entity = eref.entity();
                if eref.get::<&crate::components::Visibility>().map(|v| !v.is_visible()).unwrap_or(false) {
                    continue;
                }
                if !in_view(&pos, view_min_x, view_min_y, view_max_x, view_max_y, CULL_MARGIN) {
                    continue;
                }
                // kind order: NPC(0) < GroundItem(1) < Monster(2) < Player(3)
                renderables.push((pos.y, 3, Renderable::Player {
                    entity,
                    player: (*player).clone(),
                    pos: *pos,
                    appearance: (*appearance).clone(),
                    anim_frame: *anim_frame,
                }));
            }
        } else {
            for eref in world.iter() {
                let Some(player) = eref.get::<&Player>() else { continue };
                let Some(pos) = eref.get::<&Position>() else { continue };
                let Some(appearance) = eref.get::<&PlayerAppearance>() else { continue };
                let Some(anim_frame) = eref.get::<&AnimationFrame>() else { continue };
                let entity = eref.entity();
                if eref.get::<&crate::components::Visibility>().map(|v| !v.is_visible()).unwrap_or(false) {
                    continue;
                }
                if !in_view(&pos, view_min_x, view_min_y, view_max_x, view_max_y, CULL_MARGIN) {
                    continue;
                }
                renderables.push((pos.y, 2, Renderable::Player {
                    entity,
                    player: (*player).clone(),
                    pos: *pos,
                    appearance: (*appearance).clone(),
                    anim_frame: *anim_frame,
                }));
            }
        }

        // NPC/Monster：渲染 LibrarySprite + 名称 + 血条
        if !local_only {
            use crate::components::{NetworkObjectType, NetworkSync};
            for (entity, (sync, spr, pos)) in world.iter().filter_map(|e| {
                let sync = e.get::<&NetworkSync>()?;
                let spr = e.get::<&LibrarySprite>()?;
                let pos = e.get::<&Position>()?;
                Some((e.entity(), (sync, spr, pos)))
            }) {
                if sync.object_type != NetworkObjectType::NPC && sync.object_type != NetworkObjectType::Monster {
                    continue;
                }
                if world.get::<&crate::components::Visibility>(entity).ok().map(|v| !v.is_visible()).unwrap_or(false) {
                    continue;
                }
                if !matches!(spr.blend_mode, SpriteBlendMode::Alpha) {
                    continue;
                }
                if !in_view(&pos, view_min_x, view_min_y, view_max_x, view_max_y, CULL_MARGIN) {
                    continue;
                }

                // 获取名称（Monster 或 OtherPlayer）
                let name = world.get::<&Monster>(entity)
                    .ok()
                    .map(|m| m.name.clone())
                    .or_else(|| world.get::<&OtherPlayer>(entity)
                        .ok()
                        .map(|op| op.name.clone()));

                // 获取血量
                let (hp_cur, hp_max) = world.get::<&Health>(entity)
                    .ok()
                    .map(|hp| (Some(hp.current), Some(hp.max)))
                    .unwrap_or((None, None));

                // 检查是否有交互提示标记
                let has_interaction_hint = world.get::<&crate::components::InteractionHint>(entity).is_ok();

                // kind order: NPC(0) < GroundItem(1) < Monster(2) < Player(3)
                let kind_order = if sync.object_type == NetworkObjectType::NPC { 0 } else { 2 };
                renderables.push((
                    pos.y,
                    kind_order,
                    Renderable::LibrarySprite {
                        spr: *spr,
                        pos: *pos,
                        kind_order,
                        name,
                        hp_current: hp_cur,
                        hp_max,
                        has_interaction_hint,
                    },
                ));
            }
        }

        // 地面物品：渲染 GroundItem + Position
        for (_entity, (gnd, pos)) in world.iter().filter_map(|e| {
            let gnd = e.get::<&crate::components::GroundItem>()?;
            let pos = e.get::<&Position>()?;
            Some((e.entity(), (gnd, pos)))
        }) {
            if !in_view(&pos, view_min_x, view_min_y, view_max_x, view_max_y, CULL_MARGIN) {
                continue;
            }
            // kind order: GroundItem(1) 在 NPC(0) 之后、Monster(2) 之前
            renderables.push((pos.y, 1, Renderable::GroundItem {
                item: (*gnd).clone(),
                pos: *pos,
            }));
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
                Renderable::LibrarySprite { spr, pos, kind_order: _, name, hp_current, hp_max, has_interaction_hint } => {
                    let tint = Color::new(1.0, 1.0, 1.0, alpha.clamp(0.0, 1.0));
                    let Some(ref info) = spr.library.get_texture(spr.texture_index()) else {
                        continue;
                    };
                    let Some(ref tex) = info.image else {
                        continue;
                    };

                    let draw_x = pos.x + info.offset_x as f32;
                    let draw_y = pos.y + info.offset_y as f32;
                    draw_texture_ex(tex, draw_x, draw_y, tint, DrawTextureParams { ..Default::default() });

                    // 名称、血条和交互提示
                    Self::draw_object_name_and_health(&pos, info, name.as_deref(), hp_current, hp_max);
                    if has_interaction_hint {
                        Self::draw_interaction_hint(&pos, info);
                    }
                }
                Renderable::GroundItem { item, pos } => {
                    // 绘制地面物品：简单图标+名称
                    Self::draw_ground_item(&item, &pos, alpha);
                }
            }
        }

        Ok(())
    }

    /// 在物体头顶绘制名称和血条
    fn draw_object_name_and_health(
        pos: &Position,
        sprite_info: &crate::resources::mlibrary::ImageInfo,
        name: Option<&str>,
        hp_current: Option<i32>,
        hp_max: Option<i32>,
    ) {
        // 名称绘制在精灵顶部
        let name_y = pos.y - sprite_info.height as f32 - 14.0;
        let name_x = pos.x - 30.0; // 居中偏移

        if let Some(name) = name {
            if !name.is_empty() {
                draw_text_cn(name, name_x, name_y, 10.0, WHITE);
            }
        }

        // 血条：在名称下方
        if let (Some(cur), Some(max)) = (hp_current, hp_max) {
            if max > 0 {
                let bar_width = 60.0;
                let bar_height = 4.0;
                let bar_x = pos.x - bar_width / 2.0;
                let bar_y = name_y + 2.0;

                // 背景
                draw_rectangle(bar_x, bar_y, bar_width, bar_height, Color::from_rgba(80, 0, 0, 200));

                // 血条填充
                let fill = (cur as f32 / max as f32).clamp(0.0, 1.0);
                let bar_color = if fill > 0.6 {
                    Color::from_rgba(0, 200, 0, 255)
                } else if fill > 0.3 {
                    Color::from_rgba(255, 255, 0, 255)
                } else {
                    Color::from_rgba(255, 50, 50, 255)
                };
                draw_rectangle(bar_x, bar_y, bar_width * fill, bar_height, bar_color);

                // 边框
                draw_rectangle_lines(bar_x, bar_y, bar_width, bar_height, 1.0, Color::from_rgba(200, 200, 200, 180));
            }
        }
    }

    /// 在 NPC 头顶绘制交互提示（黄色 "!"）
    ///
    /// 绘制位置在名称上方，使用半透明金色文字，带简单脉冲动画。
    fn draw_interaction_hint(pos: &Position, sprite_info: &crate::resources::mlibrary::ImageInfo) {
        let hint_y = pos.y - sprite_info.height as f32 - 28.0;
        let hint_x = pos.x - 4.0; // "!" 宽度约 8px，居中

        // 简单脉冲：0.7 ~ 1.0 alpha，约 3Hz
        let t = macroquad::time::get_time();
        let pulse = 0.85 + 0.15 * (t * 3.0).sin() as f32;
        let color = Color::from_rgba(255, 220, 50, (pulse * 255.0) as u8);

        draw_text_cn("!", hint_x, hint_y, 14.0, color);
    }

    /// 绘制地面物品
    ///
    /// 绘制一个半透明光晕 + 物品名称/金币数量。
    /// 未来可以从 UserItem.info 获取完整 ItemInfo 纹理。
    fn draw_ground_item(item: &crate::components::GroundItem, pos: &Position, alpha: f32) {
        // 脉冲光晕
        let t = macroquad::time::get_time();
        let pulse = 0.6 + 0.3 * (t * 2.0).sin() as f32;

        if item.gold_amount > 0 {
            // 金币袋：金色光晕
            let glow_color = Color::from_rgba(255, 215, 0, (pulse * alpha * 180.0) as u8);
            draw_circle(pos.x, pos.y, 16.0, glow_color);
            draw_text_cn(&item.gold_amount.to_string(), pos.x - 12.0, pos.y + 4.0, 10.0,
                Color::from_rgba(255, 215, 0, (alpha * 255.0) as u8));
        } else {
            // 物品：青色光晕 + 物品索引
            let glow_color = Color::from_rgba(100, 200, 255, (pulse * alpha * 150.0) as u8);
            draw_circle(pos.x, pos.y, 14.0, glow_color);

            let name = if let Some(ref info) = item.item.info {
                info.name.clone()
            } else {
                format!("物品 #{}", item.item.item_index)
            };
            draw_text_cn(&name, pos.x - 20.0, pos.y + 4.0, 9.0,
                Color::from_rgba(200, 230, 255, (alpha * 255.0) as u8));
        }
    }
}

