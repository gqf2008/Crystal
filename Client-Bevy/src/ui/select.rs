// ============================================================================
// SelectPlugin - 角色选择（Sprite 精确坐标版，对齐 macroquad SelectScene）
// ============================================================================

use bevy::prelude::*;

use crate::map_renderer::GameLibraries;
use crate::network::NetworkContext;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_button, spawn_ui_camera, spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image,
    UiButton, UiEntity, UiFont, UiImageCache,
};

pub struct SelectPlugin;

impl Plugin for SelectPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectAnim>();
        app.init_resource::<UiImageCache>();
        app.init_resource::<UiFont>();
        app.add_systems(OnEnter(AppState::Select), setup_select_ui);
        app.add_systems(OnExit(AppState::Select), cleanup_select_ui);
        app.add_systems(
            Update,
            (select_ui_system, select_anim_system, ui_button_system)
                .run_if(in_state(AppState::Select)),
        );
    }
}

#[derive(Resource, Default)]
pub struct SelectAnim {
    pub preview_frame: usize,
    pub preview_timer: f32,
    pub preview_handles: Vec<Handle<Image>>,
}

#[derive(Component)]
struct PreviewImg;

#[derive(Component)]
struct CharButton(i32);

#[derive(Clone, Copy, PartialEq)]
enum BottomBtn {
    Start,
    NewChar,
    Delete,
    Credits,
    Exit,
}

#[derive(Component)]
struct BottomButton(BottomBtn);

fn preview_base_index(class: mir2_shared::MirClass, gender: mir2_shared::MirGender) -> usize {
    let g = match gender {
        mir2_shared::MirGender::Female => 1usize,
        _ => 0,
    };
    match class {
        mir2_shared::MirClass::Archer => {
            if g == 0 {
                100
            } else {
                140
            }
        }
        _ => 20 + (class as usize * 20) + (g * 280),
    }
}

fn class_slot(c: &mir2_shared::SelectInfo) -> usize {
    match c.class {
        mir2_shared::MirClass::Warrior => 0usize,
        mir2_shared::MirClass::Wizard => 1,
        mir2_shared::MirClass::Taoist => 2,
        mir2_shared::MirClass::Assassin => 3,
        mir2_shared::MirClass::Archer => 4,
    }
}

fn setup_select_ui(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut anim: ResMut<SelectAnim>,
    mut cache: ResMut<UiImageCache>,
    mut ui_font: ResMut<UiFont>,
    net: Res<NetworkContext>,
) {
    libs.0.ensure_initialized();
    ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    let font = ui_font.0.clone();
    spawn_ui_camera(commands.reborrow());

    // 背景 Prguse[65]（1024x768）
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 65) {
        spawn_ui_sprite(&mut commands, h, 0.0, 0.0, 0.0, 1.0);
    }
    // 标题 Title[40]
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 40) {
        spawn_ui_sprite(&mut commands, h, 468.0, 20.0, 1.0, 1.0);
    }
    // 服务器名
    spawn_ui_text(&mut commands, &font, "Legend of Mir 2", 460.0, 77.0, 17.0, Color::WHITE, 2.0);

    // 角色预览（初始）
    anim.preview_handles.clear();
    if let Some(c) = net.characters.first() {
        let base = preview_base_index(c.class, c.gender);
        for i in 0..16usize {
            if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::ChrSel, base + i) {
                anim.preview_handles.push(h);
            }
        }
    }
    if let Some(pv) = anim.preview_handles.first().cloned() {
        let e = spawn_ui_sprite(&mut commands, pv, 260.0, 420.0, 3.0, 1.2);
        commands.entity(e).insert(PreviewImg);
    }

    // 角色信息（Last Online 对齐原版 (200,623)/(280,623)）
    spawn_ui_text(&mut commands, &font, "Last Online:", 200.0, 623.0, 14.0, Color::WHITE, 2.0);
    if let Some(c) = net.characters.first() {
        spawn_ui_text(
            &mut commands,
            &font,
            &c.last_access.format("%Y-%m-%d %H:%M").to_string(),
            280.0,
            623.0,
            14.0,
            Color::WHITE,
            2.0,
        );
    }

    // 角色按钮（4 槽位）
    let positions = [(637.0f32, 194.0f32), (637.0, 298.0), (637.0, 402.0), (637.0, 506.0)];
    for (i, (x, y)) in positions.iter().enumerate() {
        if let Some(c) = net.characters.get(i) {
            let slot = class_slot(c);
            let frame = if net.selected_index == Some(i as i32) { slot + 5 } else { slot };
            if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 660 + frame) {
                let e = spawn_ui_sprite(&mut commands, h, *x, *y, 2.0, 1.0);
                commands.entity(e).insert((
                    CharButton(i as i32),
                    UiButton { rect: (*x, *y, 280.0, 90.0), clicked: false },
                ));
            }
            // 名字/Lv/职业
            let class_name = match c.class {
                mir2_shared::MirClass::Warrior => "战士",
                mir2_shared::MirClass::Wizard => "法师",
                mir2_shared::MirClass::Taoist => "道士",
                mir2_shared::MirClass::Assassin => "刺客",
                mir2_shared::MirClass::Archer => "弓手",
            };
            spawn_ui_text(&mut commands, &font, &c.name, x + 107.0, y + 18.0, 13.0, Color::WHITE, 3.0);
            spawn_ui_text(&mut commands, &font, &format!("Lv.{}", c.level), x + 107.0, y + 37.0, 11.0, Color::srgb(0.75, 0.75, 0.75), 3.0);
            spawn_ui_text(&mut commands, &font, class_name, x + 178.0, y + 37.0, 11.0, Color::srgb(0.75, 0.75, 0.75), 3.0);
        } else {
            // 空槽位 Prguse[44]
            if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 44) {
                spawn_ui_sprite(&mut commands, h, *x, *y, 2.0, 1.0);
            }
        }
    }

    // 底部按钮（三态帧，对齐原版 SelectScene）
    let screen_w = 1280.0f32;
    let x_point = (screen_w - 200.0) / 5.0;
    let y = 800.0 - 32.0;
    spawn_bottom_btn(&mut commands, &mut libs, &mut images, &mut cache, 340, 100.0 + x_point - x_point / 2.0 - 50.0, y, BottomBtn::Start);
    spawn_bottom_btn(&mut commands, &mut libs, &mut images, &mut cache, 343, 100.0 + x_point * 2.0 - x_point / 2.0 - 50.0, y, BottomBtn::NewChar);
    spawn_bottom_btn(&mut commands, &mut libs, &mut images, &mut cache, 346, 100.0 + x_point * 3.0 - x_point / 2.0 - 50.0, y, BottomBtn::Delete);
    spawn_bottom_btn(&mut commands, &mut libs, &mut images, &mut cache, 349, 100.0 + x_point * 4.0 - x_point / 2.0 - 50.0, y, BottomBtn::Credits);
    spawn_bottom_btn(&mut commands, &mut libs, &mut images, &mut cache, 352, 100.0 + x_point * 5.0 - x_point / 2.0 - 50.0, y, BottomBtn::Exit);
}

