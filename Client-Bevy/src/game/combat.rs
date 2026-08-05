// ============================================================================
// 战斗反馈（M10）
// 网络驱动：ObjectStruck（受击动画）/ ObjectDied（死亡）/ DamageIndicator（伤害飘字）
// 参考：Client-Macroquad/src/systems/logic/combat/* + presentation/floating_text_system.rs
// ============================================================================

use bevy::prelude::*;

use crate::actor::{ActorAnim, NetObjectId};
use crate::game::hud::HudState;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiFont;
use bevy::sprite::Anchor;

/// 服务器战斗事件（网络 handler 发送，战斗系统消费）
#[derive(Message, Debug, Clone, Copy)]
pub enum CombatEvent {
    Struck { object_id: u32, direction: u8 },
    Died { object_id: u32, death_type: u8 },
    Revived { object_id: u32 },
    Damage { object_id: u32, damage: i32, dmg_type: u8 },
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
            (apply_combat_events, advance_combat_timers, advance_damage_texts)
                .after(crate::network::network_system)
                .run_if(in_state(AppState::Game)),
        );
        app.add_systems(
            Update,
            attack_mode_system.run_if(in_state(AppState::Game)),
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
    mut actors: Query<(Entity, &NetObjectId, &mut ActorAnim)>,
) {
    for ev in events.read() {
        match ev {
            CombatEvent::Struck { object_id, direction } => {
                crate::game::sound::play_sound(&mut commands, &mut audio_assets, &sound_bank, 10060);
                for (e, id, mut anim) in &mut actors {
                    if id.0 == *object_id {
                        anim.action = mir2_shared::enums::MirAction::Attack1;
                        anim.direction = *direction;
                        anim.frame_index = 0;
                        commands.entity(e).insert(StruckTimer(0.6));
                        break;
                    }
                }
            }
            CombatEvent::Died { object_id, .. } => {
                for (e, id, mut anim) in &mut actors {
                    if id.0 == *object_id {
                        anim.action = mir2_shared::enums::MirAction::Dead;
                        anim.frame_index = 0;
                        // 本地玩家死亡由 Death 包管理（复活时恢复），不自动 despawn
                        if hud.player_object_id != Some(*object_id) {
                            commands.entity(e).insert(DeathTimer(3.0));
                        }
                        break;
                    }
                }
            }
            CombatEvent::Revived { object_id } => {
                // 复活：恢复站立 + 清除死亡计时（本地玩家由 Revived 包驱动）
                for (e, id, mut anim) in &mut actors {
                    if id.0 == *object_id {
                        anim.action = mir2_shared::enums::MirAction::Standing;
                        anim.frame_index = 0;
                        commands.entity(e).remove::<DeathTimer>();
                        break;
                    }
                }
            }
            // 伤害飘字（挂到目标实体上自动跟随）
            CombatEvent::Damage { object_id, damage, .. } => {
                // 命中探测：非本地玩家的伤害事件 = 玩家攻击命中目标（#57）
                if hud.player_object_id != Some(*object_id) {
                    probe.hits += 1;
                }
                if !ui_font.0.is_strong() {
                    continue;
                }
                let Some(target) = actors.iter().find(|(_, id, _)| id.0 == *object_id).map(|(e, _, _)| e) else {
                    continue;
                };
                commands.entity(target).with_children(|p| {
                    p.spawn((
                        Text2d::new(format!("-{}", damage)),
                        Anchor::TOP_LEFT,
                        TextColor(Color::srgb(1.0, 0.9, 0.3)),
                        TextFont {
                            font: FontSource::Handle(ui_font.0.clone()),
                            font_size: FontSize::Px(16.0),
                            ..default()
                        },
                        Transform::from_xyz(0.0, -40.0, 20.0),
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
