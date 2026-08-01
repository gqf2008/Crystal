// ============================================================================
// SelectPlugin - 角色选择界面（对齐 macroquad SelectScene）
// ============================================================================
// ChrSel 动画背景 + 角色预览（ChrSel[base_index+frame]，16帧/0.25s）
// + 角色列表按钮 + 进入游戏。

use bevy::prelude::*;

use crate::map_renderer::GameLibraries;
use crate::network::NetworkContext;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::theme::{colors, load_cn_font};

pub struct SelectPlugin;

impl Plugin for SelectPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectAnim>();
        app.add_systems(OnEnter(AppState::Select), setup_select_ui);
        app.add_systems(OnExit(AppState::Select), cleanup_select_ui);
        app.add_systems(
            Update,
            (select_ui_system, select_anim_system).run_if(in_state(AppState::Select)),
        );
    }
}

#[derive(Component)]
struct SelectRoot;
#[derive(Component)]
struct SelectBg;
#[derive(Component)]
struct PreviewImg;
#[derive(Component)]
struct CharButton(i32);
#[derive(Component)]
struct EnterButton;
#[derive(Component)]
struct InfoText;

/// 选角动画状态（背景 + 角色预览）
#[derive(Resource, Default)]
pub struct SelectAnim {
    pub bg_frame: usize,
    pub bg_timer: f32,
    pub bg_handles: Vec<Handle<Image>>,
    pub preview_frame: usize,
    pub preview_timer: f32,
    pub preview_handles: Vec<Handle<Image>>,
}