fn spawn_bottom_btn(
    commands: &mut Commands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    cache: &mut UiImageCache,
    index: usize,
    x: f32,
    y: f32,
    kind: BottomBtn,
) {
    if let Some(e) = spawn_ui_button(commands, libs, images, cache, LibraryName::Title, index, index + 1, index + 2, x, y, 2.0, 100.0, 25.0) {
        commands.entity(e).insert(BottomButton(kind));
    }
}

fn cleanup_select_ui(mut commands: Commands, root: Query<Entity, With<UiEntity>>) {
    for e in root.iter() {
        commands.entity(e).despawn();
    }
}

fn select_ui_system(
    mut net: ResMut<NetworkContext>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut anim: ResMut<SelectAnim>,
    mut cache: ResMut<UiImageCache>,
    char_btns: Query<(&UiButton, &CharButton)>,
    bottom: Query<(&UiButton, &BottomButton)>,
    mut preview: Query<&mut Sprite, With<PreviewImg>>,
) {
    // 选择角色
    for (btn, cb) in char_btns.iter() {
        if btn.clicked && net.selected_index != Some(cb.0) {
            net.selected_index = Some(cb.0);
            if let Some(c) = net.characters.iter().find(|c| c.index == cb.0) {
                let base = preview_base_index(c.class, c.gender);
                anim.preview_handles.clear();
                for i in 0..16usize {
                    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::ChrSel, base + i) {
                        anim.preview_handles.push(h);
                    }
                }
                anim.preview_frame = 0;
                if let Ok(mut s) = preview.single_mut() {
                    if let Some(h) = anim.preview_handles.first() {
                        s.image = h.clone();
                    }
                }
            }
        }
    }
    // 底部按钮
    for (btn, bb) in bottom.iter() {
        if btn.clicked {
            match bb.0 {
                BottomBtn::Start => {
                    if let Some(idx) = net.selected_index {
                        net.send_packet(&mir2_shared::packets::client::account::StartGame {
                            character_index: idx,
                        });
                    }
                }
                BottomBtn::Credits => {}
                BottomBtn::Exit => std::process::exit(0),
                BottomBtn::NewChar | BottomBtn::Delete => {}
            }
        }
    }
}

fn select_anim_system(
    mut anim: ResMut<SelectAnim>,
    time: Res<Time>,
    mut preview: Query<&mut Sprite, With<PreviewImg>>,
) {
    anim.preview_timer += time.delta_secs();
    if anim.preview_timer >= 0.25 {
        anim.preview_timer = 0.0;
        anim.preview_frame = (anim.preview_frame + 1) % anim.preview_handles.len().max(1);
        if let Ok(mut s) = preview.single_mut() {
            if let Some(h) = anim.preview_handles.get(anim.preview_frame) {
                s.image = h.clone();
            }
        }
    }
}
