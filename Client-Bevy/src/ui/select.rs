// ============================================================================
// SelectPlugin - 角色选择（对齐 macroquad SelectScene：Prguse 背景 + Title 按钮 + 预览）
// ============================================================================

use bevy::prelude::*;

use crate::map_renderer::GameLibraries;
use crate::network::NetworkContext;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::theme::{load_cn_font, ImageButton};

pub struct SelectPlugin;

impl Plugin for SelectPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectAnim>();
        app.add_systems(OnEnter(AppState::Select), setup_select_ui);
        app.add_systems(OnExit(AppState::Select), cleanup_select_ui);
        app.add_systems(
            Update,
            (select_ui_system, select_anim_system, crate::ui::theme::image_button_system)
                .run_if(in_state(AppState::Select)),
        );
    }
}

#[derive(Component)]
struct SelectRoot;
#[derive(Component)]
struct PreviewImg;
#[derive(Component)]
struct CharButton(i32);
#[derive(Component)]
struct StartButton;
#[derive(Component)]
struct NewCharButton;
#[derive(Component)]
struct DeleteButton;
#[derive(Component)]
struct InfoText;

#[derive(Resource, Default)]
pub struct SelectAnim {
    pub preview_frame: usize,
    pub preview_timer: f32,
    pub preview_handles: Vec<Handle<Image>>,
}

fn preview_base_index(class: mir2_shared::MirClass, gender: mir2_shared::MirGender) -> usize {
    let g = match gender {
        mir2_shared::MirGender::Female => 1usize,
        _ => 0,
    };
    match class {
        mir2_shared::MirClass::Archer => {
            if g == 0 { 100 } else { 140 }
        }
        _ => 20 + (class as usize * 20) + (g * 280),
    }
}

fn setup_select_ui(
    mut commands: Commands,
    mut fonts: ResMut<Assets<Font>>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut anim: ResMut<SelectAnim>,
    net: Res<NetworkContext>,
) {
    libs.0.ensure_initialized();
    let font = FontSource::Handle(load_cn_font(&mut fonts));

    // 背景 Prguse[65] + 标题 Title[40]
    let bg = crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 65);
    let title = crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Title, 40);
    let empty_slot = crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 44);
    // 角色按钮帧 Title[660-669]：class*1 + selected+5
    let btn_frames: Vec<Option<Handle<Image>>> = (0..10usize)
        .map(|i| crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Title, 660 + i))
        .collect();
    // 底部按钮
    let start_btn = load_btn(&mut libs, &mut images, &[340, 341, 342]);
    let new_btn = load_btn(&mut libs, &mut images, &[343, 344, 345]);
    let del_btn = load_btn(&mut libs, &mut images, &[346, 347, 348]);

    // 预览帧
    anim.preview_handles.clear();
    if let Some(c) = net.characters.first() {
        let base = preview_base_index(c.class, c.gender);
        for i in 0..16usize {
            if let Some(h) =
                crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::ChrSel, base + i)
            {
                anim.preview_handles.push(h);
            }
        }
    }

    let screen_w = 1280.0f32;
    let x_point = (screen_w - 200.0) / 5.0;
    let bottom_y = 800.0 - 32.0;

    commands
        .spawn((
            SelectRoot,
            Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
            BackgroundColor(Color::BLACK),
        ))
        .with_children(|root| {
            if let Some(bg) = bg {
                root.spawn((
                    ImageNode { image: bg, ..default() },
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0), top: Val::Px(0.0),
                        width: Val::Percent(100.0), height: Val::Percent(100.0),
                        ..default()
                    },
                ));
            }
            if let Some(t) = title {
                root.spawn((
                    ImageNode { image: t, ..default() },
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(468.0), top: Val::Px(20.0),
                        ..default()
                    },
                ));
            }
            // 角色预览（左侧，scale 1.2）
            if let Some(pv) = anim.preview_handles.first().cloned() {
                root.spawn((
                    PreviewImg,
                    ImageNode { image: pv, ..default() },
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(260.0), top: Val::Px(420.0),
                        ..default()
                    },
                ));
            }
            // 角色信息
            root.spawn((
                InfoText,
                Text::new(""),
                TextFont { font: font.clone(), font_size: FontSize::Px(14.0), ..default() },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(200.0), top: Val::Px(623.0),
                    ..default()
                },
            ));
            // 角色按钮（右侧 4 个槽位）
            let positions = [(637.0f32, 194.0f32), (637.0, 298.0), (637.0, 402.0), (637.0, 506.0)];
            for (i, (x, y)) in positions.iter().enumerate() {
                let has = i < net.characters.len();
                let slot_img = if has {
                    let c = &net.characters[i];
                    let base = match c.class {
                        mir2_shared::MirClass::Warrior => 0usize,
                        mir2_shared::MirClass::Wizard => 1,
                        mir2_shared::MirClass::Taoist => 2,
                        mir2_shared::MirClass::Assassin => 3,
                        mir2_shared::MirClass::Archer => 4,
                    };
                    btn_frames.get(base).cloned().flatten()
                } else {
                    empty_slot.clone()
                };
                if let Some(img) = slot_img {
                    let is_selected = net.selected_index == Some(i as i32);
                    // 选中用 +5 帧（需要单独加载）
                    let final_img = if has && is_selected {
                        let c = &net.characters[i];
                        let base = match c.class {
                            mir2_shared::MirClass::Warrior => 0usize,
                            mir2_shared::MirClass::Wizard => 1,
                            mir2_shared::MirClass::Taoist => 2,
                            mir2_shared::MirClass::Assassin => 3,
                            mir2_shared::MirClass::Archer => 4,
                        };
                        btn_frames.get(base + 5).cloned().flatten().unwrap_or(img)
                    } else {
                        img
                    };
                    root.spawn((
                        CharButton(i as i32),
                        Button,
                        ImageNode { image: final_img, ..default() },
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(*x), top: Val::Px(*y),
                            width: Val::Px(280.0), height: Val::Px(90.0),
                            ..default()
                        },
                    ))
                    .with_children(|b| {
                        if let Some(c) = net.characters.get(i) {
                            let class_name = match c.class {
                                mir2_shared::MirClass::Warrior => "战士",
                                mir2_shared::MirClass::Wizard => "法师",
                                mir2_shared::MirClass::Taoist => "道士",
                                mir2_shared::MirClass::Assassin => "刺客",
                                mir2_shared::MirClass::Archer => "弓手",
                            };
                            b.spawn((
                                Text::new(format!("{}\nLv.{}  {}", c.name, c.level, class_name)),
                                TextFont { font: font.clone(), font_size: FontSize::Px(13.0), ..default() },
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(107.0), top: Val::Px(14.0),
                                    ..default()
                                },
                            ));
                        }
                    });
                }
            }
            // 底部按钮
            let btn_x = |k: f32| 100.0 + k - x_point / 2.0 - 50.0;
            if let Some(b) = start_btn { spawn_btn(root, b, StartButton, btn_x(x_point), bottom_y); }
            if let Some(b) = new_btn { spawn_btn(root, b, NewCharButton, btn_x(x_point * 2.0), bottom_y); }
            if let Some(b) = del_btn { spawn_btn(root, b, DeleteButton, btn_x(x_point * 3.0), bottom_y); }
        });
}