/// 角色预览基址（对齐 macroquad draw_character_preview）
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

    // 背景帧 ChrSel[0..18]
    anim.bg_handles.clear();
    for i in 0..19usize {
        if let Some(h) =
            crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::ChrSel, i)
        {
            anim.bg_handles.push(h);
        }
    }
    // 预览帧：当前选中角色 base_index + 0..16
    anim.preview_handles.clear();
    if let Some(c) = net.characters.first() {
        let base = preview_base_index(c.class, c.gender);
        for i in 0..16usize {
            if let Some(h) = crate::ui::theme::load_lib_image(
                &mut libs,
                &mut images,
                LibraryName::ChrSel,
                base + i,
            ) {
                anim.preview_handles.push(h);
            }
        }
    }

    commands
        .spawn((
            SelectRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::BLACK),
        ))
        .with_children(|root| {
            // ChrSel 背景
            if let Some(bg) = anim.bg_handles.first().cloned() {
                root.spawn((
                    SelectBg,
                    ImageNode { image: bg, ..default() },
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                ));
            }
            // 角色预览（左侧大图）
            if let Some(pv) = anim.preview_handles.first().cloned() {
                root.spawn((
                    PreviewImg,
                    ImageNode { image: pv, ..default() },
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(140.0),
                        top: Val::Px(180.0),
                        ..default()
                    },
                ));
            }
            // 角色信息
            root.spawn((
                InfoText,
                Text::new(""),
                TextFont {
                    font: font.clone(),
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(colors::TEXT),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(300.0),
                    top: Val::Px(180.0),
                    ..default()
                },
            ));
            // 角色列表（右侧按钮）
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(300.0),
                    top: Val::Px(240.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    ..default()
                },
            ))
            .with_children(|list| {
                if net.characters.is_empty() {
                    list.spawn((
                        Text::new("暂无角色"),
                        TextFont {
                            font: font.clone(),
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor(colors::TEXT),
                    ));
                } else {
                    for c in net.characters.iter() {
                        let class_name = match c.class {
                            mir2_shared::MirClass::Warrior => "战士",
                            mir2_shared::MirClass::Wizard => "法师",
                            mir2_shared::MirClass::Taoist => "道士",
                            mir2_shared::MirClass::Assassin => "刺客",
                            mir2_shared::MirClass::Archer => "弓箭手",
                        };
                        let gender_name = match c.gender {
                            mir2_shared::MirGender::Male => "男",
                            _ => "女",
                        };
                        let label =
                            format!("{}  Lv.{}  {}{}", c.name, c.level, class_name, gender_name);
                        list.spawn((
                            CharButton(c.index),
                            Button,
                            Node {
                                width: Val::Px(240.0),
                                height: Val::Px(34.0),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(colors::BUTTON_BG),
                        ))
                        .with_children(|b| {
                            b.spawn((
                                Text::new(label),
                                TextFont {
                                    font: font.clone(),
                                    font_size: FontSize::Px(15.0),
                                    ..default()
                                },
                                TextColor(colors::TEXT),
                            ));
                        });
                    }
                }
                // 进入游戏按钮
                list.spawn((
                    EnterButton,
                    Button,
                    Node {
                        width: Val::Px(240.0),
                        height: Val::Px(36.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        margin: UiRect::top(Val::Px(16.0)),
                        ..default()
                    },
                    BackgroundColor(colors::TITLE_GOLD),
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::new("进 入 游 戏"),
                        TextFont {
                            font: font.clone(),
                            font_size: FontSize::Px(18.0),
                            ..default()
                        },
                        TextColor(Color::BLACK),
                    ));
                });
            });
        });
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
    mut char_bg: Query<(&CharButton, &mut BackgroundColor)>,
    enter: Query<&Interaction, (With<EnterButton>, Without<CharButton>)>,
    mut info: Query<&mut Text, (With<InfoText>, Without<CharButton>)>,
) {
    // 选择角色 → 更新预览
    for (interaction, char_btn) in chars.iter() {
        if *interaction == Interaction::Pressed && net.selected_index != Some(char_btn.0) {
                net.selected_index = Some(char_btn.0);
                // 重载预览帧
                if let Some(c) = net.characters.iter().find(|c| c.index == char_btn.0) {
                    let base = preview_base_index(c.class, c.gender);
                    anim.preview_handles.clear();
                    for i in 0..16usize {
                        if let Some(h) = crate::ui::theme::load_lib_image(
                            &mut libs,
                            &mut images,
                            LibraryName::ChrSel,
                            base + i,
                        ) {
                            anim.preview_handles.push(h);
                        }
                    }
                    anim.preview_frame = 0;
                }
        }
    }
    // 高亮选中
    for (btn, mut bg) in char_bg.iter_mut() {
        *bg = if net.selected_index == Some(btn.0) {
            BackgroundColor(colors::BUTTON_PRESS)
        } else {
            BackgroundColor(colors::BUTTON_BG)
        };
    }
    // 进入游戏
    if enter.iter().any(|i| *i == Interaction::Pressed) {
        if let Some(idx) = net.selected_index {
            net.send_packet(&mir2_shared::packets::client::account::StartGame {
                character_index: idx,
            });
        }
    }
    // 角色信息
    if let Ok(mut t) = info.single_mut() {
        if let Some(c) = net
            .characters
            .iter()
            .find(|c| Some(c.index) == net.selected_index)
        {
            t.0 = format!("{}  Lv.{}", c.name, c.level);
        }
    }
}

/// 背景 + 预览动画
fn select_anim_system(
    mut anim: ResMut<SelectAnim>,
    time: Res<Time>,
    mut bg: Query<&mut ImageNode, (With<SelectBg>, Without<PreviewImg>)>,
    mut pv: Query<&mut ImageNode, (With<PreviewImg>, Without<SelectBg>)>,
) {
    anim.bg_timer += time.delta_secs();
    if anim.bg_timer >= 0.15 {
        anim.bg_timer = 0.0;
        anim.bg_frame = (anim.bg_frame + 1) % anim.bg_handles.len().max(1);
        if let Ok(mut node) = bg.single_mut() {
            if let Some(h) = anim.bg_handles.get(anim.bg_frame) {
                node.image = h.clone();
            }
        }
    }
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
