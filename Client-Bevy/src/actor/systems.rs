// ============================================================================
// actor 模块拆分（#72）
// ============================================================================

use bevy::prelude::*;
use mir2_shared::MirAction;
use crate::resources::libraries::ArrayLibType;
use crate::objects::frames::get_player_frame;
use crate::map_renderer::{FrontTile, TILE_HEIGHT, TILE_WIDTH};
use crate::game::hud::HudState;
use super::components::*;
use super::frames::{actor_frame, mount_lib_frames, mount_player_action};
use super::spawn::depth_z;

pub(crate) fn advance_actor_animations(
    time: Res<Time>,
    mut commands: Commands,
    sound_bank: Res<crate::game::sound::SoundBank>,
    mut audio_assets: ResMut<Assets<bevy::audio::AudioSource>>,
    mut actors: Query<(
        Option<&ActorAppearance>,
        Option<&MonsterAppearance>,
        Option<&NpcAppearance>,
        &mut ActorAnim,
        &Children,
        Option<&MountState>,
    )>,
    mut layers: Query<&mut SpriteLayer>,
) {
    let dt_ms = time.delta_secs() * 1000.0;
    for (player, monster, npc, mut anim, children, mounted) in &mut actors {
        let Some(frame) = actor_frame(player, monster, npc, &anim) else {
            continue;
        };
        // #1624：挥击音边缘检测（C# MonsterObject.cs:2126 Attack2 第 3 帧 PlaySwingSound）
        let prev_frame = anim.frame_index;
        let draw_frame = frame.start + (anim.direction as i32) * frame.offset() + anim.frame_index;
        let effect_frame =
            frame.effect_start + (anim.direction as i32) * frame.effect_offset() + anim.frame_index;

        // M60：骑乘时玩家帧表改用坐骑动作（C# Frames.cs Mounts 段）
        let mount_draw_frame = mounted.is_some().then(|| {
            let ma = mount_player_action(anim.action);
            get_player_frame(ma)
                .map(|mf| mf.start + (anim.direction as i32) * mf.offset() + anim.frame_index)
                .unwrap_or(draw_frame)
        });
        let (mount_base, mount_off) = mount_lib_frames(anim.action);
        let mount_layer_frame = mount_base + (anim.direction as i32) * mount_off + anim.frame_index;

        anim.elapsed_ms += dt_ms;
        let interval = frame.interval.max(1) as f32;
        let count = frame.count.max(1);
        while anim.elapsed_ms >= interval {
            anim.elapsed_ms -= interval;
            anim.frame_index = (anim.frame_index + 1) % count;
        }

        // #1624/#1627：怪物挥击音——按 C# 帧号边缘触发（进入目标帧一次）：
        //   Attack1 帧 3（MonsterObject.cs:1589）、Attack2 帧 3（:2126）、Attack3 帧 2（:2407）
        let swing_frame: i32 = match anim.action {
            MirAction::Attack1 | MirAction::Attack2 => 3,
            MirAction::Attack3 => 2,
            _ => i32::MAX,
        };
        if anim.frame_index == swing_frame && prev_frame != swing_frame {
            if let Some(monster) = monster {
                if let Some(sound_id) = crate::game::sound::monster_swing_sound(monster.monster_type) {
                    crate::game::sound::play_sound(&mut commands, &mut audio_assets, &sound_bank, sound_id);
                }
            }
        }

        // #1632：怪物行走音——Walking 第 1 帧左步 / 第 4 帧右步（C# MonsterObject.cs:1280-1283），边缘触发
        if anim.action == MirAction::Walking && anim.frame_index != prev_frame {
            let step_left = match anim.frame_index {
                1 => Some(true),
                4 => Some(false),
                _ => None,
            };
            if let Some(left) = step_left {
                if let Some(monster) = monster {
                    if let Some(sound_id) = crate::game::sound::monster_walk_sound(monster.monster_type, left) {
                        crate::game::sound::play_sound(&mut commands, &mut audio_assets, &sound_bank, sound_id);
                    }
                }
            }
        }

        for child in children.iter() {
            if let Ok(mut layer) = layers.get_mut(child) {
                if layer.is_mount {
                    layer.frame = mount_layer_frame;
                } else if layer.is_effect {
                    layer.frame = effect_frame;
                } else if let Some(mdf) = mount_draw_frame {
                    layer.frame = mdf;
                } else {
                    layer.frame = draw_frame;
                }
            }
        }
    }
}

