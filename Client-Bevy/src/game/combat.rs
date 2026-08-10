// ============================================================================
// 战斗反馈（M10）
// 网络驱动：ObjectStruck（受击动画）/ ObjectDied（死亡）/ DamageIndicator（伤害飘字）
// 参考：Client-Macroquad/src/systems/logic/combat/* + presentation/floating_text_system.rs
// ============================================================================

use bevy::prelude::*;

use crate::actor::{ActorAnim, MonsterAppearance, NetObjectId};
use crate::game::hud::HudState;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiFont;
use bevy::sprite::Anchor;

/// 服务器战斗事件（网络 handler 发送，战斗系统消费）
#[derive(Message, Debug, Clone, Copy)]
pub enum CombatEvent {
    /// 对象受击（S.ObjectStruck：怪物/NPC 受击动画；#1568 带攻击者用于受击音）
    Struck { object_id: u32, attacker_id: u32, direction: u8 },
    /// S.Struck：本地玩家被击中（C# User.Struck 受击动画）
    PlayerStruck,
    /// S.ObjectHealth：对象血量百分比（C# 头顶血条）
    ObjectHealth { object_id: u32, percent: u8, expire: u16 },
    Died { object_id: u32, death_type: u8 },
    Revived { object_id: u32 },
    Damage { object_id: u32, damage: i32, dmg_type: u8 },
    /// #224 对象施法（S.ObjectMagic）：施法者播 Spell 动作
    SpellCast { object_id: u32 },
    /// #224 对象远程攻击（S.ObjectRangeAttack）：施法者播 AttackRange 动作
    RangeAttack { object_id: u32 },
    /// #234/#1624 对象近战攻击（S.ObjectAttack）：施法者按 attack_type 播 Attack1-5 动作
    Attack { object_id: u32, direction: u8, attack_type: u8 },
    /// #238 对象蓝量（S.ObjectMana）
    ObjectMana { object_id: u32, percent: u8 },
    /// #246 采集（S.ObjectHarvest/ObjectHarvested）：目标播 Harvest 动作
    Harvest { object_id: u32, direction: u8 },
}

/// 真实服务器命中探测（#57）：DamageIndicator（非本地玩家）计数，
/// 供 --real-verify 判断攻击是否命中（远程怪够不着时无增长 → 换目标）
#[derive(Resource, Default)]
pub struct RealHitProbe {
    pub hits: u32,
}

/// 伤害飘字
#[derive(Component)]
pub struct DamageText {
    pub vy: f32,
    pub life: f32,
}

/// 受击动画计时（结束后回站立）
#[derive(Component)]
pub struct StruckTimer(pub f32);

/// 对象血量（C# S.ObjectHealth：percent + expire 秒）
#[derive(Component)]
pub struct ActorHp {
    pub percent: u8,
    pub expire: f32,
}

/// 已生成头顶血条的父实体标记
#[derive(Component)]
pub struct ActorHpBar;

/// 头顶血条背景（子实体）
#[derive(Component)]
pub struct HpBarBg;

/// 头顶血条红色填充（子实体）
#[derive(Component)]
pub struct HpBarFill;

/// #238 对象蓝量（C# S.ObjectMana：percent + 刷新式生命周期）
#[derive(Component)]
pub struct ActorMp {
    pub percent: u8,
    pub expire: f32,
}

/// 已生成头顶蓝条的父实体标记
#[derive(Component)]
pub struct ActorMpBar;

/// 头顶蓝条背景（子实体）
#[derive(Component)]
pub struct MpBarBg;

/// 头顶蓝条蓝色填充（子实体）
#[derive(Component)]
pub struct MpBarFill;

/// 死亡移除计时
#[derive(Component)]
pub struct DeathTimer(pub f32);

/// 攻击模式（C# User.AttackMode；Ctrl+H 循环切换）
#[derive(Resource)]
pub struct AttackModeState {
    pub mode: mir2_shared::enums::AttackMode,
}

