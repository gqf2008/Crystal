// ============================================================================
// SelectPlugin - 角色选择界面（bevy_ui 原生 UI）
// ============================================================================

use bevy::prelude::*;

use crate::network::NetworkContext;
use crate::scenes::AppState;
use crate::ui::theme::{colors, CN_FONT};

pub struct SelectPlugin;

impl Plugin for SelectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Select), setup_select_ui);
        app.add_systems(OnExit(AppState::Select), cleanup_select_ui);
        app.add_systems(
            Update,
            select_ui_system.run_if(in_state(AppState::Select)),
        );
    }
}

#[derive(Component)]
struct SelectRoot;
#[derive(Component)]
struct CharButton(i32);
#[derive(Component)]
struct EnterButton;

fn setup_select_ui(
    mut commands: Commands,
    assets: Res<AssetServer>,
    net: Res<NetworkContext>,
) {
    let font = FontSource::Handle(assets.load(CN_FONT));
    commands
        .spawn((
            SelectRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.05, 0.06, 0.09)),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(28.0)),
                    row_gap: Val::Px(10.0),
                    ..default()
                },
                BackgroundColor(colors::PANEL_BG),
            ))
            .with_children(|card| {
                card.spawn((
                    Text::new("选 择 角 色"),
                    TextFont {
                        font: font.clone(),
                        font_size: FontSize::Px(26.0),
                        ..default()
                    },
                    TextColor(colors::TITLE_GOLD),
                ));

                if net.characters.is_empty() {
                    card.spawn((
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
                        let label = format!(
                            "{}  Lv.{}  {}{}",
                            c.name,
                            c.level,
                            class_name,
                            match c.gender {
                                mir2_shared::MirGender::Male => "男",
                                _ => "女",
                            }
                        );
                        card.spawn((
                            CharButton(c.index),
                            Button,
                            Node {
                                width: Val::Px(280.0),
                                height: Val::Px(36.0),
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
                card.spawn((
                    EnterButton,
                    Button,
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(38.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(colors::BUTTON_BG),
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::new("进 入 游 戏"),
                        TextFont {
                            font: font.clone(),
                            font_size: FontSize::Px(18.0),
                            ..default()
                        },
                        TextColor(colors::TEXT),
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
    chars: Query<(&Interaction, &CharButton)>,
    mut char_bg: Query<(&CharButton, &mut BackgroundColor)>,
    enter: Query<&Interaction, (With<EnterButton>, Without<CharButton>)>,
) {
    // 选择角色
    for (interaction, char_btn) in chars.iter() {
        if *interaction == Interaction::Pressed {
            net.selected_index = Some(char_btn.0);
        }
    }
    // 高亮选中角色
    for (btn, mut bg) in char_bg.iter_mut() {
        *bg = if net.selected_index == Some(btn.0) {
            BackgroundColor(colors::BUTTON_PRESS)
        } else {
            BackgroundColor(colors::BUTTON_BG)
        };
    }
    // 进入游戏
    let pressed = enter.iter().any(|i| *i == Interaction::Pressed);
    if pressed {
        if let Some(idx) = net.selected_index {
            net.send_packet(&mir2_shared::packets::client::account::StartGame {
                character_index: idx,
            });
        }
    }
}