/// 渲染：按 SpriteLayer 帧号取图（带缓存），更新 Sprite 与相对位置

pub(crate) fn dump_depth_debug(
    actors: Query<(
        Option<&ActorAppearance>,
        Option<&MonsterAppearance>,
        Option<&NpcAppearance>,
        &Transform,
    )>,
    front: Query<(&Transform, &crate::map_renderer::FrontTile)>,
    ghosts: Query<&Visibility, With<GhostLayer>>,
    local: Query<&Transform, (With<LocalPlayer>, Without<GhostLayer>)>,
    q_layer: Query<&SpriteLayer>,
    mut frames: Local<u32>,
) {
    if std::env::var_os("CRYSTAL_DEPTH_DEBUG").is_none() {
        return;
    }
    *frames += 1;
    if *frames != 30 {
        return;
    }
    let mut lines = String::from("depth debug\n");
    for (player, monster, npc, tf) in &actors {
        let label = if player.is_some() {
            "player"
        } else if monster.is_some() {
            "monster"
        } else if npc.is_some() {
            "npc"
        } else {
            continue;
        };
        lines.push_str(&format!(
            "  actor {} at world_y={:.0} z={:.4}\n",
            label, -tf.translation.y, tf.translation.z
        ));
    }
    // 角色附近（中心 1280x800 视野内）的 front 瓦片 z
    let mut fz: Vec<(f32, f32)> = front
        .iter()
        .filter(|(_, ft)| ft.base_y > 10800.0 && ft.base_y < 11600.0)
        .map(|(tf, _)| (tf.translation.z, tf.translation.x))
        .collect();
    fz.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    lines.push_str(&format!(
        "  front tiles in view(base_y 10800..11600): {}  z range {:.4}..{:.4}\n",
        fz.len(),
        fz.first().map(|x| x.0).unwrap_or(0.0),
        fz.last().map(|x| x.0).unwrap_or(0.0)
    ));
    // 角色纹理诊断：统计角色实体上 Sprite 图像是否有效
    {
        let layers = q_layer.iter().count();
        lines.push_str(&format!("  sprite_layers={}\n", layers));
    }

    // ghost 统计：玩家前方遮挡瓦片数（本地玩家）
    let player_y = actors
        .iter()
        .filter(|(player, _, _, _)| player.is_some())
        .map(|(_, _, _, tf)| -tf.translation.y)
        .next();
    if let Some(foot_y) = player_y {
        let occluding = front
            .iter()
            .filter(|(tf, ft)| {
                ft.bottom > foot_y
                    && (tf.translation.x - 16600.0).abs() < 300.0
                    && ft.base_y < foot_y + 500.0
            })
            .count();
        lines.push_str(&format!(
            "  player foot_y={:.0} occluding_front_tiles={}\n",
            foot_y, occluding
        ));
    }
    // ghost 状态：本地玩家是否被遮挡、残影是否可见
    if let Ok(tf) = local.single() {
        let foot_x = tf.translation.x;
        let foot_y = -tf.translation.y;
        let occluded = front.iter().any(|(_, ft)| {
            ft.bottom > foot_y
                && ft.left < foot_x + 22.0
                && ft.right > foot_x - 22.0
                && ft.top < foot_y + 2.0
                && ft.bottom > foot_y - 92.0
        });
        let visible_count = ghosts.iter().filter(|v| **v == Visibility::Visible).count();
        lines.push_str(&format!(
            "  ghost: occluded={} visible_layers={}/{}\n",
            occluded,
            visible_count,
            ghosts.iter().count()
        ));
    }
    let _ = std::fs::write("E:/tmp/depth_debug.txt", lines);
}

