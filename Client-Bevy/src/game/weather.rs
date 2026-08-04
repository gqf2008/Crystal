// ============================================================================
// 天气系统（M11）
// 天气码：0=晴 1=雨 2=雪 3=雾 4=沙（对齐 macroquad weather_system.rs）
// 网络：MapChanged.weather / MapInformation.weather_particles → WeatherState
// 粒子：屏幕空间简单粒子（雨=下落短线，雪=缓慢白点）
// ============================================================================

use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::scenes::AppState;
use crate::ui::sprite_ui::UiEntity;

/// 当前天气码
#[derive(Resource, Default)]
pub struct WeatherState {
    pub code: u16,
    pub rain: bool,
    pub snow: bool,
}

/// 雨粒子
#[derive(Component)]
pub struct RainDrop {
    pub vy: f32,
}

/// 雪粒子
#[derive(Component)]
pub struct SnowFlake {
    pub vy: f32,
    pub sway: f32,
    pub phase: f32,
}

const PARTICLE_COUNT: usize = 120;

pub struct WeatherPlugin;

impl Plugin for WeatherPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WeatherState>();
                app.add_systems(
            Update,
            weather_server_events.run_if(in_state(AppState::Game)),
        );
app.add_systems(
            Update,
            (weather_update_system, advance_particles).run_if(in_state(AppState::Game)),
        );
    }
}

/// 天气码变化时生成/销毁粒子
fn weather_update_system(
    mut commands: Commands,
    weather: Res<WeatherState>,
    mut images: ResMut<Assets<Image>>,
    mut existing: Local<Vec<Entity>>,
) {
    // 需要的粒子种类
    let need_rain = weather.code == 1;
    let need_snow = weather.code == 2;
    if need_rain == weather.rain && need_snow == weather.snow {
        return;
    }
    // 清理旧粒子
    for e in existing.drain(..) {
        commands.entity(e).despawn();
    }
    // 生成新粒子
    if need_rain {
        let mut rgba = Vec::with_capacity(2 * 6 * 4);
        for _ in 0..(2 * 6) {
            rgba.extend_from_slice(&[255, 255, 255, 255]);
        }
        let white = images.add(crate::map_renderer::make_image(rgba, 2, 6));
        for _ in 0..PARTICLE_COUNT {
            let x = fastrand_f32() * 1024.0;
            let y = fastrand_f32() * 768.0;
            let e = commands
                .spawn((
                    UiEntity,
                    RainDrop {
                        vy: 600.0 + fastrand_f32() * 200.0,
                    },
                    Sprite::from_image(white.clone()),
                    Anchor::TOP_LEFT,
                    Transform::from_xyz(x, -y, 50.0),
                    Visibility::Visible,
                ))
                .id();
            existing.push(e);
        }
    } else if need_snow {
        let mut rgba = Vec::with_capacity(2 * 2 * 4);
        for _ in 0..(2 * 2) {
            rgba.extend_from_slice(&[255, 255, 255, 255]);
        }
        let white = images.add(crate::map_renderer::make_image(rgba, 2, 2));
        for _ in 0..PARTICLE_COUNT {
            let x = fastrand_f32() * 1024.0;
            let y = fastrand_f32() * 768.0;
            let e = commands
                .spawn((
                    UiEntity,
                    SnowFlake {
                        vy: 40.0 + fastrand_f32() * 30.0,
                        sway: 20.0 + fastrand_f32() * 30.0,
                        phase: fastrand_f32() * std::f32::consts::TAU,
                    },
                    Sprite::from_image(white.clone()),
                    Anchor::TOP_LEFT,
                    Transform::from_xyz(x, -y, 50.0),
                    Visibility::Visible,
                ))
                .id();
            existing.push(e);
        }
    }
}

fn fastrand_f32() -> f32 {
    // 轻量伪随机（避免引入依赖）
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    (seed.wrapping_mul(1664525).wrapping_add(1013904223)) as f32 / u32::MAX as f32
}

/// 推进雨/雪粒子
fn advance_particles(
    time: Res<Time>,
    mut rain: Query<(&mut RainDrop, &mut Transform), Without<SnowFlake>>,
    mut snow: Query<(&mut SnowFlake, &mut Transform), Without<RainDrop>>,
) {
    let dt = time.delta_secs();
    for (drop, mut tf) in &mut rain {
        tf.translation.y -= drop.vy * dt;
        if tf.translation.y < -768.0 {
            tf.translation.y = 0.0;
        }
    }
    for (mut flake, mut tf) in &mut snow {
        tf.translation.y -= flake.vy * dt;
        flake.phase += dt * 2.0;
        tf.translation.x += flake.sway * dt * flake.phase.sin();
        if tf.translation.y < -768.0 {
            tf.translation.y = 0.0;
        }
        if tf.translation.x < 0.0 {
            tf.translation.x = 1024.0;
        }
        if tf.translation.x > 1024.0 {
            tf.translation.x = 0.0;
        }
    }
}


/// 消费服务端天气事件（网络层只广播 ServerEvent）
fn weather_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut weather: ResMut<WeatherState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::WeatherChanged { code } = ev {
            weather.code = *code;
        }
    }
}
