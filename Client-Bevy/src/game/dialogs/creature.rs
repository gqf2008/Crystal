// ============================================================================
// 宠物对话框（M47）
// 参考：C# IntelligentCreatureDialog + ServerRust hero.rs 宠物系统
// 网络（ServerRust 实际 wire）：
//   C: RequestIntelligentCreatureUpdates[bool u8] / UpdateIntelligentCreature[type u8][pickup u8]
//   S: UpdateIntelligentCreatureList[count i32][per: type u8][pickup u8][enabled u8][hunger u8][name dotnet]
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::text_input::{TextInputDisplay, TextInputField, TextInputRect, TextInputState, TextInputSubmit};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};

/// 宠物条目
#[derive(Debug, Clone, Default)]
pub struct CreatureEntry {
    pub creature_type: u8,
    pub pickup_mode: u8,
    pub enabled: bool,
    pub hunger: u8,
    pub name: String,
    /// 是否当前激活（召唤中）
    pub active: bool,
}

/// 宠物状态
#[derive(Resource, Default)]
pub struct CreatureState {
    pub creatures: Vec<CreatureEntry>,
    pub message: String,
    /// 当前选中的宠物行（C# BeforeAfterDraw 选中语义）
    pub selected: usize,
    /// 改名输入框是否打开（C# CreatureRenameButton → MirInputBox）
    pub rename_open: bool,
    /// 释放验证输入框是否打开（C# ReleaseButton → MirInputBox）
    pub release_open: bool,
}

#[derive(Component)]
pub struct CreatureWidget;

#[derive(Component)]
pub struct CreatureClose;

#[derive(Component)]
pub struct CreatureRefresh;

/// 改名按钮（C# CreatureRenameButton Title[570-572]）
#[derive(Component)] struct CreatureRenameBtn;

/// 解散按钮（C# DismissButton Title[580-582]）
#[derive(Component)] struct CreatureDismissBtn;

/// 召唤按钮（C# SummonButton Title[576-578]，选中未激活宠物时显示）
#[derive(Component)] struct CreatureSummonBtn;

/// 释放按钮（C# ReleaseButton Title[583-585]）
#[derive(Component)] struct CreatureReleaseBtn;

/// 自动模式按钮（C# AutomaticModeButton）
#[derive(Component)] struct CreatureAutoBtn;

/// 半自动模式按钮（C# SemiAutoModeButton）
#[derive(Component)] struct CreatureSemiBtn;

/// 改名输入框（TextInput id 33）
#[derive(Component)] struct CreatureRenameInput;

/// 改名确认
#[derive(Component)] struct CreatureRenameOk;

/// 释放验证输入框（TextInput id 34）
#[derive(Component)] struct CreatureReleaseInput;

/// 释放确认
#[derive(Component)] struct CreatureReleaseOk;

#[derive(Component)]
pub struct CreatureLine(usize);

pub struct CreaturePlugin;

impl Plugin for CreaturePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CreatureState>();
                app.add_systems(
            Update,
            creature_server_events.run_if(in_state(AppState::Game)),
        );