/// Ghost 残影层标记：镜像本地玩家的对应图层（按库匹配）
#[allow(clippy::type_complexity)]
pub(crate) fn update_local_ghost(
    mut ghosts: Query<
        (&mut Sprite, &mut Transform, &mut Visibility, &GhostLayer),
        (Without<SpriteLayer>, Without<LocalPlayer>),
    >,
    local: Query<(&Transform, &Children), (With<LocalPlayer>, Without<GhostLayer>)>,
    layers: Query<(&Sprite, &Transform, &SpriteLayer), Without<GhostLayer>>,
    front: Query<&crate::map_renderer::FrontTile>,
) {
    let Ok((root_tf, children)) = local.single() else {
        return;
    };
    let foot_x = root_tf.translation.x;
    let foot_y = -root_tf.translation.y;
    // 玩家身体包围盒（覆盖身体/武器/翅膀）
    let (bl, bt, br, bb) = (foot_x - 22.0, foot_y - 92.0, foot_x + 22.0, foot_y + 2.0);
    let occluded = front.iter().any(|ft| {
        ft.bottom > foot_y && ft.left < br && ft.right > bl && ft.top < bb && ft.bottom > bt
    });
    const GHOST_ALPHA: f32 = 0.55; // 与 macroquad PLAYER_GHOST_ALPHA 一致
    const GHOST_LOCAL_Z: f32 = 0.5; // 本地 z 偏移：保证世界 z 高于所有 front 瓦片

    for (mut gs, mut gt, mut gv, gl) in &mut ghosts {
        let mut matched = None;
        for child in children.iter() {
            if let Ok((ls, lt, ll)) = layers.get(child) {
                if ll.lib == gl.lib {
                    matched = Some((ls, lt));
                    break;
                }
            }
        }
        match matched {
            Some((ls, lt)) if occluded => {
                gs.image = ls.image.clone();
                gs.color = Color::srgba(1.0, 1.0, 1.0, GHOST_ALPHA);
                gt.translation = Vec3::new(lt.translation.x, lt.translation.y, GHOST_LOCAL_Z);
                *gv = Visibility::Visible;
            }
            _ => {
                *gv = Visibility::Hidden;
            }
        }
    }
}

/// 调试：记录玩家位置采样，验证移动平滑（每 6 帧一次，前 90 帧，CRYSTAL_DEPTH_DEBUG 开启）
pub(crate) fn log_player_walk(local: Query<&Transform, With<LocalPlayer>>, mut frames: Local<u32>) {
    if std::env::var_os("CRYSTAL_DEPTH_DEBUG").is_none() {
        return;
    }
    *frames += 1;
    if !(*frames).is_multiple_of(6) {
        return;
    }
    if let Ok(tf) = local.single() {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("E:/tmp/player_walk.txt")
        {
            let _ = writeln!(
                f,
                "frame={} x={:.1} y={:.1}",
                *frames, tf.translation.x, tf.translation.y
            );
        }
    }
}

/// 角色 z 与脚底世界 Y 保持同步（移动/转向时深度正确）
pub(crate) fn sync_actor_depth(
    mut actors: Query<
        &mut Transform,
        Or<(With<Player>, With<Monster>, With<Npc>)>,
    >,
) {
    for mut tf in &mut actors {
        // translation.y = -世界Y（Bevy y 向上）
        tf.translation.z = depth_z(-tf.translation.y);
    }
}

/// 穿戴装备后同步本地玩家外观层（SpriteLayer slot ← HudState.equipment）
/// 槽位：0=武器(CWeapon) 1=衣服(CArmour) 2=头盔 3=项链 ... 与 ServerRust EquipmentSlot 一致
pub(crate) fn sync_player_equipment(
    hud: Res<crate::game::hud::HudState>,
    players: Query<(Entity, &Children), (With<LocalPlayer>, With<Player>)>,
    mut layers: Query<&mut SpriteLayer>,
) {
    let Ok((_, children)) = players.single() else { return };
    let armour_slot = hud
        .equipment
        .get(1)
        .and_then(|s| s.as_ref())
        .map(|i| i.item_index.max(0) as u32)
        .unwrap_or(0);
    let weapon_slot = hud
        .equipment
        .get(0)
        .and_then(|s| s.as_ref())
        .map(|i| i.item_index.max(0) as u32)
        .unwrap_or(0);
    for child in children.iter() {
        if let Ok(mut layer) = layers.get_mut(child) {
            match layer.lib {
                ArrayLibType::CArmours => {
                    if layer.slot != armour_slot {
                        layer.slot = armour_slot;
                        layer.frame = 0;
                    }
                }
                ArrayLibType::CWeapons => {
                    if layer.slot != weapon_slot {
                        layer.slot = weapon_slot;
                        layer.frame = 0;
                    }
                }
                _ => {}
            }
        }
    }
}

