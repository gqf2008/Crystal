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
use crate::resources::libraries::ArrayLibType;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{ui_array_image, ui_image, UiImageCache};

/// 待生成特效（网络事件 → 渲染，按 target object_id 定位）
#[derive(Message, Debug, Clone, Copy)]
pub enum PendingEffect {
    /// 魔法弹道：从玩家飞向目标。`fx` 有值时按原版帧表播（远程攻击的箭矢），否则退回染色方块。
    Projectile {
        target_id: u32,
        color: [f32; 3],
        fx: Option<crate::game::spell_effects::MissileFx>,
    },
    /// 命中爆炸：在目标位置扩散
    Burst { target_id: u32, color: [f32; 3] },
    /// 世界对象弹道：从 source 对象飞向 destination 对象（#224 ObjectProjectile/ObjectMagic/ObjectRangeAttack）
    ProjectileFromTo {
        source_id: u32,
        destination_id: u32,
        color: [f32; 3],
        fx: Option<crate::game::spell_effects::MissileFx>,
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
    /// 对象特效（`S.ObjectEffect`）：原版 `GameScene.cs:4711-4930` 的 `ObjectEffect` switch
    /// ——护盾光环 / 传送 / 治疗 / 暴击 / 冰柱 / 天罚 / 觉醒 / 月雾…每一类都是真帧动画。
    /// 此前这些**一律画成染色方块**（本变体就是那处占位表现的替代）。
    ObjectEffect {
        /// 包里的 `p.ObjectID`
        object_id: u32,
        /// `p.Effect`（`SpellEffect` 枚举值，用来查表）
        effect: u8,
        /// `p.EffectType`：多数 case 是帧段步进；MPEater 第二条里它是**另一个对象 ID**
        effect_type: u32,
        /// `p.Time`（ms）：`Repeat = p.Time > 0` 类循环的持续时间
        time: u32,
        /// `p.DelayTime`（ms）：原版 `StartTime = CMain.Time + p.DelayTime`
        delay_ms: u32,
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

/// `S.RangeAttack`（本地玩家远程攻击）→ 弹道特效：**原版帧表优先**
/// （C# `PlayerObject.cs` MirAction.AttackRange1/2/3 的 `CreateProjectile`），
/// 表未覆盖的技能才退回占位色块。这是该映射的**单一出口**，门禁直接打这里。
pub fn range_attack_projectile(target_id: u32, spell: u8) -> PendingEffect {
    PendingEffect::Projectile {
        target_id,
        color: spell_color(spell),
        fx: crate::game::spell_effects::range_missile(spell),
    }
}

/// `S.ObjectRangeAttack`（其他玩家/怪物远程攻击）→ 弹道特效（同上，单一出口）
pub fn object_range_attack_projectile(
    source_id: u32,
    destination_id: u32,
    spell: u8,
) -> PendingEffect {
    PendingEffect::ProjectileFromTo {
        source_id,
        destination_id,
        color: spell_color(spell),
        fx: crate::game::spell_effects::range_missile(spell),
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
    /// `SpellEffect.DelayedExplosion` 的 stage 记账：C# 只在 `stage > 已存在.stage` 时替换
    /// （`GameScene.cs:4867-4878`），重复 stage 的包不再重启动画。
    pub delayed_stage: std::collections::HashMap<u32, u32>,
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

/// 对象特效实体（`S.ObjectEffect` → 原版 `GameScene.ObjectEffect` 的帧动画）。
///
/// `pub(crate)`：只读探针 `spell_fx_probe` 要读它，让「实机看到的到底是什么」可取证
/// （此前这类表现只有单元测试钉表，没有实机判据）。
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct ObjectFxAnim {
    pub(crate) lib: crate::game::spell_effects::FxLib,
    /// 当前这次播放的起始帧（已含 effect_type/朝向/随机步进）
    pub(crate) base: usize,
    pub(crate) frames: usize,
    /// 动画自身时间（秒）——用于取帧
    pub(crate) t: f32,
    /// 自生成以来的总时间（秒）——用于 `Repeat = p.Time > 0` 的截止
    pub(crate) age: f32,
    pub(crate) dur: f32,
    pub(crate) frame_ms: f32,
    /// 跟随的目标对象（0 = 不跟随：`FxTarget::OwnerLocation` 挂在地图位置上）
    pub(crate) follow_object_id: u32,
    /// 该实体代表的对象特效名（探针/诊断用，`SpellEffect` 枚举名）
    pub(crate) name: &'static str,
    pub(crate) repeat: crate::game::spell_effects::FxRepeat,
    /// `PacketTime`：`p.Time/1000`（>0 才循环，循环到这个时长结束）
    hold_secs: f32,
    /// 延迟开播剩余（秒）：原版 `StartTime = CMain.Time + p.DelayTime`，未到就不绘制
    delay_left: f32,
    /// 当前 stage（`DelayedExplosion`：`stage != 2` 才循环）
    stage: u32,
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
                advance_object_fx,
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
    time: Res<Time>,
    actors: Query<(&NetObjectId, &Transform)>,
    players: Query<&Transform, (With<LocalPlayer>, With<NetObjectId>)>,
    // 已存活的对象特效实体：光环 Up/Down 要清同组、DelayedExplosion 换 stage 要先移除旧实体
    object_fx_q: Query<(Entity, &ObjectFxAnim)>,
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
            PendingEffect::Projectile {
                target_id,
                color,
                fx,
            } => {
                let Some((_, tf)) = actors.iter().find(|(id, _)| id.0 == target_id) else {
                    continue;
                };
                let to = Vec2::new(tf.translation.x, tf.translation.y);
                // 远程攻击（S.RangeAttack）：有原版帧表就播帧动画弹道
                let frame_missile_spawned = match fx {
                    Some(m) => spawn_frame_missile(
                        &mut commands,
                        &mut libs,
                        &mut images,
                        &mut cache,
                        m,
                        player_pos,
                        to,
                    ),
                    None => false,
                };
                if !frame_missile_spawned {
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
            }
            PendingEffect::ProjectileFromTo {
                source_id,
                destination_id,
                color,
                fx,
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
                // 其他对象的远程攻击（S.ObjectRangeAttack）：同样优先用原版帧表
                let frame_missile_spawned = match fx {
                    Some(m) => spawn_frame_missile(
                        &mut commands,
                        &mut libs,
                        &mut images,
                        &mut cache,
                        m,
                        from,
                        to,
                    ),
                    None => false,
                };
                if !frame_missile_spawned {
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
                        spawn_frame_missile(
                            &mut commands,
                            &mut libs,
                            &mut images,
                            &mut cache,
                            m,
                            from,
                            to,
                        );
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
            PendingEffect::ObjectEffect {
                object_id,
                effect,
                effect_type,
                time: hold_ms,
                delay_ms,
            } => {
                use crate::game::spell_effects as fx;
                // 原版同样先 `MapControl.Objects.TryGetValue(p.ObjectID, out var ob)`：
                // 对象不在就整包丢弃（连光环清理也不用做）。
                let Some((_, tf)) = actors.iter().find(|(id, _)| id.0 == object_id) else {
                    continue;
                };
                let owner_pos = Vec2::new(tf.translation.x, tf.translation.y);
                let effect_name = mir2_shared::enums::SpellEffect::try_from(effect)
                    .map(|e| format!("{e:?}"))
                    .unwrap_or_default();
                let entry = fx::object_fx_entry(&effect_name);
                // 光环 Up/Down 都先清同组（原版两处都 Clear+Remove，否则会叠两层）
                if let Some(group) = fx::aura_group(&effect_name) {
                    for (ent, a) in &object_fx_q {
                        if a.repeat == fx::FxRepeat::UntilDown(group)
                            && a.follow_object_id == object_id
                        {
                            commands.entity(ent).despawn();
                        }
                    }
                }
                let Some((key, list)) = entry else {
                    // C# 没有这个 case → 保留旧的占位表现（不静默）
                    debug!("对象特效表未覆盖 effect={effect_name}（退回占位表现）");
                    let c = spell_effect_color(effect);
                    commands.spawn((
                        Sprite {
                            image: white.clone(),
                            color: Color::srgba(c[0], c[1], c[2], 0.9),
                            custom_size: Some(Vec2::splat(24.0)),
                            ..default()
                        },
                        Transform::from_xyz(owner_pos.x, owner_pos.y, 21.0),
                        Burst {
                            t: 0.0,
                            dur: 0.35,
                            start_scale: 0.6,
                        },
                    ));
                    continue;
                };
                if list.is_empty() {
                    // C# 明确不画（`Critical` 被注释掉、`MagicShieldDown` 只做清理）
                    continue;
                }
                // DelayedExplosion：C# 只在 `stage > 已存在.stage` 时替换，重复 stage 不重启动画
                if list.iter().any(|f| f.repeat == fx::FxRepeat::StageNot2) {
                    if matches!(state.delayed_stage.get(&object_id), Some(p) if effect_type <= *p) {
                        continue;
                    }
                    state.delayed_stage.insert(object_id, effect_type);
                    for (ent, a) in &object_fx_q {
                        if a.name == key && a.follow_object_id == object_id {
                            commands.entity(ent).despawn();
                        }
                    }
                }
                // 只有 `DeathCrawlerBreath` 用朝向取帧段（`272 + Direction * 4`）；本端 actor
                // 侧没有可读的朝向组件（残留，见 PR），其余 case 的 dir_step 都是 0。
                let dir: u8 = 0;
                let rand = (time.elapsed_secs() * 1000.0) as u32 ^ object_id;
                for f in list {
                    if !f.when.matches(effect_type) {
                        continue;
                    }
                    let target_id = match f.target {
                        fx::FxTarget::Owner | fx::FxTarget::OwnerLocation => object_id,
                        fx::FxTarget::EffectType => effect_type,
                    };
                    let Some((_, tf)) = actors.iter().find(|(id, _)| id.0 == target_id) else {
                        continue;
                    };
                    let base = f.base_frame(effect_type, dir, rand);
                    let (dur, frame_ms) = f.timing();
                    let Some(handle) =
                        object_fx_handle(&mut libs, &mut images, &mut cache, f.lib, base)
                    else {
                        continue;
                    };
                    commands.spawn((
                        ObjectFxAnim {
                            lib: f.lib,
                            base,
                            frames: f.frames,
                            t: 0.0,
                            age: 0.0,
                            dur,
                            frame_ms,
                            follow_object_id: match f.target {
                                fx::FxTarget::OwnerLocation => 0,
                                _ => target_id,
                            },
                            name: key,
                            repeat: f.repeat,
                            hold_secs: hold_ms as f32 / 1000.0,
                            delay_left: if f.delay_from_packet {
                                delay_ms as f32 / 1000.0
                            } else {
                                0.0
                            },
                            stage: effect_type,
                        },
                        Sprite {
                            image: handle,
                            ..default()
                        },
                        bevy::sprite::Anchor::CENTER,
                        Transform::from_xyz(tf.translation.x, tf.translation.y, 21.0),
                    ));
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

/// 对象特效取一帧图：扁平库走 `ui_image`，怪物库走 `ui_array_image`
fn object_fx_handle(
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    cache: &mut UiImageCache,
    lib: crate::game::spell_effects::FxLib,
    index: usize,
) -> Option<Handle<Image>> {
    match lib {
        crate::game::spell_effects::FxLib::Flat(name) => {
            ui_image(libs, images, cache, name.library(), index)
        }
        crate::game::spell_effects::FxLib::Monster(m) => ui_array_image(
            libs,
            images,
            cache,
            ArrayLibType::Monsters,
            m as usize,
            index,
        ),
    }
}

/// 对象特效推进：取帧 + 跟随 + 按 `FxRepeat` 循环或消失。
///
/// 原版 `Effect` 的三种循环在这里一一对应：
/// - `Repeat = true`（护盾 / 元素屏障）→ 循环到收到 Down 包（由 spawn 侧清理）；
/// - `Repeat = p.Time > 0`（Stunned / FlamingMutantWeb）→ 循环满 `p.Time` 才消失，否则播一遍；
/// - `DelayedExplosionEffect` 的 `stage != 2` → 循环，stage=2 播完即消失。
fn advance_object_fx(
    time: Res<Time>,
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    // B0001 回归：下面 q 写 Transform，actors 只读 Transform，`Without<ObjectFxAnim>`
    // 划界证明两个查询不相交（与 advance_spell_fx 同一处置）。
    actors: Query<(&NetObjectId, &Transform), Without<ObjectFxAnim>>,
    mut q: Query<(Entity, &mut ObjectFxAnim, &mut Sprite, &mut Transform)>,
) {
    for (e, mut fx, mut sprite, mut tf) in &mut q {
        let dt = time.delta_secs();
        if fx.delay_left > 0.0 {
            // 原版 `Effect.Draw` 在 `CMain.Time < StartTime` 时直接 return（不绘制）
            fx.delay_left -= dt;
            if sprite.color.alpha() != 0.0 {
                sprite.color.set_alpha(0.0);
            }
            continue;
        }
        if sprite.color.alpha() == 0.0 {
            sprite.color.set_alpha(1.0);
        }
        fx.t += dt;
        fx.age += dt;
        if fx.follow_object_id != 0 {
            if let Some((_, atf)) = actors.iter().find(|(id, _)| id.0 == fx.follow_object_id) {
                tf.translation.x = atf.translation.x;
                tf.translation.y = atf.translation.y;
            }
        }
        use crate::game::spell_effects::FxRepeat;
        let looping = match fx.repeat {
            FxRepeat::Once => false,
            FxRepeat::PacketTime => fx.hold_secs > 0.0,
            FxRepeat::UntilDown(_) | FxRepeat::StageNot2 => true,
        };
        let finished = match fx.repeat {
            FxRepeat::PacketTime if fx.hold_secs > 0.0 => fx.age >= fx.hold_secs,
            FxRepeat::StageNot2 => fx.stage >= 2 && fx.t >= fx.dur,
            FxRepeat::UntilDown(_) => false,
            _ => fx.t >= fx.dur,
        };
        if finished {
            commands.entity(e).despawn();
            continue;
        }
        let step = (fx.t / fx.frame_ms.max(0.001)).floor() as usize;
        let frame = if looping {
            step % fx.frames.max(1)
        } else {
            step.min(fx.frames.saturating_sub(1))
        };
        if let Some(h) =
            object_fx_handle(&mut libs, &mut images, &mut cache, fx.lib, fx.base + frame)
        {
            sprite.image = h;
        }
    }
}

/// 施法弹道实体：一边飞一边按原版帧表循环播（C# Missile 的帧循环）
/// （`pub(crate)`：`control.rs` 的只读探针 `spell_fx_probe` 要读它——owner 反馈「魔法效果完全不对」
/// 的修复此前只有单元测试钉表，没有实机判据）
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct SpellMissileAnim {
    pub(crate) library: crate::game::spell_effects::SpellFxLibrary,
    pub(crate) base: usize,
    pub(crate) frames: usize,
    pub(crate) frame_ms: f32,
    from: Vec2,
    to: Vec2,
    t: f32,
    dur: f32,
}

/// 生成一条「按原版帧表播」的弹道实体（施法弹道与**远程攻击箭矢**共用）。
/// 返回 `false` = 首帧取不到图（调用方自行决定是否退回占位表现）。
fn spawn_frame_missile(
    commands: &mut Commands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    cache: &mut UiImageCache,
    m: crate::game::spell_effects::MissileFx,
    from: Vec2,
    to: Vec2,
) -> bool {
    let Some(handle) = ui_image(libs, images, cache, m.library.library(), m.base) else {
        return false;
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
    true
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
        // `spawn_pending_effects` 现在还要 Res<Time>（对象特效的随机档种子）
        world.insert_resource(bevy::prelude::Time::<()>::default());
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

    /// 门禁（接线，不只是表）：`S.RangeAttack` 的弹道必须用**原版远程攻击帧表**
    /// （`Client/MirObjects/PlayerObject.cs` MirAction.AttackRange1/2/3 的 CreateProjectile），
    /// 不再是染色方块 —— 玩家看到的弓/箭矢才是原版那 5 帧。
    ///
    /// 阳性对照（落地时实做）：把 `handle_npc_items.rs` 里 `S.RangeAttack` 的 `fx` 传成 `None`
    /// （或删掉 `spawn_frame_missile` 调用）→ 本测试立即红（弹道实体数为 0，只剩占位 Projectile）。
    #[test]
    fn range_attack_spawns_original_arrow_frames() {
        use bevy::ecs::system::RunSystemOnce;
        if !crate::resources::libraries::data_assets_present() {
            eprintln!("skip range_attack_spawns_original_arrow_frames: 无 Data 资产");
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
        world.insert_resource(bevy::prelude::Time::<()>::default());
        world
            .resource_mut::<crate::map_renderer::GameLibraries>()
            .0
            .ensure_initialized();
        world.insert_resource(bevy::prelude::Messages::<PendingEffect>::default());
        world.spawn((
            NetObjectId(4242),
            bevy::prelude::Transform::from_xyz(100.0, 200.0, 0.0),
        ));
        world.spawn((
            NetObjectId(4243),
            bevy::prelude::Transform::from_xyz(300.0, 200.0, 0.0),
        ));
        {
            let mut msgs = world.resource_mut::<bevy::prelude::Messages<PendingEffect>>();
            // 普通弓射（spell = 0）→ DefaultArrow；技能用 StraightShot 再验一条
            msgs.write(crate::game::effects::range_attack_projectile(4243, 0));
            msgs.write(crate::game::effects::range_attack_projectile(
                4243,
                mir2_shared::enums::Spell::StraightShot as u8,
            ));
        }
        world
            .run_system_once(spawn_pending_effects)
            .expect("spawn_pending_effects 应能运行");
        let mut mq = world.query::<&SpellMissileAnim>();
        let missiles: Vec<&SpellMissileAnim> = mq.iter(&world).collect();
        assert_eq!(
            missiles.len(),
            2,
            "普通弓射 + StraightShot 各要生成一条箭矢帧动画实体（不是占位方块）"
        );
        let bases: Vec<usize> = missiles.iter().map(|m| m.base).collect();
        assert!(
            bases.contains(&1030),
            "普通弓射必须是 Magic3[1030]（AttackRange1 的 case 5），实得 {bases:?}"
        );
        assert!(
            bases.contains(&1210),
            "StraightShot 必须是 Magic3[1210]（AttackRange2），实得 {bases:?}"
        );
        for m in &missiles {
            assert_eq!(
                m.library,
                SpellFxLibrary::Magic3,
                "远程攻击箭矢都在 Magic3 库"
            );
        }
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
                advance_object_fx,
            )
                .chain(),
        );
        // 首帧即完成调度初始化：有 B0001 冲突时这里直接 panic
        app.update();
        app.update();
    }

    /// 对象特效接线用的最小 World（与上面两条接线门禁同构）：
    /// 真实 Data 资产 + 惰性库初始化 + 对象 4242。无资产时返回 None（CI 上跳过）。
    fn object_fx_test_world() -> Option<bevy::prelude::World> {
        if !crate::resources::libraries::data_assets_present() {
            return None;
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
        world.insert_resource(bevy::prelude::Time::<()>::default());
        world
            .resource_mut::<crate::map_renderer::GameLibraries>()
            .0
            .ensure_initialized();
        world.insert_resource(bevy::prelude::Messages::<PendingEffect>::default());
        world.spawn((
            NetObjectId(4242),
            bevy::prelude::Transform::from_xyz(100.0, 200.0, 0.0),
        ));
        Some(world)
    }

    /// 给上面的 world 写一条 `S.ObjectEffect` 并跑一次生成系统（跑完 Bevy 会 flush 命令）。
    fn write_object_effect(
        world: &mut bevy::prelude::World,
        effect: mir2_shared::enums::SpellEffect,
        effect_type: u32,
        time: u32,
    ) {
        use bevy::ecs::system::RunSystemOnce;
        world
            .resource_mut::<bevy::prelude::Messages<PendingEffect>>()
            .clear();
        world
            .resource_mut::<bevy::prelude::Messages<PendingEffect>>()
            .write(PendingEffect::ObjectEffect {
                object_id: 4242,
                effect: effect as u8,
                effect_type,
                time,
                delay_ms: 0,
            });
        world
            .run_system_once(spawn_pending_effects)
            .expect("spawn_pending_effects 应能运行");
    }

    /// 门禁（接线，不只是表）：`S.ObjectEffect` 必须画出**原版真帧动画**，而不是染色方块。
    ///
    /// 为什么要有这条：`S.ObjectEffect`（护盾/传送/治疗/冰柱/天罚/觉醒…）此前在
    /// `handle_social.rs` 里只发一个 `PendingEffect::Burst`，玩家看到的是一个纯色方块 ——
    /// 表再对，只要这条接线不在，实机就还是方块。
    ///
    /// 阳性对照（实做）：把 `spawn_pending_effects` 的 ObjectEffect 分支改回
    /// `PendingEffect::Burst` 那条路径 → 本测试立即红（`ObjectFxAnim` 数为 0）。
    #[test]
    fn object_effect_spawns_original_frame_animation_not_color_block() {
        let Some(mut world) = object_fx_test_world() else {
            eprintln!(
                "skip object_effect_spawns_original_frame_animation_not_color_block: 无 Data 资产"
            );
            return;
        };
        write_object_effect(&mut world, mir2_shared::enums::SpellEffect::Teleport, 0, 0);
        let mut q = world.query::<&ObjectFxAnim>();
        let fx: Vec<&ObjectFxAnim> = q.iter(&world).collect();
        assert_eq!(
            fx.len(),
            1,
            "Teleport 必须生成一条真帧动画实体（此前是染色方块）"
        );
        assert_eq!(fx[0].name, "Teleport");
        assert_eq!(fx[0].base, 1600, "原版 `Libraries.Magic[1600]`");
        assert_eq!(fx[0].frames, 10);
        assert_eq!(fx[0].follow_object_id, 4242, "跟随包里的对象");
        let mut bq = world.query::<&Burst>();
        assert_eq!(
            bq.iter(&world).count(),
            0,
            "表里有的对象特效不应再退回占位染色方块"
        );
    }

    /// 门禁：护盾光环的 Up 会**先清同组再生成**（原版 `ShieldEffect.Clear(); Remove();`），
    /// Down 只清理不画（表里是空切片）——否则护盾会叠成两层。
    #[test]
    fn magic_shield_aura_up_clears_group_and_down_removes_it() {
        let Some(mut world) = object_fx_test_world() else {
            eprintln!("skip magic_shield_aura_up_clears_group_and_down_removes_it: 无 Data 资产");
            return;
        };
        write_object_effect(
            &mut world,
            mir2_shared::enums::SpellEffect::MagicShieldUp,
            0,
            0,
        );
        let count_aura = |world: &mut bevy::prelude::World| {
            let mut q = world.query::<&ObjectFxAnim>();
            q.iter(world)
                .filter(|f| {
                    f.repeat
                        == crate::game::spell_effects::FxRepeat::UntilDown(
                            crate::game::spell_effects::AuraGroup::MagicShield,
                        )
                })
                .count()
        };
        assert_eq!(
            count_aura(&mut world),
            1,
            "第一次 MagicShieldUp 生成一条光环"
        );
        // 第二次 Up（重复施放）——必须先清掉旧的，不能变两条
        write_object_effect(
            &mut world,
            mir2_shared::enums::SpellEffect::MagicShieldUp,
            0,
            0,
        );
        assert_eq!(
            count_aura(&mut world),
            1,
            "重复 Up 必须先清同组（原版 Clear+Remove），否则护盾叠两层"
        );
        // Down：C# 只做清理，不画（表里是 `Some(&[])`）
        write_object_effect(
            &mut world,
            mir2_shared::enums::SpellEffect::MagicShieldDown,
            0,
            0,
        );
        assert_eq!(count_aura(&mut world), 0, "MagicShieldDown 必须清掉光环");
    }

    /// 门禁：MPEater 的两条里，第二条打的是 `p.EffectType` 指的那个**对象**（原版 `ob2`），
    /// 不是施法者自己 —— 这条最容易在移植时被写成「都挂自己身上」。
    #[test]
    fn mpeater_second_effect_targets_effect_type_object() {
        let Some(mut world) = object_fx_test_world() else {
            eprintln!("skip mpeater_second_effect_targets_effect_type_object: 无 Data 资产");
            return;
        };
        world.spawn((
            NetObjectId(7777),
            bevy::prelude::Transform::from_xyz(500.0, 600.0, 0.0),
        ));
        write_object_effect(
            &mut world,
            mir2_shared::enums::SpellEffect::MPEater,
            7777,
            0,
        );
        let mut q = world.query::<&ObjectFxAnim>();
        let mut follows: Vec<u32> = q.iter(&world).map(|f| f.follow_object_id).collect();
        follows.sort_unstable();
        assert_eq!(
            follows,
            vec![4242, 7777],
            "MPEater 两条：一条挂施法者、一条挂 EffectType 指的对象"
        );
    }

    /// 门禁：表里**没有**的 case（C# 也没有，如 `KingGuard2`）才允许退回占位表现，
    /// 且不静默（走 debug）。这条守住「不为了好看把没实现的也画成方块」的边界。
    #[test]
    fn uncovered_object_effect_keeps_placeholder_fallback() {
        let Some(mut world) = object_fx_test_world() else {
            eprintln!("skip uncovered_object_effect_keeps_placeholder_fallback: 无 Data 资产");
            return;
        };
        write_object_effect(
            &mut world,
            mir2_shared::enums::SpellEffect::KingGuard2,
            0,
            0,
        );
        let mut q = world.query::<&ObjectFxAnim>();
        assert_eq!(q.iter(&world).count(), 0, "C# 没有 case → 不该画帧动画");
        let mut bq = world.query::<&Burst>();
        assert_eq!(bq.iter(&world).count(), 1, "未覆盖的特效保留占位表现");
    }

    /// 门禁：`DelayedExplosion` 只在 stage **变大**时替换（原版 `stage < p.EffectType` 才 Remove+Add），
    /// 重复同 stage 的包不能把动画重启。
    #[test]
    fn delayed_explosion_replaces_only_on_stage_increase() {
        let Some(mut world) = object_fx_test_world() else {
            eprintln!("skip delayed_explosion_replaces_only_on_stage_increase: 无 Data 资产");
            return;
        };
        let count = |world: &mut bevy::prelude::World| {
            let mut q = world.query::<&ObjectFxAnim>();
            q.iter(world).count()
        };
        write_object_effect(
            &mut world,
            mir2_shared::enums::SpellEffect::DelayedExplosion,
            0,
            0,
        );
        assert_eq!(count(&mut world), 1, "stage 0 生成一条");
        write_object_effect(
            &mut world,
            mir2_shared::enums::SpellEffect::DelayedExplosion,
            0,
            0,
        );
        assert_eq!(count(&mut world), 1, "同一 stage 重复到达不额外生成");
        write_object_effect(
            &mut world,
            mir2_shared::enums::SpellEffect::DelayedExplosion,
            1,
            0,
        );
        assert_eq!(
            count(&mut world),
            1,
            "stage 变大 → 替换（旧的移除、新的生成）"
        );
    }
}
