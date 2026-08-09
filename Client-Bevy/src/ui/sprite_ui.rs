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

/// 给所有 UI 实体加渲染层 1（只被 UI 相机渲染，避免 UI 相机重画地图）
pub fn mark_ui_render_layers(
    q: Query<Entity, Added<UiEntity>>,
    mut commands: Commands,
) {
    for e in &q {
        commands.entity(e).try_insert(bevy::camera::visibility::RenderLayers::layer(1));
    }
}

/// UI 相机（世界坐标 = 屏幕逻辑像素 0..1024 x 0..768，y 向下）
/// 带 UiEntity 标记，随场景退出清理，避免泄漏到游戏场景
pub fn spawn_ui_camera(mut commands: Commands) {
    commands.spawn((
        UiEntity,
        Camera2d,
        Transform::from_xyz(512.0, -384.0, 100.0),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed {
                width: 1024.0,
                height: 768.0,
            },
            ..OrthographicProjection::default_2d()
        }),
        Camera {
            order: 1,
            ..default()
        },
        // 只渲染 UI 层，避免把地图实体也画一遍（见 game/mod.rs）
        bevy::camera::visibility::RenderLayers::layer(1),
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

/// 强制不透明加载 UI 纹理：
/// 全局 bgra_to_rgba 会把纯黑像素转透明（为修地图黑块的 workaround），
/// 但这会毁掉黑底不透明 UI 纹理（如 HUD 底条 Prguse[1]、深色对话框背景）。
pub fn ui_image_opaque(
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    cache: &mut UiImageCache,
    name: LibraryName,
    index: usize,
) -> Option<Handle<Image>> {
    use std::collections::hash_map::Entry;
    let key = (name as u8, index);
    match cache.map.entry(key) {
        Entry::Occupied(e) => return Some(e.get().clone()),
        Entry::Vacant(_) => {}
    }
    let info = libs.0.get_image(name, index)?;
    let mut rgba = info.rgba.clone()?;
    // 黑→透明 hack 后强制恢复全不透明（UI 面板背景是设计成不透明的）
    for a in rgba.chunks_exact_mut(4) {
        a[3] = 255;
    }
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
    ui_cameras: Query<(&Camera, &GlobalTransform), With<UiEntity>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    // UI 相机 Fixed 1024x768：窗口缩放/最大化时需换算成 UI 世界坐标（按钮命中保持准确）
    let Ok((cam, gtf)) = ui_cameras.single() else {
        return;
    };
    let Ok(world) = cam.viewport_to_world_2d(gtf, cursor) else {
        return;
    };
    let cursor = Vec2::new(world.x, -world.y);
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





/// 按钮点击音效覆盖（#91）：默认 ButtonB=10104，可指定 C# SoundList 音效 id
#[derive(Component)]
pub struct ButtonSound(pub u32);

/// 按钮悬停进入音效（#91，可选挂载；C# MirButton 默认只有点击音效）
#[derive(Component)]
pub struct ButtonHoverSound(pub u32);

/// UI 按钮音效系统（#91）：
/// - 点击：播放 ButtonSound 覆盖或默认 ButtonB(10104)（对齐 C# MirControl.OnMouseClick）
/// - 悬停进入：仅对挂了 ButtonHoverSound 的按钮播放一次
pub fn ui_button_sound_system(
    mut commands: Commands,
    mut assets: ResMut<Assets<bevy::audio::AudioSource>>,
    bank: Res<crate::game::sound::SoundBank>,
    mut cache: ResMut<crate::game::sound::SoundCache>,
    windows: Query<&Window>,
    buttons: Query<(Entity, &UiButton, Option<&ButtonSound>, Option<&ButtonHoverSound>)>,
    mut hovered_prev: Local<std::collections::HashSet<Entity>>,
    ui_cameras: Query<(&Camera, &GlobalTransform), With<UiEntity>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok((cam, gtf)) = ui_cameras.single() else {
        return;
    };
    let Ok(world) = cam.viewport_to_world_2d(gtf, cursor) else {
        return;
    };
    let cursor = Vec2::new(world.x, -world.y);

    let mut hovered_now = std::collections::HashSet::new();
    for (e, btn, sound, hover_sound) in &buttons {
        let (x, y, w, h) = btn.rect;
        let over = cursor.x >= x && cursor.x <= x + w && cursor.y >= y && cursor.y <= y + h;
        if over {
            hovered_now.insert(e);
        }
        // 点击音效
        if btn.clicked {
            let id = sound.map(|s| s.0).unwrap_or(10104); // C# SoundList.ButtonB
            crate::game::sound::play_sound_cached(&mut commands, &mut assets, &bank, &mut cache, id);
        }
        // 悬停进入音效（可选）
        if over && !hovered_prev.contains(&e) {
            if let Some(hs) = hover_sound {
                crate::game::sound::play_sound_cached(&mut commands, &mut assets, &bank, &mut cache, hs.0);
            }
        }
    }
    *hovered_prev = hovered_now;
}