fn spawn_btn<M: Bundle>(
    parent: &mut ChildSpawnerCommands,
    b: (Handle<Image>, Handle<Image>, Handle<Image>),
    marker: M,
    x: f32,
    y: f32,
) {
    parent.spawn((
        marker,
        Button,
        ImageButton { normal: b.0.clone(), hover: b.1.clone(), pressed: b.2.clone() },
        ImageNode { image: b.0, ..default() },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(x), top: Val::Px(y),
            ..default()
        },
    ));
}

fn load_btn(
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    frames: &[usize; 3],
) -> Option<(Handle<Image>, Handle<Image>, Handle<Image>)> {
    let n = crate::ui::theme::load_lib_image(libs, images, LibraryName::Title, frames[0])?;
    let h = crate::ui::theme::load_lib_image(libs, images, LibraryName::Title, frames[1])?;
    let p = crate::ui::theme::load_lib_image(libs, images, LibraryName::Title, frames[2])?;
    Some((n, h, p))
}

fn cleanup_select_ui(mut commands: Commands, root: Query<Entity, With<SelectRoot>>) {
    for e in root.iter() {
        commands.entity(e).despawn();
    }
}

fn select_ui_system(
    mut net: ResMut<NetworkContext>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut anim: ResMut<SelectAnim>,
    chars: Query<(&Interaction, &CharButton)>,
    start: Query<&Interaction, (With<StartButton>, Without<CharButton>)>,
) {
    for (interaction, char_btn) in chars.iter() {
        if *interaction == Interaction::Pressed && net.selected_index != Some(char_btn.0) {
            net.selected_index = Some(char_btn.0);
            if let Some(c) = net.characters.iter().find(|c| c.index == char_btn.0) {
                let base = preview_base_index(c.class, c.gender);
                anim.preview_handles.clear();
                for i in 0..16usize {
                    if let Some(h) = crate::ui::theme::load_lib_image(
                        &mut libs, &mut images, LibraryName::ChrSel, base + i,
                    ) {
                        anim.preview_handles.push(h);
                    }
                }
                anim.preview_frame = 0;
            }
        }
    }
    if start.iter().any(|i| *i == Interaction::Pressed) {
        if let Some(idx) = net.selected_index {
            net.send_packet(&mir2_shared::packets::client::account::StartGame {
                character_index: idx,
            });
        }
    }
}

fn select_anim_system(
    mut anim: ResMut<SelectAnim>,
    time: Res<Time>,
    mut pv: Query<&mut ImageNode, With<PreviewImg>>,
) {
    anim.preview_timer += time.delta_secs();
    if anim.preview_timer >= 0.25 {
        anim.preview_timer = 0.0;
        anim.preview_frame = (anim.preview_frame + 1) % anim.preview_handles.len().max(1);
        if let Ok(mut node) = pv.single_mut() {
            if let Some(h) = anim.preview_handles.get(anim.preview_frame) {
                node.image = h.clone();
            }
        }
    }
}
