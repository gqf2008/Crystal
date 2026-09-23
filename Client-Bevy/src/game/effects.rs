// ============================================================================
// 战斗特效（M38）
// 绘制参考：C# Effect 帧动画 + macroquad；实现：魔法弹道（光球飞行）+ 命中爆炸（扩散环）
// 触发：S.MagicCast（服务器确认施法，且玩家有选中目标）→ 生成弹道
//      S.ObjectStruck（选中目标受击）→ 命中爆炸
// 纯客户端表现层；E2E 用 EffectsState.spawned 计数验证
// ============================================================================

use bevy::prelude::*;

use crate::actor::{LocalPlayer, NetObjectId};
use crate::map_renderer::GameLibraries;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{ui_image, UiImageCache};

/// 待生成特效（网络事件 → 渲染，按 target object_id 定位）
#[derive(Message, Debug, Clone, Copy)]
pub enum PendingEffect {
    /// 魔法弹道：从玩家飞向目标
    Projectile { target_id: u32, color: [f32; 3] },
    /// 命中爆炸：在目标位置扩散
    Burst { target_id: u32, color: [f32; 3] },
    /// 世界对象弹道：从 source 对象飞向 destination 对象（#224 ObjectProjectile/ObjectMagic/ObjectRangeAttack）
    ProjectileFromTo {
        source_id: u32,
        destination_id: u32,
        color: [f32; 3],
    },
    /// 地图坐标特效：在指定世界坐标生成爆炸（#230 MapEffect）
    BurstAt { x: f32, y: f32, color: [f32; 3] },
    /// 施法特效（2026-09-23）：按原版 `PlayerObject.cs` MirAction.Spell 的表播 Magic 库帧动画
    /// （此前施法只画一个染色白方块 —— 玩家反馈「魔法效果完全不对」）。
    SpellCast { object_id: u32, spell: u8, dir: u8 },
    /// 施法弹道（2026-09-23）：原版 `CreateProjectile(baseIndex, library, count, interval, skip)`
    /// —— 此前飞行物也是白方块；表里没有的法术再退回占位。
    SpellMissile {
        source_id: u32,
        destination_id: u32,
        spell: u8,
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
            (
                spawn_pending_effects,
                advance_projectiles,
                advance_bursts,
                advance_spell_fx,
                advance_spell_missiles,
            )
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
    opt: Res<crate::game::dialogs::option::OptionState>,
    mut images: ResMut<Assets<Image>>,
    mut libs: ResMut<GameLibraries>,
    mut cache: ResMut<UiImageCache>,
    actors: Query<(&NetObjectId, &Transform)>,
    players: Query<&Transform, (With<LocalPlayer>, With<NetObjectId>)>,
) {
    let pending: Vec<PendingEffect> = effects.read().copied().collect();
    if pending.is_empty() {
        return;
    }
    let white = images.add(crate::map_renderer::make_image(
        vec![255, 255, 255, 255],
        1,
        1,
    ));
    let player_pos = players
        .single()
        .map(|tf| Vec2::new(tf.translation.x, tf.translation.y))
        .unwrap_or_default();
    for e in pending {
        // C# OptionDialog Effect 开关：关闭时不生成特效
        if !opt.effect {
            continue;
        }
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
            PendingEffect::BurstAt { x, y, color } => {
                commands.spawn((
                    Sprite {
                        image: white.clone(),
                        color: Color::srgba(color[0], color[1], color[2], 0.9),
                        custom_size: Some(Vec2::splat(24.0)),
                        ..default()
                    },
                    Transform::from_xyz(x, y, 20.0),
                    Burst {
                        t: 0.0,
                        dur: 0.35,
                        start_scale: 0.6,
                    },
                ));
            }
            PendingEffect::SpellMissile {
                source_id,
                destination_id,
                spell,
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
                match mir2_shared::enums::Spell::try_from(spell)
                    .ok()
                    .and_then(crate::game::spell_effects::spell_missile)
                {
                    Some(m) => {
                        let Some(handle) = ui_image(
                            &mut libs,
                            &mut images,
                            &mut cache,
                            m.library.library(),
                            m.base,
                        ) else {
                            continue;
                        };
                        commands.spawn((
                            SpellMissileAnim {
                                library: m.library,
                                base: m.base,
                                frames: m.frames,
                                frame_ms: m.frame_ms as f32 / 1000.0,
                                from,
                                to,
                                t: 0.0,
                                dur: MISSILE_FLIGHT_SECS,
                            },
                            Sprite {
                                image: handle,
                                ..default()
                            },
                            bevy::sprite::Anchor::CENTER,
                            Transform::from_xyz(from.x, from.y, 21.0),
                        ));
                    }
                    None => {
                        // 表里没有 → 保持旧的占位弹道（debug 里说明，不静默）
                        debug!("施法弹道表未覆盖 spell={spell}（退回占位弹道）");
                        let color = spell_color(spell);
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
                }
            }
            PendingEffect::SpellCast {
                object_id,
                spell,
                dir,
            } => {
                // 原版：施法动作播一条（或多条）Magic/Magic2/Magic3 帧动画，跟随施法者。
                // 表里没有的法术才退回旧的占位表现，并且只在 debug 里说一声（不静默）。
                let Some((_, tf)) = actors.iter().find(|(id, _)| id.0 == object_id) else {
                    continue;
                };
                let pos = Vec2::new(tf.translation.x, tf.translation.y);
                match mir2_shared::enums::Spell::try_from(spell)
                    .ok()
                    .and_then(|sp| crate::game::spell_effects::spell_fx(sp, dir))
                {
                    Some(fx) => {
                        let (dur, frame_ms) = fx.timing();
                        let start = fx.start;
                        let Some(handle) = crate::ui::sprite_ui::ui_image(
                            &mut libs,
                            &mut images,
                            &mut cache,
                            fx.library.library(),
                            start,
                        ) else {
                            continue;
                        };
                        commands.spawn((
                            crate::game::spell_effects::SpellFxAnim {
                                library: fx.library,
                                base: start,
                                frames: fx.frames,
                                t: 0.0,
                                dur,
                                frame_ms,
                                follow_object_id: object_id,
                            },
                            Sprite {
                                image: handle,
                                ..default()
                            },
                            bevy::sprite::Anchor::CENTER,
                            Transform::from_xyz(pos.x, pos.y, 21.0),
                        ));
                    }
                    None => {
                        debug!("施法特效表未覆盖 spell={spell}（退回占位表现）",);
                        commands.spawn((
                            Sprite {
                                image: white.clone(),
                                color: Color::srgba(0.9, 0.8, 0.4, 0.9),
                                custom_size: Some(Vec2::splat(24.0)),
                                ..default()
                            },
                            Transform::from_xyz(pos.x, pos.y, 21.0),
                            Burst {
                                t: 0.0,
                                dur: 0.35,
                                start_scale: 0.6,
                            },
                        ));
                    }
                }
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

/// 施法特效播帧：按原版语义「在 dur 内播完 frames 帧」，并跟随施法者当前位置
/// （C# `new Effect(lib, start, frames, interval, ob)` 的 ob 跟随行为）。
fn advance_spell_fx(
    time: Res<Time>,
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    // B0001 回归：q 写 Transform 与 actors 读 Transform 冲突，进游戏即 panic（见下方门禁测试）。
    // 跟随源/目标永远是场景角色，绝不携带 SpellFxAnim，Without 划界即可证明两查询不相交。
    actors: Query<(&NetObjectId, &Transform), Without<crate::game::spell_effects::SpellFxAnim>>,
    mut q: Query<(
        Entity,
        &mut crate::game::spell_effects::SpellFxAnim,
        &mut Sprite,
        &mut Transform,
    )>,
) {
    for (e, mut fx, mut sprite, mut tf) in &mut q {
        fx.t += time.delta_secs();
        if fx.t >= fx.dur {
            commands.entity(e).despawn();
            continue;
        }
        // 跟随施法者（对象还在的话）
        if fx.follow_object_id != 0 {
            if let Some((_, atf)) = actors.iter().find(|(id, _)| id.0 == fx.follow_object_id) {
                tf.translation.x = atf.translation.x;
                tf.translation.y = atf.translation.y;
            }
        }
        let frame = ((fx.t / fx.frame_ms).floor() as usize).min(fx.frames.saturating_sub(1));
        if let Some(h) = ui_image(
            &mut libs,
            &mut images,
            &mut cache,
            fx.library.library(),
            fx.base + frame,
        ) {
            sprite.image = h;
        }
    }
}

/// 施法弹道飞行时长（秒）。原版速度由 Missile 的 interval×count 与距离共同决定，
/// 本端先用一个固定飞行时长（与旧占位弹道的 0.35s 一致），后续可按原版调优。
pub const MISSILE_FLIGHT_SECS: f32 = 0.35;

/// 施法弹道实体：一边飞一边按原版帧表循环播（C# Missile 的帧循环）
#[derive(Component, Debug, Clone, Copy)]
struct SpellMissileAnim {
    library: crate::game::spell_effects::SpellFxLibrary,
    base: usize,
    frames: usize,
    frame_ms: f32,
    from: Vec2,
    to: Vec2,
    t: f32,
    dur: f32,
}

/// 弹道推进：位置缓出插值 + 帧循环（帧用完从头循环，直到到达目标）
fn advance_spell_missiles(
    mut commands: Commands,
    time: Res<Time>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mut q: Query<(Entity, &mut SpellMissileAnim, &mut Sprite, &mut Transform)>,
) {
    for (e, mut m, mut sprite, mut tf) in &mut q {
        m.t += time.delta_secs();
        let k = (m.t / m.dur).min(1.0);
        let k2 = 1.0 - (1.0 - k) * (1.0 - k);
        let pos = m.from.lerp(m.to, k2);
        tf.translation.x = pos.x;
        tf.translation.y = pos.y;
        if m.t >= m.dur {
            commands.entity(e).despawn();
            continue;
        }
        let step = (m.t / m.frame_ms.max(0.001)).floor() as usize;
        let frame = step % m.frames.max(1);
        if let Some(h) = ui_image(
            &mut libs,
            &mut images,
            &mut cache,
            m.library.library(),
            m.base + frame,
        ) {
            sprite.image = h;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::spell_effects::{SpellFxAnim, SpellFxLibrary};

    /// 门禁（接线，不只是表）：收到 `PendingEffect::SpellCast` 必须生成**施法帧动画实体**
    /// （带 SpellFxAnim + 起始帧 = 表里的 start），而不是退回白方块。
    ///
    /// 为什么要有这条：表对了但没接线的话，玩家看到的仍然是白方块——本端此前的回归就是
    /// 「表/渲染都在，但事件路径没接」这一类。
    ///
    /// 阳性对照（实做）：把 spawn_pending_effects 里 SpellCast 分支改成直接 continue
    /// （不生成任何实体）→ 本测试立即红。
    #[test]
    fn spell_cast_spawns_library_frame_animation() {
        use bevy::ecs::system::RunSystemOnce;
        if !crate::resources::libraries::data_assets_present() {
            eprintln!("skip spell_cast_spawns_library_frame_animation: 无 Data 资产");
            return;
        }
        let mut world = bevy::prelude::World::new();
        world.insert_resource(crate::map_renderer::GameLibraries(
            crate::resources::libraries::Libraries::new(
                crate::resources::libraries::resolve_data_path(),
            ),
        ));
        world.insert_resource(bevy::prelude::Assets::<bevy::prelude::Image>::default());
        world.insert_resource(crate::ui::sprite_ui::UiImageCache::default());
        world.insert_resource(crate::game::dialogs::option::OptionState {
            effect: true,
            ..Default::default()
        });
        world.insert_resource(EffectsState::default());
        // 库是惰性初始化的：不先 ensure，ui_image 取不到 Magic[0] → 会走 continue 而不是生成实体
        world
            .resource_mut::<crate::map_renderer::GameLibraries>()
            .0
            .ensure_initialized();
        world.insert_resource(bevy::prelude::Messages::<PendingEffect>::default());
        // 一个「施法者」对象，位置随便
        world.spawn((
            NetObjectId(4242),
            bevy::prelude::Transform::from_xyz(100.0, 200.0, 0.0),
        ));
        // 一次 run 里同时排「施法帧动画 + 弹道」两条（同一个 MessageReader 会把缓冲里的
        // 消息全部读走；分两次 run 会因为新 reader 重读旧消息而把计数翻倍）
        world.spawn((
            NetObjectId(4243),
            bevy::prelude::Transform::from_xyz(300.0, 200.0, 0.0),
        ));
        {
            let mut msgs = world.resource_mut::<bevy::prelude::Messages<PendingEffect>>();
            msgs.write(PendingEffect::SpellCast {
                object_id: 4242,
                spell: mir2_shared::enums::Spell::FireBall as u8,
                dir: 0,
            });
            msgs.write(PendingEffect::SpellMissile {
                source_id: 4242,
                destination_id: 4243,
                spell: mir2_shared::enums::Spell::FireBall as u8,
            });
        }
        world
            .run_system_once(spawn_pending_effects)
            .expect("spawn_pending_effects 应能运行");
        let mut mq = world.query::<&SpellMissileAnim>();
        let missiles: Vec<&SpellMissileAnim> = mq.iter(&world).collect();
        assert_eq!(missiles.len(), 1, "FireBall 必须生成一条弹道帧动画实体");
        assert_eq!(
            missiles[0].base, 10,
            "原版 CreateProjectile(10, Magic, 6, 30, 4)"
        );
        assert_eq!(missiles[0].frames, 6);

        let mut q = world.query::<&SpellFxAnim>();
        let fx: Vec<&SpellFxAnim> = q.iter(&world).collect();
        assert_eq!(fx.len(), 1, "FireBall 必须生成一条施法帧动画实体");
        assert_eq!(fx[0].library, SpellFxLibrary::Magic);
        assert_eq!(fx[0].base, 0, "Magic[0] 起（原版 PlayerObject.cs）");
        assert_eq!(fx[0].frames, 10);
        assert_eq!(fx[0].follow_object_id, 4242, "跟随施法者");
    }

    /// B0001 接线门禁（P0，实机启动即崩挖出）：插件注册的五条特效系统放进同一调度
    /// 连跑必须能初始化——`advance_spell_fx` 曾因 `actors` 读 Transform 与 `q` 写
    /// Transform 未划界而在进游戏瞬间 panic（error\[B0001\]），表现为「一进游戏即崩」。
    ///
    /// 为什么单系统 run_system_once 拦不住：B0001 是**调度初始化期**对同系统内
    /// 多参数的冲突校验，逐系统单跑永远遇不到；只有像插件那样注册进 Schedule 才触发。
    ///
    /// 阳性对照：把 `actors` 查询的 `Without<SpellFxAnim>` 去掉 → 本测试立即红
    /// （实机已验证该 panic 真实发生，见 PR 描述）。
    #[test]
    fn effects_update_systems_init_without_query_conflict() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.insert_state(crate::scenes::AppState::Game);
        app.init_resource::<EffectsState>();
        app.add_message::<PendingEffect>();
        app.init_resource::<crate::game::dialogs::option::OptionState>();
        app.insert_resource(bevy::prelude::Assets::<bevy::prelude::Image>::default());
        app.insert_resource(crate::map_renderer::GameLibraries(
            crate::resources::libraries::Libraries::new(
                crate::resources::libraries::resolve_data_path(),
            ),
        ));
        app.insert_resource(crate::ui::sprite_ui::UiImageCache::default());
        // 与 EffectsPlugin 相同的五条系统、同样的链式注册（.after/run_if 与冲突校验无关，从略）
        app.add_systems(
            Update,
            (
                spawn_pending_effects,
                advance_projectiles,
                advance_bursts,
                advance_spell_fx,
                advance_spell_missiles,
            )
                .chain(),
        );
        // 首帧即完成调度初始化：有 B0001 冲突时这里直接 panic
        app.update();
        app.update();
    }
}
