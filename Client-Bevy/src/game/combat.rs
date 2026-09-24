// ============================================================================
// 战斗反馈（M10）
// 网络驱动：ObjectStruck（受击动画）/ ObjectDied（死亡）/ DamageIndicator（伤害飘字）
// 参考：Client-Macroquad/src/systems/logic/combat/* + presentation/floating_text_system.rs
// ============================================================================

use bevy::prelude::*;

use crate::actor::{ActorAnim, LocalPlayer, MonsterAppearance, NetObjectId};
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiFont;
use bevy::sprite::Anchor;

/// 服务器战斗事件（网络 handler 发送，战斗系统消费）
#[derive(Message, Debug, Clone, Copy)]
pub enum CombatEvent {
    /// 对象受击（S.ObjectStruck：怪物/NPC 受击动画；#1568 带攻击者用于受击音）
    Struck {
        object_id: u32,
        attacker_id: u32,
        direction: u8,
    },
    /// S.Struck：本地玩家被击中（C# User.Struck 受击动画）
    PlayerStruck,
    /// S.ObjectHealth：对象血量百分比（C# 头顶血条）
    ObjectHealth {
        object_id: u32,
        percent: u8,
        expire: u16,
    },
    Died {
        object_id: u32,
        death_type: u8,
    },
    Revived {
        object_id: u32,
    },
    Damage {
        object_id: u32,
        damage: i32,
        dmg_type: u8,
    },
    /// #224 对象施法（S.ObjectMagic）：施法者播 Spell 动作
    SpellCast {
        object_id: u32,
    },
    /// #224/#1765 对象远程攻击（S.ObjectRangeAttack）：施法者播 AttackRange 动作（Type→AttackRange1/2/3）
    RangeAttack {
        object_id: u32,
        attack_type: u8,
    },
    /// #234/#1624 对象近战攻击（S.ObjectAttack）：施法者按 attack_type 播 Attack1-5 动作
    Attack {
        object_id: u32,
        direction: u8,
        attack_type: u8,
    },
    /// #238 对象蓝量（S.ObjectMana）
    ObjectMana {
        object_id: u32,
        percent: u8,
    },
    /// #246 采集（S.ObjectHarvest/ObjectHarvested）：目标播 Harvest 动作
    Harvest {
        object_id: u32,
        direction: u8,
    },
}