app.add_systems(OnEnter(AppState::Game), spawn_creature);
        app.add_systems(OnExit(AppState::Game), cleanup_creature);
        app.add_systems(
            Update,
            (creature_ui_system, creature_action_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_creature(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_creature(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 170) {
        let e = spawn_ui_sprite(&mut commands, h, 280.0, 80.0, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Creature),
            CreatureWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        280.0 + 300.0, 83.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            CreatureClose,
            DialogRoot(DialogKind::Creature),
            CreatureWidget,
        ));
    }
    // 8 行宠物 + 2 状态行
    for i in 0..10usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            298.0, 120.0 + i as f32 * 22.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            CreatureLine(i),
            DialogRoot(DialogKind::Creature),
            CreatureWidget,
        ));
    }
    // 刷新按钮
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        480.0, 345.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            CreatureRefresh,
            DialogRoot(DialogKind::Creature),
            CreatureWidget,
        ));
    }

    // 操作按钮（C# IntelligentCreatureDialog：改名/解散/释放/自动/半自动）
    let e = spawn_ui_text(
        &mut commands, &font, "改名",
        298.0, 305.0,
        12.0, Color::WHITE, 8.0,
    );
    commands.entity(e).insert((CreatureRenameBtn, UiButton { rect: (298.0, 305.0, 44.0, 22.0), clicked: false }, DialogRoot(DialogKind::Creature), CreatureWidget));
    let e = spawn_ui_text(
        &mut commands, &font, "解散",
        360.0, 305.0,
        12.0, Color::WHITE, 8.0,
    );
    commands.entity(e).insert((CreatureDismissBtn, UiButton { rect: (360.0, 305.0, 44.0, 22.0), clicked: false }, DialogRoot(DialogKind::Creature), CreatureWidget));
    let e = spawn_ui_text(
        &mut commands, &font, "召唤",
        360.0, 305.0,
        12.0, Color::WHITE, 8.0,
    );
    commands.entity(e).insert((CreatureSummonBtn, UiButton { rect: (360.0, 305.0, 44.0, 22.0), clicked: false }, DialogRoot(DialogKind::Creature), CreatureWidget, Visibility::Hidden));
    let e = spawn_ui_text(
        &mut commands, &font, "释放",
        420.0, 305.0,
        12.0, Color::WHITE, 8.0,
    );
    commands.entity(e).insert((CreatureReleaseBtn, UiButton { rect: (420.0, 305.0, 44.0, 22.0), clicked: false }, DialogRoot(DialogKind::Creature), CreatureWidget));
    let e = spawn_ui_text(
        &mut commands, &font, "自动",
        298.0, 330.0,
        12.0, Color::WHITE, 8.0,
    );
    commands.entity(e).insert((CreatureAutoBtn, UiButton { rect: (298.0, 330.0, 44.0, 22.0), clicked: false }, DialogRoot(DialogKind::Creature), CreatureWidget));
    let e = spawn_ui_text(
        &mut commands, &font, "半自动",
        360.0, 330.0,
        12.0, Color::WHITE, 8.0,
    );
    commands.entity(e).insert((CreatureSemiBtn, UiButton { rect: (360.0, 330.0, 44.0, 22.0), clicked: false }, DialogRoot(DialogKind::Creature), CreatureWidget));

    // 改名/释放输入框（TextInput id 33/34，C# MirInputBox 语义）
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    spawn_creature_input(&mut commands, &white, &font, 33, CreatureRenameInput, "确认改名", CreatureRenameOk);
    spawn_creature_input(&mut commands, &white, &font, 34, CreatureReleaseInput, "确认释放", CreatureReleaseOk);
}

/// 宠物输入框（TextInputField(id) + 子 TextInputDisplay(id) + 确认按钮，C# MirInputBox 语义）
fn spawn_creature_input(
    commands: &mut Commands,
    white: &Handle<Image>,
    font: &Handle<Font>,
    id: usize,
    input_comp: impl Component,
    ok_label: &str,
    ok_comp: impl Component,
) {
    let box_e = commands
        .spawn((
            crate::ui::sprite_ui::UiEntity,
            DialogRoot(DialogKind::Creature),
            CreatureWidget,
            input_comp,
            TextInputField(id),
            TextInputRect(298.0, 350.0, 120.0, 20.0),
            Sprite {
                image: white.clone(),
                color: Color::srgba(0.2, 0.2, 0.25, 0.9),
                custom_size: Some(Vec2::new(120.0, 20.0)),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(298.0, -350.0, 8.1),
            Visibility::Hidden,
        ))
        .id();
    commands.entity(box_e).with_children(|p| {
        p.spawn((
            TextInputDisplay(id),
            Text2d::new(String::new()),
            bevy::sprite::Anchor::TOP_LEFT,
            TextFont {
                font: FontSource::Handle(font.clone()),
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::WHITE),
            Transform::from_xyz(4.0, -2.0, 8.2),
        ));
    });
    let ok = spawn_ui_text(commands, font, ok_label, 425.0, 350.0, 12.0, Color::WHITE, 8.0);
    commands.entity(ok).insert((
        ok_comp,
        UiButton { rect: (425.0, 350.0, 60.0, 20.0), clicked: false },
        DialogRoot(DialogKind::Creature),
        CreatureWidget,
        Visibility::Hidden,
    ));
}

/// 显隐 + 渲染 + 刷新
#[allow(clippy::too_many_arguments)]
fn creature_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<CreatureState>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    close: Query<&UiButton, With<CreatureClose>>,
    refresh_btn: Query<&UiButton, With<CreatureRefresh>>,
    mut widgets: Query<&mut Visibility, With<CreatureWidget>>,
    mut lines: Query<(&mut Text2d, &CreatureLine)>,
    mut requested: Local<bool>,
) {
    let open = mgr.is_open(DialogKind::Creature);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        *requested = false;
        return;
    }
    // 打开瞬间请求宠物列表
    if !*requested {
        *requested = true;
        net.send_packet(&crate::network::CreatureRequestWire { request: true });
        tracing::info!("🐾 请求宠物列表");
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Creature);
        }
    }
    // 行点击选中（C# BeforeAfterDraw 选中语义）
    if mouse.just_pressed(MouseButton::Left) {
        if let Ok(window) = windows.single() {
            if let Some(cursor) = window.cursor_position() {
                for i in 0..8usize {
                    let y = 120.0 + i as f32 * 22.0;
                    if cursor.x >= 298.0 && cursor.x <= 540.0 && cursor.y >= y && cursor.y <= y + 20.0 {
                        if i < state.creatures.len() {
                            state.selected = i;
                            state.message = format!("选中宠物 {}", state.creatures[i].name);
                        }
                        break;
                    }
                }
            }
        }
    }
    for (mut text, line) in &mut lines {
        text.0 = match line.0 {
            i if i < 8 => match state.creatures.get(i) {
                Some(c) => format!(
                    "{} {}（类型 {}）模式:{} 饥饿:{}",
                    if state.selected == i { ">" } else { " " },
                    if c.name.is_empty() { format!("#{}", c.creature_type) } else { c.name.clone() },
                    c.creature_type,
                    if c.pickup_mode == 0 { "自动" } else { "半自动" },
                    c.hunger
                ),
                None => String::new(),
            },
            8 => format!("宠物: {} 个", state.creatures.len()),
            9 => state.message.clone(),
            _ => String::new(),
        };
    }
    for btn in &refresh_btn {
        if btn.clicked {
            net.send_packet(&crate::network::CreatureRequestWire { request: true });
            state.message = "已请求刷新".to_string();
            tracing::info!("🐾 刷新宠物列表");
        }
    }
}


