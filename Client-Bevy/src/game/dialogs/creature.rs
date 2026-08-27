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
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_icon_button, spawn_label, spawn_panel,
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
    /// 物品过滤 9 项（全部/金币/武器/盔甲/头盔/靴子/腰带/饰品/其他）
    pub filter: [u8; 9],
    /// 品质
    pub grade: u8,
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
    /// 选项面板是否打开（C# IntelligentCreatureOptionsDialog）
    pub options_open: bool,
    /// 选项面板中的过滤勾选（0=全部 ... 8=其他）
    pub options: [bool; 9],
    /// 品质（C# ItemGrade；本批仅保存，暂不做品质选择 UI）
    pub grade: u8,
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

/// 选项按钮（C# OptionsMenuButton Title[573-575]）
#[derive(Component)] struct CreatureOptionsBtn;

/// 选项面板（C# IntelligentCreatureOptionsDialog：9 个过滤复选框 + 保存/取消）
#[derive(Component)] struct CreatureOptionsWidget;

/// 选项行（0=全部 ... 8=其他）
#[derive(Component)] struct CreatureOptionsLine(usize);

/// 选项保存
#[derive(Component)] struct CreatureOptionsSave;

/// 选项取消
#[derive(Component)] struct CreatureOptionsCancel;

/// 品质上一档（C# OptionsGradeDialog PrevButton）
#[derive(Component)] struct CreatureGradePrev;

/// 品质下一档（C# OptionsGradeDialog NextButton）
#[derive(Component)] struct CreatureGradeNext;

