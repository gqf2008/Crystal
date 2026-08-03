// ============================================================================
// 战斗反馈（M10）
// 网络驱动：ObjectStruck（受击动画）/ ObjectDied（死亡）/ DamageIndicator（伤害飘字）
// 参考：Client-Macroquad/src/systems/logic/combat/* + presentation/floating_text_system.rs
// ============================================================================

use bevy::prelude::*;

use crate::actor::{ActorAnim, NetObjectId};
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiFont;
use bevy::sprite::Anchor;

/// 服务器战斗事件（网络 handler 发送，战斗系统消费）
#[derive(Message, Debug, Clone, Copy)]
pub enum CombatEvent {
    Struck { object_id: u32, direction: u8 },
    Died { object_id: u32, death_type: u8 },
    Damage { object_id: u32, damage: i32, dmg_type: u8 },
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

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<CombatEvent>();
        app.add_systems(
            Update,
            (apply_combat_events, advance_combat_timers, advance_damage_texts)
                .after(crate::network::network_system)
                .run_if(in_state(AppState::Game)),
        );
    }
}

/// 应用受击/死亡事件 + 生成伤害飘字
fn apply_combat_events(
    mut commands: Commands,
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
                        commands.entity(e).insert(DeathTimer(3.0));
                        break;
                    }
                }
            }
            // 伤害飘字（挂到目标实体上自动跟随）
            CombatEvent::Damage { object_id, damage, .. } => {
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
