// ============================================================================
// sprite_ui - 用 Bevy Sprite 在精确屏幕坐标绘制 UI（对齐 macroquad draw_texture）
// ============================================================================

use std::collections::HashMap;

use bevy::camera::ScalingMode;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::map_renderer::{make_image, GameLibraries};
use crate::resources::libraries::LibraryName;

#[derive(Resource, Clone, Default)]
pub struct UiFont(pub Handle<Font>);

pub fn load_ui_font(assets: &mut Assets<Font>) -> Handle<Font> {
    assets.add(Font::from_bytes(
        include_bytes!("../../assets/fonts/AlibabaPuHuiTi-3-55-Regular.ttf").to_vec(),
    ))
}

/// UI 图像缓存
#[derive(Resource, Default)]
pub struct UiImageCache {
    pub map: HashMap<(u8, usize), Handle<Image>>,
}

/// 标记所有 UI 精灵/文本/按钮，退出场景时统一清理
#[derive(Component)]
pub struct UiEntity;

/// UI 相机（世界坐标 = 屏幕逻辑像素 0..1280 x 0..800，y 向下）
/// 带 UiEntity 标记，随场景退出清理，避免泄漏到游戏场景
pub fn spawn_ui_camera(mut commands: Commands) {
    commands.spawn((
        UiEntity,
        Camera2d,
        Transform::from_xyz(640.0, -400.0, 100.0),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed {
                width: 1280.0,
                height: 800.0,
            },
            ..OrthographicProjection::default_2d()
        }),
        Camera {
            order: 1,
            ..default()
        },
    ));
}

/// 按库+索引加载图像（缓存），返回句柄
pub fn ui_image(
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    cache: &mut UiImageCache,
    name: LibraryName,
    index: usize,
) -> Option<Handle<Image>> {
    let key = (name as u8, index);
    if let Some(h) = cache.map.get(&key) {
        return Some(h.clone());
    }
    let info = libs.0.get_image(name, index)?;
    let rgba = info.rgba.clone()?;
    let w = info.width.max(0) as u32;
    let h = info.height.max(0) as u32;
    if w == 0 || h == 0 {
        return None;
    }
    let handle = images.add(make_image(rgba, w, h));
    cache.map.insert(key, handle.clone());
    Some(handle)
}

/// 生成 UI 精灵（屏幕坐标 x,y 左上角，y 向下；scale 缩放）
pub fn spawn_ui_sprite(
    commands: &mut Commands,
    handle: Handle<Image>,
    x: f32,
    y: f32,
    z: f32,
    scale: f32,
) -> Entity {
    commands
        .spawn((
            UiEntity,
            Sprite::from_image(handle),
            // 屏幕坐标 = 纹理左上角（原版 draw_texture 约定）
            Anchor::TOP_LEFT,
            Transform::from_xyz(x, -y, z).with_scale(Vec3::splat(scale)),
            Visibility::default(),
        ))
        .id()
}

/// UI 文本（Text2d，屏幕坐标，y 向下）
pub fn spawn_ui_text(
    commands: &mut Commands,
    font: &Handle<Font>,
    text: &str,
    x: f32,
    y: f32,
    size: f32,
    color: Color,
    z: f32,
) -> Entity {
    commands
        .spawn((
            UiEntity,
            Text2d::new(text),
            // 左上角锚定，与宏模块 draw_text 左上角坐标一致
            Anchor::TOP_LEFT,
            TextFont {
                font: FontSource::Handle(font.clone()),
                font_size: FontSize::Px(size),
                ..default()
            },
            TextColor(color),
            Transform::from_xyz(x, -y, z),
            Visibility::default(),
        ))
        .id()
}

/// UI 按钮（矩形命中测试）
#[derive(Component)]
pub struct UiButton {
    pub rect: (f32, f32, f32, f32),
    pub clicked: bool,
}

/// 三态按钮帧（normal/hover/pressed），与原版 draw_button 一致
#[derive(Component)]
pub struct ButtonFrames {
    pub normal: Handle<Image>,
    pub hover: Handle<Image>,
    pub pressed: Handle<Image>,
}

/// 生成带三态帧的按钮（normal_idx/hover_idx/pressed_idx）
pub fn spawn_ui_button(
    commands: &mut Commands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    cache: &mut UiImageCache,
    name: LibraryName,
    normal_idx: usize,
    hover_idx: usize,
    pressed_idx: usize,
    x: f32,
    y: f32,
    z: f32,
    w: f32,
    h: f32,
) -> Option<Entity> {
    let normal = ui_image(libs, images, cache, name, normal_idx)?;
    let hover = ui_image(libs, images, cache, name, hover_idx)?;
    let pressed = ui_image(libs, images, cache, name, pressed_idx)?;
    let e = spawn_ui_sprite(commands, normal.clone(), x, y, z, 1.0);
    commands.entity(e).insert((
        UiButton { rect: (x, y, w, h), clicked: false },
        ButtonFrames { normal, hover, pressed },
    ));
    Some(e)
}

/// 按钮系统：鼠标左键按下时命中矩形 → clicked=true；
/// 带 ButtonFrames 的按钮按 hover/pressed 状态切换帧
pub fn ui_button_system(
    mut buttons: Query<(&mut UiButton, Option<&mut ButtonFrames>, &mut Sprite)>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let just = mouse.just_pressed(MouseButton::Left);
    let down = mouse.pressed(MouseButton::Left);
    for (mut btn, frames, mut sprite) in &mut buttons {
        let (x, y, w, h) = btn.rect;
        let over = cursor.x >= x && cursor.x <= x + w && cursor.y >= y && cursor.y <= y + h;
        btn.clicked = just && over;
        if let Some(frames) = frames {
            let frame = if down && over {
                &frames.pressed
            } else if over {
                &frames.hover
            } else {
                &frames.normal
            };
            if sprite.image != *frame {
                sprite.image = frame.clone();
            }
        }
    }
}