/// 宠物操作（C# IntelligentCreatureDialog ButtonClick：改名/召唤/解散/释放/自动/半自动）
#[allow(clippy::too_many_arguments)]
fn creature_action_system(
    mut state: ResMut<CreatureState>,
    net: Res<NetConnection>,
    mut input: ResMut<TextInputState>,
    mut submit: MessageReader<TextInputSubmit>,
    buttons: Query<(
        &UiButton,
        Has<CreatureRenameBtn>,
        Has<CreatureDismissBtn>,
        Has<CreatureSummonBtn>,
        Has<CreatureReleaseBtn>,
        Has<CreatureAutoBtn>,
        Has<CreatureSemiBtn>,
        Has<CreatureRenameOk>,
        Has<CreatureReleaseOk>,
    )>,
    mut btn_vis: Query<(&mut Visibility, Has<CreatureDismissBtn>, Has<CreatureSummonBtn>)>,
    mut input_vis: Query<(
        &mut Visibility,
        Has<CreatureRenameInput>,
        Has<CreatureReleaseInput>,
        Has<CreatureRenameOk>,
        Has<CreatureReleaseOk>,
    )>,
) {
    let selected = state.creatures.get(state.selected).cloned();
    let creature_type = selected.as_ref().map(|c| c.creature_type).unwrap_or(0);
    let pet_mode = selected.as_ref().map(|c| c.pickup_mode).unwrap_or(0);
    let is_active = selected.as_ref().map(|c| c.active).unwrap_or(false);
    let sel_name = selected.as_ref().map(|c| c.name.clone()).unwrap_or_default();

    // 解散仅对激活宠物显示；召唤对未激活的选中宠物显示（C# Summon/Dismiss 同位置切换）
    for (mut vis, is_dismiss, is_summon) in &mut btn_vis {
        if is_dismiss {
            *vis = if is_active { Visibility::Visible } else { Visibility::Hidden };
        } else if is_summon {
            *vis = if selected.is_some() && !is_active { Visibility::Visible } else { Visibility::Hidden };
        }
    }
    for (mut vis, is_ri, is_reli, is_rok, is_relok) in &mut input_vis {
        if is_ri {
            *vis = if state.rename_open { Visibility::Visible } else { Visibility::Hidden };
        }
        if is_reli {
            *vis = if state.release_open { Visibility::Visible } else { Visibility::Hidden };
        }
        if is_rok {
            *vis = if state.rename_open { Visibility::Visible } else { Visibility::Hidden };
        }
        if is_relok {
            *vis = if state.release_open { Visibility::Visible } else { Visibility::Hidden };
        }
    }

    let submits: Vec<usize> = submit.read().map(|s| s.0).collect();
    let mut rename_confirm = false;
    let mut release_confirm = false;
    for (btn, is_rename, is_dismiss, is_summon, is_release, is_auto, is_semi, is_rok, is_relok) in &buttons {
        if !btn.clicked {
            continue;
        }
        if is_rename {
            state.rename_open = true;
            state.release_open = false;
            input.active = Some(33);
        } else if is_dismiss {
            net.send_packet(&mir2_shared::packets::client::misc::UpdateIntelligentCreature {
                creature_type,
                pet_mode,
                custom_name: String::new(),
                summon_me: false,
                unsummon_me: true,
                release_me: false,
            });
            state.message = "已解散宠物".to_string();
        } else if is_summon {
            net.send_packet(&mir2_shared::packets::client::misc::UpdateIntelligentCreature {
                creature_type,
                pet_mode,
                custom_name: String::new(),
                summon_me: true,
                unsummon_me: false,
                release_me: false,
            });
            state.message = format!("已召唤宠物 {}", sel_name);
        } else if is_release {
            state.release_open = true;
            state.rename_open = false;
            input.active = Some(34);
        } else if is_auto {
            net.send_packet(&mir2_shared::packets::client::misc::UpdateIntelligentCreature {
                creature_type,
                pet_mode: 0,
                custom_name: String::new(),
                summon_me: false,
                unsummon_me: false,
                release_me: false,
            });
            state.message = "切换到自动模式".to_string();
        } else if is_semi {
            net.send_packet(&mir2_shared::packets::client::misc::UpdateIntelligentCreature {
                creature_type,
                pet_mode: 1,
                custom_name: String::new(),
                summon_me: false,
                unsummon_me: false,
                release_me: false,
            });
            state.message = "切换到半自动模式".to_string();
        } else if is_rok {
            rename_confirm = true;
        } else if is_relok {
            release_confirm = true;
        }
    }
    if submits.contains(&33) {
        rename_confirm = true;
    }
    if submits.contains(&34) {
        release_confirm = true;
    }
    // 改名确认（C# CreatureRenameButton → MirInputBox → UpdateIntelligentCreature.CustomName）
    if rename_confirm && state.rename_open {
        let name = input.texts.get(33).cloned().unwrap_or_default();
        let name = name.trim().to_string();
        if !name.is_empty() {
            net.send_packet(&mir2_shared::packets::client::misc::UpdateIntelligentCreature {
                creature_type,
                pet_mode,
                custom_name: name.clone(),
                summon_me: false,
                unsummon_me: false,
                release_me: false,
            });
            state.message = format!("已改名为 {}", name);
        }
        state.rename_open = false;
        if input.texts.len() > 33 { input.texts[33].clear(); }
    }
    // 释放确认（C# ReleaseButton → 输入宠物名验证 → ReleaseMe）
    if release_confirm && state.release_open {
        let name = input.texts.get(34).cloned().unwrap_or_default();
        let name = name.trim().to_string();
        if name.eq_ignore_ascii_case(&sel_name) {
            net.send_packet(&mir2_shared::packets::client::misc::UpdateIntelligentCreature {
                creature_type,
                pet_mode,
                custom_name: String::new(),
                summon_me: false,
                unsummon_me: false,
                release_me: true,
            });
            state.message = "宠物已释放".to_string();
        } else {
            state.message = "验证失败：名字不匹配".to_string();
        }
        state.release_open = false;
        if input.texts.len() > 34 { input.texts[34].clear(); }
    }
}
/// 消费服务端宠物列表事件（网络层只广播 ServerEvent）
fn creature_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut creature: ResMut<CreatureState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::CreatureList { creatures } = ev {
            creature.creatures = creatures.clone();
            // #619：列表更新提示（--creature-test 依赖）
            creature.message = format!("宠物列表已更新（{} 个）", creatures.len());
        }
        if let ServerEvent::CreatureAcquired { creature_type } = ev {
            // #274：获得新宠物
            if !creature
                .creatures
                .iter()
                .any(|c| c.creature_type == *creature_type)
            {
                creature.creatures.push(CreatureEntry {
                    creature_type: *creature_type,
                    ..Default::default()
                });
            }
            creature.message = format!("获得新宠物（type {}）", creature_type);
        }
        if let ServerEvent::CreatureRenameEnabled { can_rename } = ev {
            creature.message = format!("宠物{}重命名", if *can_rename { "可以" } else { "不可" });
        }
        if let ServerEvent::CreaturePickupToggled { enabled } = ev {
            creature.message = format!("宠物拾取模式: {}", if *enabled { "开启" } else { "关闭" });
        }
    }
}