impl Default for AttackModeState {
    fn default() -> Self {
        Self {
            mode: mir2_shared::enums::AttackMode::Peace,
        }
    }
}

/// 攻击模式中文名
pub fn attack_mode_name(mode: mir2_shared::enums::AttackMode) -> &'static str {
    match mode {
        mir2_shared::enums::AttackMode::Peace => "和平",
        mir2_shared::enums::AttackMode::Group => "组队",
        mir2_shared::enums::AttackMode::Guild => "行会",
        mir2_shared::enums::AttackMode::EnemyGuild => "敌会",
        mir2_shared::enums::AttackMode::RedBrown => "红名",
        mir2_shared::enums::AttackMode::All => "全体",
    }
}

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RealHitProbe>();
        app.init_resource::<AttackModeState>();
        app.add_message::<CombatEvent>();
        app.add_systems(
            Update,
            attack_mode_system.run_if(in_state(AppState::Game)),
        );
        app.add_systems(
            Update,
            attack_mode_server_events.run_if(in_state(AppState::Game)),
        );
        // #234 修复：战斗反馈系统此前未注册（受击动画/伤害飘字/头顶血条/死亡移除从未生效）
        app.add_systems(
            Update,
            (
                apply_combat_events,
                advance_combat_timers,
                advance_damage_texts,
                actor_hp_bar_system,
                actor_mp_bar_system,
            )
                .chain()
                .after(crate::network::network_system)
                .run_if(in_state(AppState::Game)),
        );
    }
}

/// Ctrl+H 循环切换攻击模式（#156 C# KeybindOptions.ChangeAttackmode）
fn attack_mode_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<AttackModeState>,
    net: Res<crate::network::NetConnection>,
) {
    if !(keys.just_pressed(KeyCode::KeyH)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight)))
    {
        return;
    }
    use mir2_shared::enums::AttackMode;
    let next = match state.mode {
        AttackMode::Peace => AttackMode::Group,
        AttackMode::Group => AttackMode::Guild,
        AttackMode::Guild => AttackMode::All,
        _ => AttackMode::Peace,
    };
    state.mode = next;
    net.send_packet(&mir2_shared::packets::client::misc::ChangeAMode { mode: next });
    tracing::info!("⚔️ 攻击模式 -> {:?}（{}）", next, attack_mode_name(next));
}

