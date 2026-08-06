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
#[derive(Message, Debug, Clone, Copy)]
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
    /// 世界对象弹道：从 source 对象飞向 destination 对象（#224 ObjectProjectile/ObjectMagic/ObjectRangeAttack）
    ProjectileFromTo {
        source_id: u32,
        destination_id: u32,
        color: [f32; 3],
    },
}

/// 技能 → 弹道颜色（#224，参考 macroquad network_apply_system 的 Spell 映射）
pub(crate) fn spell_color(spell: u8) -> [f32; 3] {
    use mir2_shared::enums::Spell;
    match Spell::try_from(spell) {
        Ok(Spell::FireBall) => [1.0, 0.55, 0.1],
        Ok(Spell::GreatFireBall) | Ok(Spell::HellFire) => [1.0, 0.2, 0.1],
        Ok(Spell::ThunderBolt) | Ok(Spell::Lightning) => [0.5, 0.5, 1.0],
        Ok(Spell::Healing) => [0.3, 1.0, 0.4],
        Ok(Spell::Poisoning) => [0.6, 0.2, 0.7],
        Ok(Spell::Teleport) => [1.0, 1.0, 1.0],
        Ok(Spell::MagicShield) => [0.4, 0.8, 1.0],
        Ok(Spell::HalfMoon) => [1.0, 1.0, 0.7],
        Ok(Spell::ShoulderDash) => [0.8, 0.8, 0.8],
        _ => [1.0, 1.0, 0.4],
    }
}

/// SpellEffect → 特效颜色（#224，参考 macroquad 的 暴击/致命/护盾 映射）
pub(crate) fn spell_effect_color(effect: u8) -> [f32; 3] {
    use mir2_shared::enums::SpellEffect;
    match SpellEffect::try_from(effect) {
        Ok(SpellEffect::Critical) => [1.0, 0.9, 0.2],
        Ok(SpellEffect::FatalSword) => [1.0, 0.3, 0.3],
        Ok(SpellEffect::MagicShieldUp) => [0.4, 0.8, 1.0],
        Ok(SpellEffect::MagicShieldDown) => [1.0, 0.5, 0.2],
        Ok(SpellEffect::Healing) => [0.3, 1.0, 0.4],
        Ok(SpellEffect::Teleport) => [1.0, 1.0, 1.0],
        Ok(SpellEffect::Stunned) => [0.9, 0.9, 0.4],
        _ => [1.0, 0.8, 0.3],
    }
}

/// 特效状态（已生成特效计数，E2E 验证用；待生成特效走 Message<PendingEffect>）
#[derive(Resource, Default)]
pub struct EffectsState {
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
        app.add_message::<PendingEffect>();
        app.add_systems(
            Update,
            (spawn_pending_effects, advance_projectiles, advance_bursts)
                .chain()
                .after(crate::network::network_system)
                .run_if(in_state(AppState::Game)),
        );
    }
}

/// 消费 pending：按目标实体定位生成弹道/爆炸
fn spawn_pending_effects(
    mut commands: Commands,
    mut state: ResMut<EffectsState>,
    mut effects: MessageReader<PendingEffect>,
    mut images: ResMut<Assets<Image>>,
    actors: Query<(&NetObjectId, &Transform)>,
    players: Query<&Transform, (With<LocalPlayer>, With<NetObjectId>)>,
) {
    let pending: Vec<PendingEffect> = effects.read().copied().collect();
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
            PendingEffect::ProjectileFromTo {
                source_id,
                destination_id,
                color,
            } => {
                let mut from = None;
                let mut to = None;
                for (id, tf) in &actors {
                    if id.0 == source_id {
                        from = Some(Vec2::new(tf.translation.x, tf.translation.y));
                    }
                    if id.0 == destination_id {
                        to = Some(Vec2::new(tf.translation.x, tf.translation.y));
                    }
                }
                let (Some(from), Some(to)) = (from, to) else {
                    continue;
                };
                commands.spawn((
                    Sprite {
                        image: white.clone(),
                        color: Color::srgb(color[0], color[1], color[2]),
                        custom_size: Some(Vec2::splat(14.0)),
                        ..default()
                    },
                    Transform::from_xyz(from.x, from.y, 20.0),
                    Projectile {
                        from,
                        to,
                        t: 0.0,
                        dur: 0.35,
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
