// ============================================================================
// 日夜循环（M11 简化版）
// 参考：C# TimeOfDay / macroquad day-night
// 简化：本地 24 分钟一天循环，夜晚叠加半透明深色覆盖层
// ============================================================================

use bevy::prelude::*;

use crate::scenes::AppState;
use crate::ui::sprite_ui::UiEntity;

/// 昼夜状态
#[derive(Resource)]
pub struct DayNight {
    /// 游戏时间（分钟，0..1440）
    pub time_minutes: f32,
    /// 一天实际时长（秒）
    pub day_length_secs: f32,
    /// 是否启用
    pub enabled: bool,
}

impl Default for DayNight {
    fn default() -> Self {
        Self {
            time_minutes: 8.0 * 60.0, // 早上 8 点开始
            day_length_secs: 24.0 * 60.0, // 24 分钟一天
            enabled: true,
        }
    }
}

#[derive(Component)]
pub struct NightOverlay;

pub struct DayNightPlugin;

impl Plugin for DayNightPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DayNight>();
        app.add_systems(OnEnter(AppState::Game), spawn_overlay);
        app.add_systems(OnExit(AppState::Game), cleanup_overlay);
        app.add_systems(
            Update,
            day_night_system.run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_overlay(mut commands: Commands, overlay: Query<Entity, With<NightOverlay>>) {
    for e in overlay.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_overlay(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    commands.spawn((
        UiEntity,
        NightOverlay,
        Sprite {
            image: white,
            custom_size: Some(Vec2::new(1024.0, 768.0)),
            color: Color::srgba(0.02, 0.02, 0.12, 0.0),
            ..default()
        },
        Transform::from_xyz(512.0, -384.0, 40.0),
        Visibility::Visible,
    ));
}

/// 推进时间并更新夜晚覆盖层透明度
fn day_night_system(
    mut dn: ResMut<DayNight>,
    time: Res<Time>,
    mut overlay: Query<&mut Sprite, With<NightOverlay>>,
) {
    if !dn.enabled {
        return;
    }
    dn.time_minutes += time.delta_secs() * (1440.0 / dn.day_length_secs.max(1.0));

    // 亮度曲线：6:00 日出 → 18:00 日落 → 24:00 最深
    let t = dn.time_minutes / 1440.0; // 0..1
    // 用正弦近似白天亮夜晚暗
    let daylight = ((t * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2).sin() + 1.0) / 2.0;
    let darkness = (1.0 - daylight).clamp(0.0, 0.55);

    if let Ok(mut sprite) = overlay.single_mut() {
        sprite.color = Color::srgba(0.02, 0.02, 0.12, darkness);
    }
}
