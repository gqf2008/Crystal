// ============================================================================
// credits - 选角界面 CREDITS 制作名单弹窗（#85）
// ============================================================================
// 参考 Client-Macroquad scenes/select_scene/credits_dialog.rs：
// 模态面板（Prguse[360]）+ 居中标题 + 左对齐内容行，ESC / 任意左键点击关闭。
// 原版 C# SelectScene 的 CreditsButton.Click 为空，但用户要求实现功能。

use bevy::prelude::*;
use bevy::input::keyboard::KeyboardInput;

use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_image, UiEntity, UiFont, UiImageCache,
};

/// CREDITS 弹窗开关（select_ui_system 打开，本模块系统渲染/关闭）
#[derive(Resource, Default)]
pub struct CreditsState {
    pub visible: bool,
}

/// 弹窗实体标记（打开时生成，关闭时清除）
#[derive(Component)]
pub(crate) struct CreditsDlg;

/// 内容行（macroquad 版一致）：文字 / 字号 / 是否标题（居中）/ 颜色
fn content() -> Vec<(&'static str, f32, bool, bevy::prelude::Color)> {
    vec![
        ("Legend of Mir 2", 20.0, true, Color::srgb(1.0, 0.84, 0.0)),
        ("Bevy 客户端移植", 14.0, true, Color::srgb(0.72, 0.72, 0.72)),
        ("", 8.0, false, Color::WHITE),
        ("Version 0.1.0", 13.0, false, Color::WHITE),
        ("Technology: Rust + Bevy", 13.0, false, Color::WHITE),
        ("", 8.0, false, Color::WHITE),
        ("Development", 14.0, true, Color::srgb(0.4, 0.78, 1.0)),
        ("Original: Crystal Team", 12.0, false, Color::WHITE),
        ("Bevy Port: Community", 12.0, false, Color::WHITE),
        ("", 10.0, false, Color::WHITE),
        ("Press ESC or Click to Close", 11.0, true, Color::srgb(0.6, 0.6, 0.6)),
    ]
}

/// CREDITS 弹窗：打开时生成面板+文本，ESC/左键关闭时清除
#[allow(clippy::too_many_arguments)]
pub fn credits_update_system(
    mut commands: Commands,
    mut state: ResMut<CreditsState>,
    mut keys: MessageReader<KeyboardInput>,
    windows: Query<&Window>,
    mouse: Res<ButtonInput<MouseButton>>,
    dlg: Query<Entity, With<CreditsDlg>>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mut ui_font: ResMut<UiFont>,
    mut fonts: ResMut<Assets<Font>>,
    mut opened: Local<bool>,
) {
    // 调试：BEVY_OPEN_CREDITS=1 进入选角即打开（与 BEVY_OPEN_MODAL 一致，live 截图验证用）
    if std::env::var("BEVY_OPEN_CREDITS").as_deref() == Ok("1") && !*opened {
        state.visible = true;
        *opened = true;
    }
    let existing = dlg.iter().count();
    if state.visible {
        if existing == 0 {
            libs.0.ensure_initialized();
            if !ui_font.0.is_strong() {
                ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
            }
            let font = ui_font.0.clone();
            // 面板尺寸：Prguse[360]（原版 MirMessageBox 456x190）
            let (dw, dh) = libs
                .0
                .get_image(LibraryName::Prguse, 360)
                .map(|i| (i.width.max(0) as f32, i.height.max(0) as f32))
                .unwrap_or((456.0, 190.0));
            let dx = (1024.0 - dw) / 2.0;
            let dy = (768.0 - dh) / 2.0;

            if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 360) {
                let e = spawn_ui_sprite(&mut commands, h, dx, dy, 6.0, 1.0);
                commands.entity(e).insert(CreditsDlg);
            }

            // 内容：顶部偏移 30；标题居中，普通行左对齐 x+60
            let top = dy + 30.0;
            let center_x = dx + dw / 2.0;
            let left_x = dx + 60.0;
            let mut y = top;
            for (text, size, is_title, color) in content() {
                if text.is_empty() {
                    y += size * 0.4;
                    continue;
                }
                let tx = if is_title { center_x } else { left_x };
                let e = spawn_ui_text(
                    &mut commands,
                    &font,
                    text,
                    tx,
                    y,
                    size,
                    color,
                    7.0,
                );
                commands.entity(e).insert(CreditsDlg);
                y += size + if is_title { 10.0 } else { 5.0 };
            }
        }
        // 关闭：ESC 或任意左键（打开当帧 existing==0 不关闭，避免点击按钮即开即关）
        if existing > 0 {
            let esc = keys.read().any(|k| k.key_code == KeyCode::Escape);
            let _ = windows.single();
            if esc || mouse.just_pressed(MouseButton::Left) {
                state.visible = false;
            }
        }
    } else if existing > 0 {
        for e in dlg.iter() {
            commands.entity(e).despawn();
        }
    }
}