/// 真实服务器命中探测（#57）：DamageIndicator（非本地玩家）计数，
/// 供 --real-verify 判断攻击是否命中（远程怪够不着时无增长 → 换目标）
#[derive(Resource, Default)]
pub struct RealHitProbe {
    pub hits: u32,
    /// 已把写入排上队（即真正走过 apply 路径）的战斗事件计数——给实机夹具当**非空转判据**：
    /// `l5s_switch_survives_combat.ps1` 用它断言「Struck 事件真的到达并被处理」，
    /// 否则「事件没来」与「修复有效」在报告里无法区分。
    pub struck_applied: u32,
    pub player_struck_applied: u32,
    pub died_applied: u32,
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
        app.add_systems(Update, attack_mode_system.run_if(in_state(AppState::Game)));
        app.add_systems(
            Update,
            apply_pending_attack_mode.run_if(in_state(AppState::Game)),
        );
        app.add_systems(
            Update,
            record_combat_events.run_if(in_state(AppState::Game)),
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
/// #2595：文本输入聚焦时让路（Ctrl+H 不在 C# ChatTextBox_KeyDown 转发表内，
/// MainDialogs.cs:1160-1185 仅 F1-F12/Tab）
fn attack_mode_system(
    keys: Res<ButtonInput<KeyCode>>,
    gate: Res<crate::game::input_gate::TextInputGate>,
    mut state: ResMut<AttackModeState>,
    net: Res<crate::network::NetConnection>,
) {
    if gate.0 {
        return;
    }
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

/// 玩家验收能力（2026-09-22）：消费 control RPC 的「切换攻击模式」请求。
///
/// 与 Ctrl+H（[`attack_mode_system`]）走同一条出口：写 `AttackModeState` + 发 `ChangeAMode`。
/// 独立成系统而非并入 control 命令循环，是因为 `apply_control_commands` 的参数已达 Bevy 上限（16）。
fn apply_pending_attack_mode(
    mut control: ResMut<crate::game::player_control::ControlState>,
    mut state: ResMut<AttackModeState>,
    net: Res<crate::network::NetConnection>,
) {
    let Some(mode) = control.pending_attack_mode.take() else {
        return;
    };
    state.mode = mode;
    control.last_attack_mode = Some(mode);
    net.send_packet(&mir2_shared::packets::client::misc::ChangeAMode { mode });
    tracing::info!(
        "🎮 control attack_mode -> {:?}（{}）",
        mode,
        attack_mode_name(mode)
    );
}

/// 玩家验收能力（2026-09-22）：把战斗相关事件记进 `ControlState.combat_log`，
/// 供只读 RPC `combat_probe` 读出——判据取自**服务端事件**而不是 `nearby` 的视野成员变化。
/// （`apply_combat_events` 是另一个 reader，Bevy 的消息可被多个 reader 各读一遍，互不影响。）
fn record_combat_events(
    mut control: ResMut<crate::game::player_control::ControlState>,
    mut events: MessageReader<CombatEvent>,
) {
    use crate::game::player_control::{push_combat_log, CombatLogItem};
    for ev in events.read() {
        let item = match ev {
            CombatEvent::Struck {
                object_id,
                attacker_id,
                ..
            } => CombatLogItem {
                kind: "struck",
                object_id: *object_id,
                value: 0,
                actor_id: *attacker_id,
            },
            CombatEvent::ObjectHealth {
                object_id, percent, ..
            } => CombatLogItem {
                kind: "object_health",
                object_id: *object_id,
                value: *percent as i32,
                actor_id: 0,
            },
            CombatEvent::Damage {
                object_id, damage, ..
            } => CombatLogItem {
                kind: "damage",
                object_id: *object_id,
                value: *damage,
                actor_id: 0,
            },
            CombatEvent::Died { object_id, .. } => CombatLogItem {
                kind: "died",
                object_id: *object_id,
                value: 0,
                actor_id: 0,
            },
            CombatEvent::Revived { object_id } => CombatLogItem {
                kind: "revived",
                object_id: *object_id,
                value: 0,
                actor_id: 0,
            },
            CombatEvent::Attack {
                object_id,
                attack_type,
                ..
            } => CombatLogItem {
                kind: "attack",
                object_id: *object_id,
                value: *attack_type as i32,
                actor_id: 0,
            },
            CombatEvent::SpellCast { object_id } => CombatLogItem {
                kind: "spell",
                object_id: *object_id,
                value: 0,
                actor_id: 0,
            },
            _ => continue,
        };
        push_combat_log(&mut control.combat_log, item);
    }
}

/// 应用受击/死亡事件 + 生成伤害飘字
fn apply_combat_events(
    mut commands: Commands,
    // #2633 批次4 步7：本地判定改读 `NetObjectId`/`ActorAppearance`（HudState 已于步9 删除）；
    // 实体缺失 = 非本地/未生成（同原 local_id=None、gender=0 默认）
    local_q: Query<(&NetObjectId, &crate::actor::ActorAppearance), With<LocalPlayer>>,
    loadout_q: Query<&crate::game::player_state::Loadout, With<LocalPlayer>>,
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
    let local = local_q.single().ok();
    let local_id = local.map(|(id, _)| id.0);
    let local_gender = local.map(|(_, a)| a.gender as u8).unwrap_or(0);
    for ev in events.read() {
        match ev {
            CombatEvent::Struck {
                object_id,
                attacker_id,
                direction,
            } => {
                // #1568：C# MonsterObject.PlayStruckSound——本地玩家攻击时按自己武器播受击音
                // （武器 shape 读 `Loadout` 组件，#2633 批次4 步6）
                let weapon_shape = loadout_q
                    .single()
                    .ok()
                    .and_then(|l| l.slots.get(0))
                    .and_then(|s| s.as_ref())
                    .map(|i| i.shape)
                    .unwrap_or(-1);
                let struck_sound = if local_id == Some(*attacker_id) {
                    crate::game::sound::monster_struck_sound(weapon_shape)
                } else {
                    Some(10060) // 默认 StruckShort（非本地玩家攻击者武器未知）
                };
                if let Some(sound_id) = struck_sound {
                    crate::game::sound::play_sound(
                        &mut commands,
                        &mut audio_assets,
                        &sound_bank,
                        sound_id,
                    );
                }
                for (e, id, mut anim, mon, appr) in &mut actors {
                    if id.0 == *object_id {
                        anim.action = mir2_shared::enums::MirAction::Attack1;
                        anim.direction = *direction;
                        anim.frame_index = 0;
                        // #3089（换图崩溃）：命令是延迟落地的——同帧若发生地图重建（换图会把对象实体 despawn+重建）→ 命中失效 Entity 会让 Bevy panic
                        crate::game::movement::safe_insert(&mut commands, e, StruckTimer(0.6));
                        // 非空转判据：写过一次 = 这条事件真的被 apply 处理过（见 RealHitProbe 注释）
                        probe.struck_applied = probe.struck_applied.saturating_add(1);
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
                    crate::game::sound::player_flinch_sound(local_gender),
                );
                for (e, id, mut anim, _mon, _appr) in &mut actors {
                    if local_id == Some(id.0) {
                        anim.action = mir2_shared::enums::MirAction::Struck;
                        anim.frame_index = 0;
                        // 同上：本地玩家实体在换图重建里同样会被 despawn
                        crate::game::movement::safe_insert(&mut commands, e, StruckTimer(0.6));
                        probe.player_struck_applied = probe.player_struck_applied.saturating_add(1);
                        break;
                    }
                }
            }
            CombatEvent::ObjectHealth {
                object_id,
                percent,
                expire,
            } => {
                // C# S.ObjectHealth：挂载血量（血条系统渲染/过期）
                for (e, id, _, _, _) in &mut actors {
                    if id.0 == *object_id {
                        crate::game::movement::safe_insert(
                            &mut commands,
                            e,
                            ActorHp {
                                percent: *percent,
                                expire: *expire as f32,
                            },
                        );
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
                        crate::game::movement::safe_insert(&mut commands, e, StruckTimer(0.6));
                        break;
                    }
                }
            }
            CombatEvent::ObjectMana { object_id, percent } => {
                // #238：更新/插入对象蓝条（刷新式 15s 生命周期）
                for (e, id, _, _, _) in &mut actors {
                    if id.0 == *object_id {
                        crate::game::movement::safe_insert(
                            &mut commands,
                            e,
                            ActorMp {
                                percent: *percent,
                                expire: 15.0,
                            },
                        );
                        break;
                    }
                }
            }
            CombatEvent::Died {
                object_id,
                death_type,
            } => {
                // #1564：本地玩家死亡音（C# PlayDieSound 按性别）
                if local_id == Some(*object_id) {
                    crate::game::sound::play_sound(
                        &mut commands,
                        &mut audio_assets,
                        &sound_bank,
                        crate::game::sound::player_die_sound(local_gender),
                    );
                }
                for (e, id, mut anim, mon, appr) in &mut actors {
                    if id.0 == *object_id {
                        // #1790：C# ObjectDied.Type 1/2——特效+立即移除，不播尸体动画
                        if *death_type != 0 && local_id != Some(*object_id) {
                            crate::game::movement::safe_despawn(&mut commands, e);
                            probe.died_applied = probe.died_applied.saturating_add(1);
                            break;
                        }
                        anim.action = mir2_shared::enums::MirAction::Dead;
                        anim.frame_index = 0;
                        // 本地玩家死亡由 Death 包管理（复活时恢复），不自动 despawn
                        if local_id != Some(*object_id) {
                            crate::game::movement::safe_insert(&mut commands, e, DeathTimer(3.0));
                            probe.died_applied = probe.died_applied.saturating_add(1);
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
                        crate::game::movement::safe_insert(&mut commands, e, StruckTimer(0.6));
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
                        // #1765：怪物无 Attack2-5 帧表时回退 Attack1（避免动画冻结）
                        anim.action = match appr {
                            Some(m) => crate::objects::frames::resolve_monster_attack_action(
                                m.monster_type,
                                *attack_type,
                                mir2_shared::enums::MirDirection::try_from(*direction)
                                    .unwrap_or(mir2_shared::enums::MirDirection::Up),
                                m.stage,
                            ),
                            None => mir2_shared::enums::MirAction::Attack1,
                        };
                        anim.direction = *direction;
                        anim.frame_index = 0;
                        crate::game::movement::safe_insert(&mut commands, e, StruckTimer(0.6));
                        // #1624：怪物攻击动作起始音（C# SetAction → Play*AttackSound）
                        if mon.is_some() {
                            if let Some(appr) = appr {
                                let sound_id = match action {
                                    mir2_shared::enums::MirAction::Attack2 => {
                                        Some(crate::game::sound::monster_second_attack_sound(
                                            appr.monster_type,
                                        ))
                                    }
                                    mir2_shared::enums::MirAction::Attack3 => {
                                        crate::game::sound::monster_third_attack_sound(
                                            appr.monster_type,
                                        )
                                    }
                                    mir2_shared::enums::MirAction::Attack4 => {
                                        crate::game::sound::monster_fourth_attack_sound(
                                            appr.monster_type,
                                        )
                                    }
                                    mir2_shared::enums::MirAction::Attack5 => {
                                        Some(crate::game::sound::monster_fifth_attack_sound(
                                            appr.monster_type,
                                        ))
                                    }
                                    _ => Some(crate::game::sound::monster_attack_sound(
                                        appr.monster_type,
                                    )),
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
            CombatEvent::RangeAttack {
                object_id,
                attack_type,
            } => {
                // #224/#1765：远程攻击动作——玩家用 AttackRange1（C# Action.AttackRange1）；
                // 怪物按 Type 选 AttackRange1/2/3，无帧表回退 Attack1（避免动画冻结）
                for (e, id, mut anim, mon, appr) in &mut actors {
                    if id.0 == *object_id {
                        anim.action = match appr {
                            Some(m) => crate::objects::frames::resolve_monster_range_attack_action(
                                m.monster_type,
                                *attack_type,
                                mir2_shared::enums::MirDirection::try_from(anim.direction)
                                    .unwrap_or(mir2_shared::enums::MirDirection::Up),
                                m.stage,
                            ),
                            None => mir2_shared::enums::MirAction::AttackRange1,
                        };
                        anim.frame_index = 0;
                        crate::game::movement::safe_insert(&mut commands, e, StruckTimer(0.6));
                        // #1629：怪物远程攻击动作起始音（C# PlayRangeSound，AttackRange1）
                        if mon.is_some() {
                            if let Some(appr) = appr {
                                if let Some(sound_id) =
                                    crate::game::sound::monster_range_sound(appr.monster_type)
                                {
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
            CombatEvent::Revived { object_id } => {
                // 复活：恢复站立 + 清除死亡计时（本地玩家由 Revived 包驱动）
                for (e, id, mut anim, mon, appr) in &mut actors {
                    if id.0 == *object_id {
                        anim.action = mir2_shared::enums::MirAction::Standing;
                        anim.frame_index = 0;
                        crate::game::movement::safe_remove::<DeathTimer>(&mut commands, e);
                        // #1634：怪物复活音（C# PlayReviveSound，MonsterObject.cs:4128；僵尸 705）
                        if mon.is_some() {
                            if let Some(appr) = appr {
                                if let Some(sound_id) =
                                    crate::game::sound::monster_revive_sound(appr.monster_type)
                                {
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
            // 伤害飘字（挂到目标实体上自动跟随）
            CombatEvent::Damage {
                object_id,
                damage,
                dmg_type,
            } => {
                // 命中探测：非本地玩家的伤害事件 = 玩家攻击命中目标（#57）
                if local_id != Some(*object_id) {
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
                // C# Damage.Draw：显示在目标头顶上方约 75px（暴击 +Offset15）并向上飘 50px；
                // actor 子实体 +y 向上（根在脚底），故起始 y 用正值、vy 向上
                let y = if is_crit { 145.0 } else { 130.0 };
                let mut spawned: Vec<Entity> = Vec::new();
                commands.entity(target).with_children(|p| {
                    spawned.push(
                        p.spawn((
                            Text2d::new(text.clone()),
                            Anchor::TOP_LEFT,
                            TextColor(color),
                            TextFont {
                                font: FontSource::Handle(ui_font.0.clone()),
                                font_size: FontSize::Px(16.0),
                                ..default()
                            },
                            Transform::from_xyz(0.0, y, 20.0),
                            DamageText {
                                vy: 50.0,
                                life: 1.2,
                            },
                        ))
                        .id(),
                    );
                });
                // C# Damage.cs:36-37 OutLine=true OutLineColour=Black：伤害数字带黑色描边
                if let Some(text_entity) = spawned.first() {
                    crate::ui::outlined_text::outline_on(
                        &mut commands,
                        *text_entity,
                        &text,
                        ui_font.0.clone(),
                        16.0,
                        Anchor::TOP_LEFT,
                        true,
                    );
                }
            }
        }
    }
}

/// 受击/死亡计时
fn advance_combat_timers(
    mut commands: Commands,
    time: Res<Time>,
    sound_bank: Res<crate::game::sound::SoundBank>,
    mut audio_assets: ResMut<Assets<bevy::audio::AudioSource>>,
    mut struck: Query<(Entity, &mut StruckTimer, &mut ActorAnim), Without<DeathTimer>>,
    mut deaths: Query<(Entity, &mut DeathTimer, Option<&MonsterAppearance>), Without<StruckTimer>>,
) {
    for (e, mut t, mut anim) in &mut struck {
        t.0 -= time.delta_secs();
        if t.0 <= 0.0 {
            anim.action = mir2_shared::enums::MirAction::Standing;
            anim.frame_index = 0;
            crate::game::movement::safe_remove::<StruckTimer>(&mut commands, e);
        }
    }
    for (e, mut t, appr) in &mut deaths {
        t.0 -= time.delta_secs();
        if t.0 <= 0.0 {
            // #1634：怪物 Dead 状态音（C# PlayDeadSound，MonsterObject.cs:4113；仅特殊怪 +5）
            if let Some(appr) = appr {
                if let Some(sound_id) = crate::game::sound::monster_dead_sound(appr.monster_type) {
                    crate::game::sound::play_sound(
                        &mut commands,
                        &mut audio_assets,
                        &sound_bank,
                        sound_id,
                    );
                }
            }
            crate::game::movement::safe_despawn(&mut commands, e);
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
            crate::game::movement::safe_despawn(&mut commands, e);
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
    let white = images.add(crate::map_renderer::make_image(
        vec![255, 255, 255, 255],
        1,
        1,
    ));
    for (e, mut hp, bar) in &mut actors {
        hp.expire -= time.delta_secs();
        if hp.expire <= 0.0 {
            let children: Vec<Entity> = bars
                .iter()
                .filter(|(_, c, _, _)| c.parent() == e)
                .map(|(e2, _, _, _)| e2)
                .collect();
            for c in children {
                crate::game::movement::safe_despawn(&mut commands, c);
            }
            crate::game::movement::safe_remove::<ActorHp>(&mut commands, e);
            crate::game::movement::safe_remove::<ActorHpBar>(&mut commands, e);
            continue;
        }
        if bar.is_none() {
            crate::game::movement::safe_insert(&mut commands, e, ActorHpBar);
            // NOTE(#3089)：下面的 with_children 仍是裸写法——它的闭包借用局部句柄（需要 'static 化
            // 才能像 safe_insert 那样延后执行），本批未改造，已记入 PR/walgit 的残留清单。
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
    let white = images.add(crate::map_renderer::make_image(
        vec![255, 255, 255, 255],
        1,
        1,
    ));
    for (e, mut mp, bar) in &mut actors {
        mp.expire -= time.delta_secs();
        if mp.expire <= 0.0 {
            let children: Vec<Entity> = bars
                .iter()
                .filter(|(_, c, _, _)| c.parent() == e)
                .map(|(e2, _, _, _)| e2)
                .collect();
            for c in children {
                crate::game::movement::safe_despawn(&mut commands, c);
            }
            crate::game::movement::safe_remove::<ActorMp>(&mut commands, e);
            crate::game::movement::safe_remove::<ActorMpBar>(&mut commands, e);
            continue;
        }
        if bar.is_none() {
            crate::game::movement::safe_insert(&mut commands, e, ActorMpBar);
            // NOTE(#3089)：下面的 with_children 仍是裸写法——它的闭包借用局部句柄（需要 'static 化
            // 才能像 safe_insert 那样延后执行），本批未改造，已记入 PR/walgit 的残留清单。
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
