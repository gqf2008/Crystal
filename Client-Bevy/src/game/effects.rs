// ============================================================================
// 战斗特效（M38）
// 绘制参考：C# Effect 帧动画 + macroquad；实现：魔法弹道（光球飞行）+ 命中爆炸（扩散环）
// 触发：S.MagicCast（服务器确认施法，且玩家有选中目标）→ 生成弹道
//      S.ObjectStruck（选中目标受击）→ 命中爆炸
// 纯客户端表现层；E2E 用 EffectsState.spawned 计数验证
// ============================================================================

use bevy::prelude::*;

use crate::actor::{LocalPlayer, NetObjectId};
use crate::scenes::AppState;

/// 待生成特效（网络事件 → 渲染，按 target object_id 定位）
#[derive(Debug, Clone, Copy)]
pub enum PendingEffect {
    /// 魔法弹道：从玩家飞向目标
    Projectile {
        target_id: u32,
        color: [f32; 3],
    },
    /// 命中爆炸：在目标位置扩散
    Burst {
        target_id: u32,
        color: [f32; 3],
    },
}

/// 特效状态（网络层写入 pending，特效系统消费）
#[derive(Resource, Default)]
pub struct EffectsState {
    pub pending: Vec<PendingEffect>,
    /// 已生成特效计数（E2E 验证）
    pub spawned: u64,
}

#[derive(Component)]
struct Projectile {
    from: Vec2,
    to: Vec2,
    t: f32,
    dur: f32,
}

#[derive(Component)]
struct Burst {
    t: f32,
    dur: f32,
    start_scale: f32,
}

pub struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EffectsState>();
        app.add_systems(
            Update,
            (spawn_pending_effects, advance_projectiles, advance_bursts)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

/// 消费 pending：按目标实体定位生成弹道/爆炸
fn spawn_pending_effects(
    mut commands: Commands,
    mut state: ResMut<EffectsState>,
    mut images: ResMut<Assets<Image>>,
    actors: Query<(&NetObjectId, &Transform)>,
    players: Query<&Transform, (With<LocalPlayer>, With<NetObjectId>)>,
) {
    let pending = std::mem::take(&mut state.pending);
    if pending.is_empty() {
        return;
    }
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    let player_pos = players
        .single()
        .map(|tf| Vec2::new(tf.translation.x, tf.translation.y))
        .unwrap_or_default();
    for e in pending {
        state.spawned += 1;
        match e {
            PendingEffect::Projectile { target_id, color } => {
                let Some((_, tf)) = actors.iter().find(|(id, _)| id.0 == target_id) else {
                    continue;
                };
                let to = Vec2::new(tf.translation.x, tf.translation.y);
                commands.spawn((
                    Sprite {
                        image: white.clone(),
                        color: Color::srgb(color[0], color[1], color[2]),
                        custom_size: Some(Vec2::splat(14.0)),
                        ..default()
                    },
                    Transform::from_xyz(player_pos.x, player_pos.y, 20.0),
                    Projectile {
                        from: player_pos,
                        to,
                        t: 0.0,
                        dur: 0.28,
                    },
                ));
            }
            PendingEffect::Burst { target_id, color } => {
                let Some((_, tf)) = actors.iter().find(|(id, _)| id.0 == target_id) else {
                    continue;
                };
                commands.spawn((
                    Sprite {
                        image: white.clone(),
                        color: Color::srgba(color[0], color[1], color[2], 0.9),
                        custom_size: Some(Vec2::splat(24.0)),
                        ..default()
                    },
                    Transform::from_xyz(tf.translation.x, tf.translation.y, 20.0),
                    Burst {
                        t: 0.0,
                        dur: 0.35,
                        start_scale: 0.6,
                    },
                ));
            }
        }
    }
}

/// 弹道飞行（缓出）后消失
fn advance_projectiles(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut Projectile, &mut Transform)>,
) {
    for (e, mut p, mut tf) in &mut q {
        p.t += time.delta_secs();
        let k = (p.t / p.dur).min(1.0);
        let k2 = 1.0 - (1.0 - k) * (1.0 - k);
        let pos = p.from.lerp(p.to, k2);
        tf.translation.x = pos.x;
        tf.translation.y = pos.y;
        if p.t >= p.dur {
            commands.entity(e).despawn();
        }
    }
}

/// 命中爆炸：扩散 + 淡出
fn advance_bursts(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut Burst, &mut Sprite)>,
) {
    for (e, mut b, mut sp) in &mut q {
        b.t += time.delta_secs();
        let k = (b.t / b.dur).min(1.0);
        sp.custom_size = Some(Vec2::splat(24.0 * (b.start_scale + k * 2.5)));
        sp.color.set_alpha((1.0 - k) * 0.9);
        if b.t >= b.dur {
            commands.entity(e).despawn();
        }
    }
}