/// 品质显示（C# GradeLabel）
#[derive(Component)] struct CreatureGradeText;

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
            (creature_ui_system, creature_action_system, creature_options_system)
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
    mut fonts: ResMut<Assets<Font>>,
    mut cjk_font: ResMut<UiCjkFont>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();
    let cjk = shared_cjk_font(&mut fonts, &mut cjk_font);

    // 面板 Prguse[170] @ (280,80)。加宽加高到 320x320：操作按钮/选项面板/输入框
    // 全在面板内（旧 sprite 布局底部元素 rel y=225-270 悬空 207 高面板外）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 170) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, 280.0, 80.0, 320.0, 320.0, 30);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::Creature), CreatureWidget));

    commands.entity(panel).with_children(|p| {
        // 关闭 Prguse2[360/361/362] @(300,3)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 300.0, 3.0, 20.0, 20.0, 10).insert(CreatureClose);
        }
        // 8 行宠物 + 2 状态行 @(18,40+22i)
        for i in 0..10usize {
            spawn_label(p, &cjk, "", 18.0, 40.0 + i as f32 * 22.0, 12.0, Color::WHITE, 9)
                .insert(CreatureLine(i));
        }
        // 刷新按钮 @(200,265)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
        ) {
            spawn_icon_button(p, n, h, pr, 200.0, 265.0, 76.0, 25.0, 10).insert(CreatureRefresh);
        }
        // 操作文本按钮（改名/解散/召唤/释放/自动/半自动/选项）
        for (x, y, marker, text) in [
            (18.0, 225.0, "rename", "改名"),
            (80.0, 225.0, "dismiss", "解散"),
            (80.0, 225.0, "summon", "召唤"),
            (140.0, 225.0, "release", "释放"),
            (18.0, 250.0, "auto", "自动"),
            (80.0, 250.0, "semi", "半自动"),
            (140.0, 250.0, "opts", "选项"),
        ] {
            let mut cmds = spawn_container(p, x, y, 44.0, 22.0, 10);
            cmds.insert((
                Button,
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            ));
            match marker {
                "rename" => cmds.insert(CreatureRenameBtn),
                "dismiss" => cmds.insert(CreatureDismissBtn),
                "summon" => cmds.insert((CreatureSummonBtn, Visibility::Hidden)),
                "release" => cmds.insert(CreatureReleaseBtn),
                "auto" => cmds.insert(CreatureAutoBtn),
                "semi" => cmds.insert(CreatureSemiBtn),
                _ => cmds.insert(CreatureOptionsBtn),
            };
            cmds.with_children(|c| {
                spawn_label(c, &font, text, 0.0, 5.0, 12.0, Color::WHITE, 11);
            });
        }
        // 选项面板覆盖层（C# IntelligentCreatureOptionsDialog：9 个过滤项 + 保存/取消 + 品质）
        for i in 0..9usize {
            spawn_container(p, 20.0, 40.0 + i as f32 * 22.0, 200.0, 20.0, 10)
                .insert((
                    Button,
                    CreatureOptionsWidget,
                    CreatureOptionsLine(i),
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                    Visibility::Hidden,
                ))
                .with_children(|c| {
                    spawn_label(c, &font, "", 0.0, 4.0, 12.0, Color::WHITE, 11);
                });
        }
        spawn_container(p, 20.0, 255.0, 44.0, 22.0, 10)
            .insert((
                Button,
                CreatureOptionsWidget,
                CreatureOptionsSave,
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                Visibility::Hidden,
            ))
            .with_children(|c| { spawn_label(c, &font, "保存", 0.0, 5.0, 12.0, Color::WHITE, 11); });
        spawn_container(p, 80.0, 255.0, 44.0, 22.0, 10)
            .insert((
                Button,
                CreatureOptionsWidget,
                CreatureOptionsCancel,
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                Visibility::Hidden,
            ))
            .with_children(|c| { spawn_label(c, &font, "取消", 0.0, 5.0, 12.0, Color::WHITE, 11); });
        spawn_label(p, &cjk, "品质:全部", 20.0, 280.0, 12.0, Color::WHITE, 11)
            .insert((CreatureOptionsWidget, CreatureGradeText, Visibility::Hidden));
        for (x, marker, text) in [(100.0, "prev", "◀"), (130.0, "next", "▶")] {
            let mut cmds = spawn_container(p, x, 280.0, 20.0, 20.0, 10);
            cmds.insert((
                Button,
                CreatureOptionsWidget,
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                Visibility::Hidden,
            ));
            if marker == "prev" {
                cmds.insert(CreatureGradePrev);
            } else {
                cmds.insert(CreatureGradeNext);
            }
            cmds.with_children(|c| { spawn_label(c, &font, text, 0.0, 4.0, 12.0, Color::WHITE, 11); });
        }
        // 改名/释放输入框（TextInput id 33/34，C# MirInputBox 语义）@(18,270) + 确认 @(145,270)
        spawn_creature_input(p, &mut images, &font, 33, CreatureRenameInput, "确认改名", CreatureRenameOk);
        spawn_creature_input(p, &mut images, &font, 34, CreatureReleaseInput, "确认释放", CreatureReleaseOk);
    });
}

/// 宠物输入框（TextInputField(id) + 子 TextInputDisplay(id) + 确认按钮，C# MirInputBox 语义）
#[allow(clippy::too_many_arguments)]
fn spawn_creature_input(
    parent: &mut bevy::ecs::hierarchy::ChildSpawnerCommands,
    images: &mut Assets<Image>,
    font: &Handle<Font>,
    id: usize,
    input_comp: impl Component,
    ok_label: &str,
    ok_comp: impl Component,
) {
    spawn_container(parent, 18.0, 270.0, 120.0, 20.0, 10)
        .insert((
            input_comp,
            crate::game::dialogs::text_input::TextInputField(id),
            crate::game::dialogs::text_input::TextInputRect(298.0, 350.0, 120.0, 20.0),
            BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
            Visibility::Hidden,
        ))
        .with_children(|ic| {
            ic.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(4.0),
                    top: Val::Px(2.0),
                    ..default()
                },
                Text::new(String::new()),
                TextFont {
                    font: FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                ZIndex(11),
                crate::game::dialogs::text_input::TextInputDisplay(id),
            ));
        });
    spawn_container(parent, 145.0, 270.0, 60.0, 20.0, 10)
        .insert((
            Button,
            ok_comp,
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            Visibility::Hidden,
        ))
        .with_children(|c| { spawn_label(c, font, ok_label, 0.0, 4.0, 12.0, Color::WHITE, 11); });
    let _ = images;
}