/// 应用受击/死亡事件 + 生成伤害飘字
fn apply_combat_events(
    mut commands: Commands,
    hud: Res<HudState>,
    mut probe: ResMut<RealHitProbe>,
    ui_font: Res<UiFont>,
    sound_bank: Res<crate::game::sound::SoundBank>,
    mut audio_assets: ResMut<Assets<AudioSource>>,
    mut events: MessageReader<CombatEvent>,
    mut actors: Query<(
        Entity,
        &NetObjectId,
        &mut ActorAnim,
        Option<&crate::actor::Monster>,
        Option<&MonsterAppearance>,
    )>,
) {
    for ev in events.read() {
        match ev {
            CombatEvent::Struck { object_id, attacker_id, direction } => {
                // #1568：C# MonsterObject.PlayStruckSound——本地玩家攻击时按自己武器播受击音
                let struck_sound = if hud.player_object_id == Some(*attacker_id) {
                    crate::game::sound::monster_struck_sound(
                        hud.equipment.get(0).and_then(|s| s.as_ref()).map(|i| i.shape).unwrap_or(-1),
                    )
                } else {
                    Some(10060) // 默认 StruckShort（非本地玩家攻击者武器未知）
                };
                if let Some(sound_id) = struck_sound {
                    crate::game::sound::play_sound(&mut commands, &mut audio_assets, &sound_bank, sound_id);
                }
                for (e, id, mut anim, mon, appr) in &mut actors {
                    if id.0 == *object_id {
                        anim.action = mir2_shared::enums::MirAction::Attack1;
                        anim.direction = *direction;
                        anim.frame_index = 0;
                        commands.entity(e).insert(StruckTimer(0.6));
                        // #1627：C# MirAction.Struck → PlayFlinchSound（BaseSound+2，MonsterObject.cs:1064）
                        // 注：怪物攻击音由 Attack 事件（#1624）在动作起始播放，此处不播
                        if mon.is_some() {
                            if let Some(appr) = appr {
                                crate::game::sound::play_sound(
                                    &mut commands,
                                    &mut audio_assets,
                                    &sound_bank,
                                    crate::game::sound::monster_flinch_sound(appr.monster_type),
                                );
                            }
                        }
                        break;
                    }
                }
            }
            CombatEvent::PlayerStruck => {
                // C# S.Struck：本地玩家受击动画 + 音效（#1564：性别 flinch，C# PlayFlinchSound）
                crate::game::sound::play_sound(
                    &mut commands,
                    &mut audio_assets,
                    &sound_bank,
                    crate::game::sound::player_flinch_sound(hud.gender),
                );
                for (e, id, mut anim, _mon, _appr) in &mut actors {
                    if hud.player_object_id == Some(id.0) {
                        anim.action = mir2_shared::enums::MirAction::Struck;
                        anim.frame_index = 0;
                        commands.entity(e).insert(StruckTimer(0.6));
                        break;
                    }
                }
            }
            CombatEvent::ObjectHealth { object_id, percent, expire } => {
                // C# S.ObjectHealth：挂载血量（血条系统渲染/过期）
                for (e, id, _, _, _) in &mut actors {
                    if id.0 == *object_id {
                        commands.entity(e).insert(ActorHp {
                            percent: *percent,
                            expire: *expire as f32,
                        });
                        break;
                    }
                }
            }
            CombatEvent::Harvest {
                object_id,
                direction,
            } => {
                // #246：采集动作——玩家/NPC 用 Harvest 帧（344），默认怪物回退 Attack1
                for (e, id, mut anim, mon, _appr) in &mut actors {
                    if id.0 == *object_id {
                        anim.action = if mon.is_some() {
                            mir2_shared::enums::MirAction::Attack1
                        } else {
                            mir2_shared::enums::MirAction::Harvest
                        };
                        anim.direction = *direction;
                        anim.frame_index = 0;
                        commands.entity(e).insert(StruckTimer(0.6));
                        break;
                    }
                }
            }
            CombatEvent::ObjectMana { object_id, percent } => {
                // #238：更新/插入对象蓝条（刷新式 15s 生命周期）
                for (e, id, _, _, _) in &mut actors {
                    if id.0 == *object_id {
                        commands.entity(e).insert(ActorMp {
                            percent: *percent,
                            expire: 15.0,
                        });
                        break;
                    }
                }
            }
            CombatEvent::Died { object_id, .. } => {
                // #1564：本地玩家死亡音（C# PlayDieSound 按性别）
                if hud.player_object_id == Some(*object_id) {
                    crate::game::sound::play_sound(
                        &mut commands,
                        &mut audio_assets,
                        &sound_bank,
                        crate::game::sound::player_die_sound(hud.gender),
                    );
                }
                for (e, id, mut anim, mon, appr) in &mut actors {
                    if id.0 == *object_id {
                        anim.action = mir2_shared::enums::MirAction::Dead;
                        anim.frame_index = 0;
                        // 本地玩家死亡由 Death 包管理（复活时恢复），不自动 despawn
                        if hud.player_object_id != Some(*object_id) {
                            commands.entity(e).insert(DeathTimer(3.0));
                            // #1570：怪物死亡音（C# PlayDieSound → BaseSound+3；本地玩家走性别死亡音）
                            if mon.is_some() {
                                if let Some(appr) = appr {
                                    crate::game::sound::play_sound(
                                        &mut commands,
                                        &mut audio_assets,
                                        &sound_bank,
                                        crate::game::sound::monster_die_sound(appr.monster_type),
                                    );
                                }
                            }
                        }
                        break;
                    }
                }
            }
            CombatEvent::SpellCast { object_id } => {
                // #224：施法动作——玩家用 Spell 帧（C# Action.Spell），
                // 默认怪物无 Spell 帧表 → 回退 Attack1（避免动画冻结）
                for (e, id, mut anim, mon, _appr) in &mut actors {
                    if id.0 == *object_id {
                        anim.action = if mon.is_some() {
                            mir2_shared::enums::MirAction::Attack1
                        } else {
                            mir2_shared::enums::MirAction::Spell
                        };
                        anim.frame_index = 0;
                        commands.entity(e).insert(StruckTimer(0.6));
                        break;
                    }
                }
            }
            CombatEvent::Attack {
                object_id,
                direction,
                attack_type,
            } => {
                // #234：对象近战攻击（玩家固定 Attack1；怪物按 attack_type 0-4 → Attack1-5，C# GameScene.cs:3347）
                tracing::debug!("⚔️ [ATTACK] 处理攻击 id={} type={}", object_id, attack_type);
                let action = match attack_type {
                    1 => mir2_shared::enums::MirAction::Attack2,
                    2 => mir2_shared::enums::MirAction::Attack3,
                    3 => mir2_shared::enums::MirAction::Attack4,
                    4 => mir2_shared::enums::MirAction::Attack5,
                    _ => mir2_shared::enums::MirAction::Attack1,
                };
                for (e, id, mut anim, mon, appr) in &mut actors {
                    if id.0 == *object_id {
                        anim.action = if mon.is_some() { action } else { mir2_shared::enums::MirAction::Attack1 };
                        anim.direction = *direction;
                        anim.frame_index = 0;
                        commands.entity(e).insert(StruckTimer(0.6));
                        // #1624：怪物攻击动作起始音（C# SetAction → Play*AttackSound）
                        if mon.is_some() {
                            if let Some(appr) = appr {
                                let sound_id = match action {
                                    mir2_shared::enums::MirAction::Attack2 => {
                                        Some(crate::game::sound::monster_second_attack_sound(appr.monster_type))
                                    }
                                    mir2_shared::enums::MirAction::Attack3 => {
                                        crate::game::sound::monster_third_attack_sound(appr.monster_type)
                                    }
                                    mir2_shared::enums::MirAction::Attack4 => {
                                        crate::game::sound::monster_fourth_attack_sound(appr.monster_type)
                                    }
                                    mir2_shared::enums::MirAction::Attack5 => {
                                        Some(crate::game::sound::monster_fifth_attack_sound(appr.monster_type))
                                    }
                                    _ => Some(crate::game::sound::monster_attack_sound(appr.monster_type)),
                                };
                                if let Some(sound_id) = sound_id {
                                    crate::game::sound::play_sound(
                                        &mut commands,
                                        &mut audio_assets,
                                        &sound_bank,
                                        sound_id,
                                    );
                                }
                            }
                        }
                        break;
                    }
                }
            }
            CombatEvent::RangeAttack { object_id } => {
                // #224：远程攻击动作——玩家用 AttackRange1（C# Action.AttackRange1），
                // 默认怪物无 AttackRange 帧表 → 回退 Attack1
                for (e, id, mut anim, mon, _appr) in &mut actors {
                    if id.0 == *object_id {
                        anim.action = if mon.is_some() {
                            mir2_shared::enums::MirAction::Attack1
                        } else {
                            mir2_shared::enums::MirAction::AttackRange1
                        };
                        anim.frame_index = 0;
                        commands.entity(e).insert(StruckTimer(0.6));
                        break;
                    }
                }
            }
            CombatEvent::Revived { object_id } => {
                // 复活：恢复站立 + 清除死亡计时（本地玩家由 Revived 包驱动）
                for (e, id, mut anim, _mon, _appr) in &mut actors {
                    if id.0 == *object_id {
                        anim.action = mir2_shared::enums::MirAction::Standing;
                        anim.frame_index = 0;
                        commands.entity(e).remove::<DeathTimer>();
                        break;
                    }
                }
            }
            // 伤害飘字（挂到目标实体上自动跟随）
            CombatEvent::Damage { object_id, damage, dmg_type } => {
                // 命中探测：非本地玩家的伤害事件 = 玩家攻击命中目标（#57）
                if hud.player_object_id != Some(*object_id) {
                    probe.hits += 1;
                }
                if !ui_font.0.is_strong() {
                    continue;
                }
                let target_info = actors
                    .iter()
                    .find(|(_, id, _, _, _)| id.0 == *object_id)
                    .map(|(e, _, _, mon, _)| (e, mon.is_some()));
                let Some((target, is_monster)) = target_info else {
                    continue;
                };
                // #1618：C# GameScene 飘字颜色——Miss 灰、暴击深红"暴击"、命中怪白/人红
                let is_miss = *dmg_type == 4;
                let is_crit = *dmg_type == 5;
                let text = if is_miss {
                    "Miss".to_string()
                } else if is_crit {
                    "暴击".to_string()
                } else {
                    format!("-{}", damage)
                };
                let color = if is_miss {
                    Color::srgb(0.83, 0.83, 0.83) // LightGray
                } else if is_crit {
                    Color::srgb(0.55, 0.0, 0.0) // DarkRed
                } else if is_monster {
                    Color::srgb(0.95, 0.95, 0.95) // White（怪物命中）
                } else {
                    Color::srgb(0.9, 0.2, 0.2) // Red（玩家目标）
                };
                let y = if is_crit { -55.0 } else { -40.0 }; // C# Critical Offset=15
                commands.entity(target).with_children(|p| {
                    p.spawn((
                        Text2d::new(text),
                        Anchor::TOP_LEFT,
                        TextColor(color),
                        TextFont {
                            font: FontSource::Handle(ui_font.0.clone()),
                            font_size: FontSize::Px(16.0),
                            ..default()
                        },
                        Transform::from_xyz(0.0, y, 20.0),
                        DamageText { vy: -50.0, life: 1.2 },
                    ));
                });
            }
        }
    }
}

