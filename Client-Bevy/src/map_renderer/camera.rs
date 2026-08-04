// ============================================================================
// map_renderer 模块拆分（#72）
// ============================================================================

use bevy::prelude::*;
use super::*;

pub(crate) fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Transform::default(),
        Projection::Orthographic(OrthographicProjection {
            scale: 1.0,
            ..OrthographicProjection::default_2d()
        }),
    ));
}

/// 把指定块的三层之一合成一张 RGBA 画布。块内无任何瓦片时返回 None。
///
/// 供 Bevy 渲染与离屏诊断（examples）共用，确保验证路径与渲染路径一致。

pub(crate) fn camera_control(
    mut camera: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let Ok((mut transform, mut projection)) = camera.single_mut() else {
        return;
    };
    let Projection::Orthographic(ortho) = projection.as_mut() else {
        return;
    };
    let dt = time.delta_secs();

    let mut pan = Vec3::ZERO;
    let speed = 480.0 * dt;
    if keys.pressed(KeyCode::ArrowLeft) || keys.pressed(KeyCode::KeyA) {
        pan.x -= speed;
    }
    if keys.pressed(KeyCode::ArrowRight) || keys.pressed(KeyCode::KeyD) {
        pan.x += speed;
    }
    if keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::KeyW) {
        pan.y += speed;
    }
    if keys.pressed(KeyCode::ArrowDown) || keys.pressed(KeyCode::KeyS) {
        pan.y -= speed;
    }
    transform.translation += pan;

    // 缩放：1.0 = 1 世界单位 ≈ 1 像素
    if keys.pressed(KeyCode::Equal) || keys.pressed(KeyCode::NumpadAdd) {
        ortho.scale = (ortho.scale / 1.02).max(0.02);
    }
    if keys.pressed(KeyCode::Minus) || keys.pressed(KeyCode::NumpadSubtract) {
        ortho.scale = (ortho.scale * 1.02).min(4.0);
    }
}


/// 图层调试热键：1=Back 2=Middle 3=Front静态 F=动画/混合
pub(crate) fn map_layer_toggle_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut show: ResMut<MapLayerShow>,
    mut floors: Query<(&MapFloorMark, &mut Visibility), (Without<FrontTile>,)>,
    mut fronts: Query<&mut Visibility, (With<FrontTile>, Without<MapFloorMark>)>,
    mut anims: Query<&mut Visibility, (With<crate::map_tile_anim::MapTileAnim>, Without<MapFloorMark>, Without<FrontTile>)>,
    mut lights: Query<&mut Visibility, (With<MapLight>, Without<MapFloorMark>, Without<FrontTile>, Without<crate::map_tile_anim::MapTileAnim>)>,
) {
    if keys.just_pressed(KeyCode::Digit1) { show.back = !show.back; tracing::info!("[LAYER] Back {}", if show.back {"ON"} else {"OFF"}); }
    if keys.just_pressed(KeyCode::Digit2) { show.middle = !show.middle; tracing::info!("[LAYER] Middle {}", if show.middle {"ON"} else {"OFF"}); }
    if keys.just_pressed(KeyCode::Digit3) { show.front = !show.front; tracing::info!("[LAYER] Front {}", if show.front {"ON"} else {"OFF"}); }
    if keys.just_pressed(KeyCode::KeyF) { show.anim = !show.anim; tracing::info!("[LAYER] Anim {}", if show.anim {"ON"} else {"OFF"}); }
    for (mark, mut vis) in floors.iter_mut() {
        let on = match mark.0 { Layer::Back => show.back, Layer::Middle => show.middle, Layer::Front => show.front };
        *vis = if on { Visibility::Visible } else { Visibility::Hidden };
    }
    for mut vis in fronts.iter_mut() {
        *vis = if show.front { Visibility::Visible } else { Visibility::Hidden };
    }
    // F 键同时控制动画/混合瓦片与地图灯光（原版 F 键开关动画/灯光）
    for mut vis in anims.iter_mut() {
        *vis = if show.anim { Visibility::Visible } else { Visibility::Hidden };
    }
    for mut vis in lights.iter_mut() {
        *vis = if show.anim { Visibility::Visible } else { Visibility::Hidden };
    }
}


/// 相机跟随（参考 macroquad CameraFollowSystem）：远距直跳 + lerp 平滑
pub(crate) fn camera_follow_system(
    mut camera: Query<
        &mut Transform,
        (
            With<Camera2d>,
            Without<crate::ui::sprite_ui::UiEntity>,
            Without<crate::actor::LocalPlayer>,
        ),
    >,
    players: Query<
        &Transform,
        (
            With<crate::actor::LocalPlayer>,
            With<crate::actor::NetObjectId>,
            Without<Camera2d>,
        ),
    >,
) {
    let Ok(mut cam) = camera.single_mut() else { return };
    let Ok(player) = players.single() else { return };
    // C# 风格：相机精确跟随玩家（玩家恒定在屏幕中心），
    // 消除 lerp 滞后造成的画面轻微抖动/拖影
    let p = player.translation;
    let c = cam.translation;
    let far = (p.x - c.x).abs() > 1024.0 * 6.0 || (p.y - c.y).abs() > 768.0 * 6.0;
    if far || (p.x - c.x).abs() > 0.01 || (p.y - c.y).abs() > 0.01 {
        cam.translation.x = p.x;
        cam.translation.y = p.y;
    }
}