/// 演示驱动：玩家绕方块行走；怪物/NPC 原地转向；部分怪物周期性攻击
/// #573：坐下的对象（Sitting）不参与演示驱动，保持坐姿不自动转向
pub(crate) fn demo_drive(
    time: Res<Time>,
    mut actors: Query<
        (&mut ActorAnim, &mut Transform, &mut DemoBehavior),
        Without<crate::actor::Sitting>,
    >,
) {
    let dt = time.delta_secs();

    for (mut anim, mut tf, mut behavior) in &mut actors {
        match behavior.as_mut() {
            DemoBehavior::Walk {
                side_len,
                side_progress,
                direction,
                step_progress,
                from_x,
                from_y,
                to_x,
                to_y,
                started,
            } => {
                anim.action = MirAction::Walking;
                anim.direction = *direction;
                let step_time = 0.6; // 与 Walking 帧间隔同步（6帧 * 100ms）
                if !*started {
                    // 首次：从当前位置初始化目标
                    *from_x = tf.translation.x;
                    *from_y = -tf.translation.y;
                    let (dx, dy) = dir_vec(*direction);
                    *to_x = *from_x + dx * TILE_WIDTH;
                    *to_y = *from_y + dy * TILE_HEIGHT;
                    *started = true;
                    *step_progress = 0.0;
                }
                *step_progress += dt / step_time;
                // 低帧率时可能一次跨多步：逐格完成
                while *step_progress >= 1.0 {
                    *step_progress -= 1.0;
                    *from_x = *to_x;
                    *from_y = *to_y;
                    *side_progress += 1;
                    if *side_progress >= *side_len {
                        *side_progress = 0;
                        *direction = (*direction + 2) % 8;
                        anim.direction = *direction;
                    }
                    let (dx, dy) = dir_vec(*direction);
                    *to_x = *from_x + dx * TILE_WIDTH;
                    *to_y = *from_y + dy * TILE_HEIGHT;
                }
                // 平滑插值：从当前格到目标格
                let t = (*step_progress).clamp(0.0, 1.0);
                tf.translation.x = *from_x + (*to_x - *from_x) * t;
                tf.translation.y = -(*from_y + (*to_y - *from_y) * t);
            }
            DemoBehavior::Idle { timer, interval } => {
                *timer += dt;
                if *timer >= *interval {
                    *timer = 0.0;
                    anim.direction = (anim.direction + 1) % 8;
                }
                anim.action = MirAction::Standing;
            }
            DemoBehavior::Attack {
                timer,
                interval,
                attacking,
                attack_timer,
            } => {
                *timer += dt;
                if *attacking {
                    *attack_timer += dt;
                    anim.action = MirAction::Attack1;
                    if *attack_timer >= 0.7 {
                        *attacking = false;
                        *attack_timer = 0.0;
                        anim.frame_index = 0;
                        anim.elapsed_ms = 0.0;
                    }
                } else {
                    anim.action = MirAction::Standing;
                    if *timer >= *interval {
                        *timer = 0.0;
                        *attacking = true;
                        anim.frame_index = 0;
                        anim.elapsed_ms = 0.0;
                    }
                }
            }
        }
    }
}

/// MirDirection: 0=Up 1=UpRight 2=Right 3=DownRight 4=Down 5=DownLeft 6=Left 7=UpLeft
fn dir_vec(d: u8) -> (f32, f32) {
    match d % 8 {
        0 => (0.0, -1.0),
        1 => (1.0, -1.0),
        2 => (1.0, 0.0),
        3 => (1.0, 1.0),
        4 => (0.0, 1.0),
        5 => (-1.0, 1.0),
        6 => (-1.0, 0.0),
        _ => (-1.0, -1.0),
    }
}