/// 受击/死亡计时
fn advance_combat_timers(
    mut commands: Commands,
    time: Res<Time>,
    mut struck: Query<(Entity, &mut StruckTimer, &mut ActorAnim), Without<DeathTimer>>,
    mut deaths: Query<(Entity, &mut DeathTimer), Without<StruckTimer>>,
) {
    for (e, mut t, mut anim) in &mut struck {
        t.0 -= time.delta_secs();
        if t.0 <= 0.0 {
            anim.action = mir2_shared::enums::MirAction::Standing;
            anim.frame_index = 0;
            commands.entity(e).remove::<StruckTimer>();
        }
    }
    for (e, mut t) in &mut deaths {
        t.0 -= time.delta_secs();
        if t.0 <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}

/// 推进飘字
fn advance_damage_texts(
    mut commands: Commands,
    time: Res<Time>,
    mut texts: Query<(Entity, &mut DamageText, &mut Transform)>,
) {
    for (e, mut dt, mut tf) in &mut texts {
        dt.life -= time.delta_secs();
        tf.translation.y += dt.vy * time.delta_secs();
        if dt.life <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}

/// 对象头顶血条（C# S.ObjectHealth）：生成/更新/过期清除
fn actor_hp_bar_system(
    mut commands: Commands,
    time: Res<Time>,
    mut images: ResMut<Assets<Image>>,
    mut actors: Query<(Entity, &mut ActorHp, Option<&ActorHpBar>)>,
    mut bars: Query<(Entity, &ChildOf, &mut Sprite, &HpBarFill)>,
) {
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    for (e, mut hp, bar) in &mut actors {
        hp.expire -= time.delta_secs();
        if hp.expire <= 0.0 {
            let children: Vec<Entity> = bars
                .iter()
                .filter(|(_, c, _, _)| c.parent() == e)
                .map(|(e2, _, _, _)| e2)
                .collect();
            for c in children {
                commands.entity(c).despawn();
            }
            commands.entity(e).remove::<ActorHp>().remove::<ActorHpBar>();
            continue;
        }
        if bar.is_none() {
            commands.entity(e).insert(ActorHpBar);
            commands.entity(e).with_children(|p| {
                p.spawn((
                    HpBarBg,
                    Sprite {
                        image: white.clone(),
                        color: Color::srgb(0.0, 0.0, 0.0),
                        custom_size: Some(Vec2::new(30.0, 4.0)),
                        ..default()
                    },
                    bevy::sprite::Anchor::TOP_LEFT,
                    Transform::from_xyz(-15.0, 18.0, 0.1),
                ));
                p.spawn((
                    HpBarFill,
                    Sprite {
                        image: white.clone(),
                        color: Color::srgb(0.9, 0.1, 0.1),
                        custom_size: Some(Vec2::new(30.0, 4.0)),
                        ..default()
                    },
                    bevy::sprite::Anchor::TOP_LEFT,
                    Transform::from_xyz(-15.0, 18.0, 0.2),
                ));
            });
        }
        let w = 30.0 * (hp.percent.clamp(1, 99) as f32 / 100.0);
        for (_, c, mut fs, _) in &mut bars {
            if c.parent() == e {
                fs.custom_size = Some(Vec2::new(w, 4.0));
            }
        }
    }
}

/// #238 对象头顶蓝条（C# S.ObjectMana）：生成/更新/过期清除（血条下方 y=22）
fn actor_mp_bar_system(
    mut commands: Commands,
    time: Res<Time>,
    mut images: ResMut<Assets<Image>>,
    mut actors: Query<(Entity, &mut ActorMp, Option<&ActorMpBar>)>,
    mut bars: Query<(Entity, &ChildOf, &mut Sprite, &MpBarFill)>,
) {
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    for (e, mut mp, bar) in &mut actors {
        mp.expire -= time.delta_secs();
        if mp.expire <= 0.0 {
            let children: Vec<Entity> = bars
                .iter()
                .filter(|(_, c, _, _)| c.parent() == e)
                .map(|(e2, _, _, _)| e2)
                .collect();
            for c in children {
                commands.entity(c).despawn();
            }
            commands.entity(e).remove::<ActorMp>().remove::<ActorMpBar>();
            continue;
        }
        if bar.is_none() {
            commands.entity(e).insert(ActorMpBar);
            commands.entity(e).with_children(|p| {
                p.spawn((
                    MpBarBg,
                    Sprite {
                        image: white.clone(),
                        color: Color::srgb(0.0, 0.0, 0.0),
                        custom_size: Some(Vec2::new(30.0, 4.0)),
                        ..default()
                    },
                    bevy::sprite::Anchor::TOP_LEFT,
                    Transform::from_xyz(-15.0, 22.0, 0.1),
                ));
                p.spawn((
                    MpBarFill,
                    Sprite {
                        image: white.clone(),
                        color: Color::srgb(0.1, 0.4, 1.0),
                        custom_size: Some(Vec2::new(30.0, 4.0)),
                        ..default()
                    },
                    bevy::sprite::Anchor::TOP_LEFT,
                    Transform::from_xyz(-15.0, 22.0, 0.2),
                ));
            });
        }
        let w = 30.0 * (mp.percent.clamp(1, 99) as f32 / 100.0);
        for (_, c, mut fs, _) in &mut bars {
            if c.parent() == e {
                fs.custom_size = Some(Vec2::new(w, 4.0));
            }
        }
    }
}

/// 消费 S.ChangeAMode：更新本地攻击模式状态（服务端确认）
fn attack_mode_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut state: ResMut<AttackModeState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::AttackModeChanged { mode } = ev {
            if state.mode != *mode {
                state.mode = *mode;
                tracing::info!("⚔️ 攻击模式（服务端确认）: {:?}", mode);
            }
        }
    }
}