/// 显隐 + 渲染 + 刷新
#[allow(clippy::too_many_arguments)]
fn creature_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<CreatureState>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    close: Query<(Entity, &Interaction), With<CreatureClose>>,
    refresh_btn: Query<(Entity, &Interaction), With<CreatureRefresh>>,
    mut widgets: Query<&mut Visibility, With<CreatureWidget>>,
    mut lines: Query<(&mut Text, &CreatureLine)>,
    mut requested: Local<bool>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
    panel_origin: Query<&Node, With<CreatureWidget>>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
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
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::Creature);
        }
    }
    // 行点击选中（C# BeforeAfterDraw 选中语义）
    if mouse.just_pressed(MouseButton::Left) {
        if let Ok(window) = windows.single() {
            if let Some(cursor) = window.cursor_position() {
                let (ox, oy) = panel_origin
                    .single()
                    .map(|n| crate::ui::theme::node_origin(n, (280.0, 80.0)))
                    .unwrap_or((280.0, 80.0));
                for i in 0..8usize {
                    let y = oy + 40.0 + i as f32 * 22.0;
                    if cursor.x >= ox + 18.0 && cursor.x <= ox + 260.0 && cursor.y >= y && cursor.y <= y + 20.0 {
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
    for (e, inter) in &refresh_btn {
        if edge(e, inter, &mut prev_inter) {
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
        Entity,
        &Interaction,
        Has<CreatureRenameBtn>,
        Has<CreatureDismissBtn>,
        Has<CreatureSummonBtn>,
        Has<CreatureReleaseBtn>,
        Has<CreatureAutoBtn>,
        Has<CreatureSemiBtn>,
        Has<CreatureRenameOk>,
        Has<CreatureReleaseOk>,
    )>,
    // #1299：Bevy B0001——两个 &mut Visibility Query 冲突，用 ParamSet 顺序访问（#1298 合并后启动 panic）
    mut vis: ParamSet<(
        Query<(
            &mut Visibility,
            Has<CreatureDismissBtn>,
            Has<CreatureSummonBtn>,
        )>,
        Query<(
            &mut Visibility,
            Has<CreatureRenameInput>,
            Has<CreatureReleaseInput>,
            Has<CreatureRenameOk>,
            Has<CreatureReleaseOk>,
        )>,
    )>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }

    let selected = state.creatures.get(state.selected).cloned();
    let creature_type = selected.as_ref().map(|c| c.creature_type).unwrap_or(0);
    let pet_mode = selected.as_ref().map(|c| c.pickup_mode).unwrap_or(0);
    let is_active = selected.as_ref().map(|c| c.active).unwrap_or(false);
    let sel_name = selected.as_ref().map(|c| c.name.clone()).unwrap_or_default();

    // 解散仅对激活宠物显示；召唤对未激活的选中宠物显示（C# Summon/Dismiss 同位置切换）
    for (mut vis, is_dismiss, is_summon) in &mut vis.p0() {
        if is_dismiss {
            *vis = if is_active { Visibility::Visible } else { Visibility::Hidden };
        } else if is_summon {
            *vis = if selected.is_some() && !is_active { Visibility::Visible } else { Visibility::Hidden };
        }
    }
    for (mut vis, is_ri, is_reli, is_rok, is_relok) in &mut vis.p1() {
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
    for (e, inter, is_rename, is_dismiss, is_summon, is_release, is_auto, is_semi, is_rok, is_relok) in &buttons {
        if !edge(e, inter, &mut prev_inter) {
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
                filter: [0; 9],
                grade: 0,
                options_save: false,
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
                filter: [0; 9],
                grade: 0,
                options_save: false,
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
                filter: [0; 9],
                grade: 0,
                options_save: false,
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
                filter: [0; 9],
                grade: 0,
                options_save: false,
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
                filter: [0; 9],
                grade: 0,
                options_save: false,
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
                filter: [0; 9],
                grade: 0,
                options_save: false,
            });
            state.message = "宠物已释放".to_string();
        } else {
            state.message = "验证失败：名字不匹配".to_string();
        }
        state.release_open = false;
        if input.texts.len() > 34 { input.texts[34].clear(); }
    }
}
/// 选项标签（C# IntelligentCreatureOptionsDialog.OptionNames）
const CREATURE_OPTION_LABELS: [&str; 9] = ["全部", "金币", "武器", "盔甲", "头盔", "靴子", "腰带", "饰品", "其他"];
/// 品质标签（C# OptionsGradeDialog GradeStrings：全部/普通/稀有/神话/传说/英雄）
const CREATURE_GRADE_LABELS: [&str; 6] = ["全部", "普通", "稀有", "神话", "传说", "英雄"];

/// 品质循环（dir>0 下一档，否则上一档；0..5，对齐 C# Prev/Next）
fn creature_grade_cycle(grade: u8, dir: i8) -> u8 {
    if dir > 0 { (grade + 1) % 6 } else { (grade + 5) % 6 }
}

/// 过滤切换（对齐 C# IntelligentCreatureItemFilter.SetItemFilter）
fn creature_filter_toggle(f: &mut [bool; 9], idx: usize) {
    match idx {
        0 => {
            f[0] = true;
            for i in 1..9 { f[i] = false; }
        }
        1..=8 => {
            f[0] = false;
            f[idx] = !f[idx];
        }
        _ => {}
    }
    if (1..9).all(|i| f[i]) {
        f[0] = true;
        for i in 1..9 { f[i] = false; }
    } else if (1..9).all(|i| !f[i]) {
        f[0] = true;
    }
}

/// 选项面板（C# IntelligentCreatureOptionsDialog：9 个过滤项 + 保存/取消）
#[allow(clippy::too_many_arguments)]
fn creature_options_system(
    mut state: ResMut<CreatureState>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    opts_btn: Query<(Entity, &Interaction), With<CreatureOptionsBtn>>,
    save_btn: Query<(Entity, &Interaction), With<CreatureOptionsSave>>,
    cancel_btn: Query<(Entity, &Interaction), With<CreatureOptionsCancel>>,
    grade_btns: Query<(Entity, &Interaction, Has<CreatureGradePrev>, Has<CreatureGradeNext>)>,
    panel_origin: Query<&Node, With<CreatureWidget>>,
    mut lines: Query<
        (&mut Text, Option<&CreatureOptionsLine>, Has<CreatureGradeText>),
        Or<(With<CreatureOptionsLine>, With<CreatureGradeText>)>,
    >,
    // #1299：Bevy B0001——四个 &mut Visibility Query 需互相 Without（#1298 合并后启动 panic）
    mut widgets: Query<
        &mut Visibility,
        (
            With<CreatureOptionsWidget>,
            Without<CreatureOptionsLine>,
            Without<CreatureOptionsSave>,
            Without<CreatureOptionsCancel>,
        ),
    >,
    mut line_vis: Query<
        &mut Visibility,
        (
            With<CreatureOptionsLine>,
            Without<CreatureOptionsWidget>,
            Without<CreatureOptionsSave>,
            Without<CreatureOptionsCancel>,
        ),
    >,
    mut save_vis: Query<
        &mut Visibility,
        (
            With<CreatureOptionsSave>,
            Without<CreatureOptionsWidget>,
            Without<CreatureOptionsLine>,
            Without<CreatureOptionsCancel>,
        ),
    >,
    mut cancel_vis: Query<
        &mut Visibility,
        (
            With<CreatureOptionsCancel>,
            Without<CreatureOptionsWidget>,
            Without<CreatureOptionsLine>,
            Without<CreatureOptionsSave>,
        ),
    >,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }

    for (e, inter) in &opts_btn {
        if edge(e, inter, &mut prev_inter) {
            if !state.options_open {
                state.options_open = true;
                if let Some(c) = state.creatures.get(state.selected) {
                    let f = c.filter;
                    let g = c.grade;
                    for i in 0..9 { state.options[i] = f[i] != 0; }
                    state.grade = g;
                }
            } else {
                state.options_open = false;
            }
        }
    }
    if state.options_open && mouse.just_pressed(MouseButton::Left) {
        if let Ok(window) = windows.single() {
            if let Some(cursor) = window.cursor_position() {
                let (ox, oy) = panel_origin
                    .single()
                    .map(|n| crate::ui::theme::node_origin(n, (280.0, 80.0)))
                    .unwrap_or((280.0, 80.0));
                for i in 0..9usize {
                    let y = oy + 40.0 + i as f32 * 22.0;
                    if cursor.x >= ox + 20.0 && cursor.x <= ox + 220.0 && cursor.y >= y && cursor.y <= y + 20.0 {
                        creature_filter_toggle(&mut state.options, i);
                        break;
                    }
                }
            }
        }
    }
    for (e, inter) in &save_btn {
        if edge(e, inter, &mut prev_inter) && state.options_open {
            let mut filter = [0u8; 9];
            for i in 0..9 { filter[i] = if state.options[i] { 1 } else { 0 }; }
            let selected = state.creatures.get(state.selected).cloned();
            let creature_type = selected.as_ref().map(|c| c.creature_type).unwrap_or(0);
            let pet_mode = selected.as_ref().map(|c| c.pickup_mode).unwrap_or(0);
            net.send_packet(&mir2_shared::packets::client::misc::UpdateIntelligentCreature {
                creature_type,
                pet_mode,
                custom_name: String::new(),
                summon_me: false,
                unsummon_me: false,
                release_me: false,
                filter,
                grade: state.grade,
                options_save: true,
            });
            state.message = "宠物拾取设置已保存".to_string();
            state.options_open = false;
        }
    }
    for (e, inter) in &cancel_btn {
        if edge(e, inter, &mut prev_inter) {
            state.options_open = false;
        }
    }
    // 品质切换（C# OptionsGradeDialog Prev/Next 循环）
    for (e, inter, is_prev, is_next) in &grade_btns {
        if edge(e, inter, &mut prev_inter) && state.options_open {
            if is_prev {
                state.grade = creature_grade_cycle(state.grade, -1);
            } else if is_next {
                state.grade = creature_grade_cycle(state.grade, 1);
            }
        }
    }
    for (mut text, line, is_grade) in &mut lines {
        text.0 = if !state.options_open {
            String::new()
        } else if let Some(l) = line {
            format!("{} {}", if state.options[l.0] { "■" } else { "□" }, CREATURE_OPTION_LABELS[l.0])
        } else if is_grade {
            format!("品质:{}", CREATURE_GRADE_LABELS[(state.grade as usize).min(5)])
        } else {
            String::new()
        };
    }
    for mut vis in widgets.iter_mut() {
        *vis = if state.options_open { Visibility::Visible } else { Visibility::Hidden };
    }
    for mut vis in line_vis.iter_mut() {
        *vis = if state.options_open { Visibility::Visible } else { Visibility::Hidden };
    }
    for mut vis in save_vis.iter_mut() {
        *vis = if state.options_open { Visibility::Visible } else { Visibility::Hidden };
    }
    for mut vis in cancel_vis.iter_mut() {
        *vis = if state.options_open { Visibility::Visible } else { Visibility::Hidden };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_toggle_all() {
        let mut f = [false; 9];
        creature_filter_toggle(&mut f, 0);
        assert!(f[0]);
        for i in 1..9 { assert!(!f[i]); }
    }

    #[test]
    fn filter_toggle_categories_and_auto_all() {
        let mut f = [false; 9];
        creature_filter_toggle(&mut f, 1);
        assert!(!f[0] && f[1]);
        for i in 2..9 { creature_filter_toggle(&mut f, i); }
        // 8 类全开 → 自动回退为「全部」
        assert!(f[0]);
        for i in 1..9 { assert!(!f[i]); }
    }

    #[test]
    fn filter_toggle_off_last_restores_all() {
        let mut f = [false; 9];
        creature_filter_toggle(&mut f, 1);
        creature_filter_toggle(&mut f, 1);
        assert!(f[0]);
    }

    #[test]
    fn grade_cycle_next() {
        assert_eq!(creature_grade_cycle(0, 1), 1);
        assert_eq!(creature_grade_cycle(5, 1), 0);
    }

    #[test]
    fn grade_cycle_prev() {
        assert_eq!(creature_grade_cycle(0, -1), 5);
        assert_eq!(creature_grade_cycle(3, -1), 2);
    }
}
