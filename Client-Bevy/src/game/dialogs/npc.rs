// ============================================================================
// NPC 对话框（M9 第 2 批）
// 布局参考：macroquad npc_dialog.rs / C# NPCDialogs.cs
//   - 背景 Prguse[384/385]，位置 (0,0)
//   - 文本区 (8,34)，行距 18；[@XXX] 行是选项，点击发送 CallNPC
//   - 关闭按钮 Prguse2[360-362] 在 (413,3)
// 网络：NPCResponse（行列表）→ 显示；CallNPC 推进
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use bevy::sprite::Anchor;

use crate::game::dialogs::text_input::{
    TextInputDisplay, TextInputField, TextInputRect, TextInputState, TextInputSubmit,
};
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};
use crate::ui::scroll_list::{spawn_scroll_bar, ScrollList};

/// NPC 对话框状态（网络写入）
#[derive(Resource, Default)]
pub struct NpcDialogState {
    pub visible: bool,
    pub npc_object_id: u32,
    pub lines: Vec<String>,
}

#[derive(Component)]
pub struct NpcDialogWidget;

#[derive(Component)]
pub struct NpcClose;

#[derive(Component)]
pub struct NpcLine(usize);

#[derive(Component)]
pub struct NpcQuest;

/// #272 NPC 输入状态（S.NPCRequestInput）
#[derive(Resource, Default)]
pub struct NpcInputState {
    pub npc_id: u32,
    pub page_name: String,
    pub active: bool,
}

/// #272 NPC 输入覆盖层根
#[derive(Component)]
pub struct NpcInputRoot;

/// #272 NPC 输入确定按钮
#[derive(Component)]
pub struct NpcInputOk;

pub struct NpcDialogPlugin;

impl Plugin for NpcDialogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NpcDialogState>();
        app.add_systems(OnEnter(AppState::Game), spawn_npc_dialog);
        app.add_systems(OnExit(AppState::Game), cleanup_npc_dialog);
        app.add_systems(
            Update,
            npc_dialog_server_events.run_if(in_state(AppState::Game)),
        );
        app.init_resource::<NpcInputState>();
        app.add_systems(
            Update,
            npc_input_overlay
                .after(crate::network::network_system)
                .run_if(in_state(AppState::Game)),
        );
        app.add_systems(
            Update,
            (npc_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_npc_dialog(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_npc_dialog(
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

    // 背景 Prguse[384]
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 384) {
        let e = spawn_ui_sprite(&mut commands, h, 0.0, 0.0, 6.0, 1.0);
        // #118 长对话页滚轮滚动（C# NPC 对话框支持 MouseWheel）
        let (track, thumb) = spawn_scroll_bar(&mut commands, &mut images, (420.0, 34.0, 4.0, 144.0), 6.3);
        commands.entity(track).insert((
            DialogRoot(crate::game::dialogs::DialogKind::Npc),
            NpcDialogWidget,
            Visibility::Visible,
        ));
        commands.entity(thumb).insert((
            DialogRoot(crate::game::dialogs::DialogKind::Npc),
            NpcDialogWidget,
            Visibility::Visible,
        ));
        commands.entity(e).insert((
            DialogRoot(crate::game::dialogs::DialogKind::Npc),
            NpcDialogWidget,
            Visibility::Hidden,
            ScrollList {
                rect_rel: (8.0, 34.0, 400.0, 144.0),
                row_h: 18.0,
                visible: 8,
                total: 0,
                offset: 0,
                step: 3,
                track_rel: (420.0, 34.0, 4.0, 144.0),
                thumb: Some(thumb),
                z: 8.0,
            },
        ));
    }

    // 关闭按钮
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        413.0, 3.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            NpcClose,
            DialogRoot(crate::game::dialogs::DialogKind::Npc),
            NpcDialogWidget,
        ));
    }

    // 任务按钮（#90 续：MirAnimatedButton，C# NPCDialog QuestButton
    // Title[530..539] 10 帧 130ms 循环 + 悬停 284 / 按下 286，点击切换任务日志）
    {
        let bg_h = libs
            .0
            .get_image(LibraryName::Prguse, 384)
            .map(|i| i.height.max(0) as f32)
            .unwrap_or(210.0);
        if let Some(e) = crate::ui::controls::spawn_animated_button(
            &mut commands,
            &mut libs,
            &mut images,
            &mut cache,
            LibraryName::Title,
            530,
            10,
            Some(284),
            Some(286),
            172.0,
            bg_h - 30.0,
            8.5,
            96.0,
            25.0,
            0.13,
            true,
        ) {
            commands.entity(e).insert((
                NpcQuest,
                DialogRoot(crate::game::dialogs::DialogKind::Npc),
                Visibility::Hidden,
            ));
        }
    }

    // 8 行文本
    for i in 0..8usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            8.0, 34.0 + i as f32 * 18.0,
            13.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            NpcLine(i),
            DialogRoot(crate::game::dialogs::DialogKind::Npc),
            NpcDialogWidget,
        ));
    }
}

/// 显示/关闭 + 文本渲染 + 选项点击
fn npc_ui_system(
    mut npc: ResMut<NpcDialogState>,
    mut npc_goods: ResMut<crate::game::dialogs::npc_goods::NpcGoodsState>,
    mut sell_panel: ResMut<crate::game::dialogs::sell_panel::SellPanelState>,
    mut storage: ResMut<crate::game::dialogs::storage::StorageState>,
    mut mgr: ResMut<crate::game::dialogs::DialogManager>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    close: Query<&UiButton, With<NpcClose>>,
    mut widgets: Query<&mut Visibility, With<NpcDialogWidget>>,
    mut quest_btns: Query<(&UiButton, &mut Visibility), (With<NpcQuest>, Without<NpcDialogWidget>)>,
    mut lines: Query<(&mut Text2d, &mut TextColor, &NpcLine)>,
    mut scroll: Query<&mut ScrollList, With<NpcDialogWidget>>,
) {
    for mut vis in widgets.iter_mut() {
        *vis = if npc.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // 任务按钮（C# CheckQuestButtonDisplay：NPC 有可用任务才显示）
    let has_quest = npc
        .lines
        .iter()
        .any(|l| l.contains("可接受任务") || l.contains("可完成任务"));
    for (btn, mut vis) in &mut quest_btns {
        *vis = if npc.visible && has_quest {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if btn.clicked && npc.visible && has_quest {
            mgr.toggle(DialogKind::QuestLog);
        }
    }
    if !npc.visible {
        // C# 语义：NPC 对话框关闭时联动隐藏商店/出售/仓库面板
        if npc_goods.visible {
            npc_goods.visible = false;
        }
        if sell_panel.visible {
            sell_panel.visible = false;
        }
        if storage.visible {
            storage.visible = false;
            mgr.close(crate::game::dialogs::DialogKind::Storage);
        }
        return;
    }

    // 关闭
    for btn in &close {
        if btn.clicked {
            npc.visible = false;
        }
    }

    // 滚动偏移 + 总行数（#118）
    {
        let mut sl = scroll.single_mut();
        if let Ok(sl) = sl.as_mut() {
            sl.set_total(npc.lines.len());
        }
    }
    let off = scroll.single().map(|s| s.offset).unwrap_or(0);

    // 渲染行 + 选项悬停高亮
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    for (mut text, mut color, line) in &mut lines {
        if let Some(l) = npc.lines.get(off + line.0) {
            text.0 = l.clone();
            let y = 34.0 + line.0 as f32 * 18.0;
            let hover = cursor.x >= 8.0 && cursor.x <= 400.0 && cursor.y >= y && cursor.y <= y + 16.0;
            let c = if is_clickable_npc_line(l) {
                if hover {
                    Color::srgb(1.0, 0.95, 0.4)
                } else {
                    Color::srgb(1.0, 0.85, 0.3)
                }
            } else {
                Color::WHITE
            };
            if color.0 != c {
                color.0 = c;
            }
        } else {
            text.0 = String::new();
        }
    }

    // 点击选项行（以 [@ 开头的行，#118 含滚动偏移）
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    for (i, l) in npc.lines.iter().enumerate() {
        if i >= off && i < off + 8 && is_clickable_npc_line(l) {
            let row = i - off;
            let y = 34.0 + row as f32 * 18.0;
            if cursor.x >= 8.0 && cursor.x <= 400.0 && cursor.y >= y && cursor.y <= y + 16.0 {
                let key = extract_npc_key(l);
                // 菜单类型标记（购买按钮据此区分 BuyItem / BuyItemBack）
                npc_goods.is_buyback = key.eq_ignore_ascii_case("[@BuyBack]");
                net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                    object_id: npc.npc_object_id,
                    key: key.clone(),
                });
                tracing::info!("🧙 NPC 选项: {} → {}", l.trim(), key);
                break;
            }
        }
    }
}

/// 可点击的 NPC 菜单行：[@XXX] 或 <文字/@XXX>（原版 C# 链接格式）
fn is_clickable_npc_line(line: &str) -> bool {
    let t = line.trim();
    t.starts_with("[@") || t.contains("/@")
}

/// 提取菜单键（统一为 "[@XXX]" 格式，服务端按该格式匹配）
pub fn extract_npc_key(line: &str) -> String {
    let t = line.trim();
    if t.starts_with("[@") {
        let end = t.find(']').unwrap_or(t.len());
        t[..end].to_string()
    } else if let Some(slash) = t.find("/@") {
        let rest = &t[slash + 1..];
        let end = rest.find('>').unwrap_or(rest.len());
        format!("[@{}]", &rest[..end])
    } else {
        t.to_string()
    }
}


/// 消费服务端 NPC 对话事件（网络层只广播 ServerEvent）
fn npc_dialog_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut npc: ResMut<NpcDialogState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::NpcDialog { lines, visible } = ev {
            npc.lines = lines.clone();
            npc.visible = *visible;
        }
    }
}


/// #272：NPC 输入覆盖层——S.NPCRequestInput → 弹输入框；确定/Enter → C.NPCConfirmInput
#[allow(clippy::too_many_arguments)]
fn npc_input_overlay(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
    net: Res<NetConnection>,
    mut state: ResMut<NpcInputState>,
    mut text_state: ResMut<TextInputState>,
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut submits: MessageReader<TextInputSubmit>,
    ok_btns: Query<&UiButton, With<NpcInputOk>>,
    mut roots: Query<&mut Visibility, With<NpcInputRoot>>,
) {
    use crate::network::server_event::ServerEvent;

    for ev in events.read() {
        if let ServerEvent::NpcInputRequest { npc_id, page_name } = ev {
            state.npc_id = *npc_id;
            state.page_name = page_name.clone();
            state.active = true;
            text_state.texts.resize(1, String::new());
            text_state.texts[0].clear();
            text_state.active = Some(0);
            if roots.iter_mut().count() == 0 {
                spawn_npc_input_overlay(
                    &mut commands,
                    &mut libs,
                    &mut images,
                    &mut cache,
                    &mut fonts,
                    &mut ui_font,
                    page_name,
                );
            }
            for mut vis in roots.iter_mut() {
                *vis = Visibility::Visible;
            }
            tracing::info!("⌨️ [NPC] 输入框打开 npc={} page={}", npc_id, page_name);
        }
    }

    let submitted = submits.read().any(|s| s.0 == 0);
    let ok_clicked = ok_btns.iter().any(|b| b.clicked);
    if state.active && (submitted || ok_clicked) {
        let value = text_state.texts.first().cloned().unwrap_or_default();
        net.send_packet(&mir2_shared::packets::client::npc::NPCConfirmInput {
            npc_id: state.npc_id,
            page_name: state.page_name.clone(),
            value,
        });
        tracing::info!(
            "⌨️ [NPC] 提交输入 -> npc={} page={}",
            state.npc_id,
            state.page_name
        );
        state.active = false;
        text_state.active = None;
        for mut vis in roots.iter_mut() {
            *vis = Visibility::Hidden;
        }
    }
}

/// 生成输入覆盖层（面板 + 提示 + 输入框 + 确定）
#[allow(clippy::too_many_arguments)]
fn spawn_npc_input_overlay(
    commands: &mut Commands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    cache: &mut UiImageCache,
    fonts: &mut Assets<Font>,
    ui_font: &mut UiFont,
    page_name: &str,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(fonts);
    }
    let font = ui_font.0.clone();
    let white = images.add(crate::map_renderer::make_image(
        vec![255, 255, 255, 255],
        1,
        1,
    ));

    // 面板
    let root = if let Some(h) = ui_image(libs, images, cache, LibraryName::Prguse, 170) {
        let e = spawn_ui_sprite(commands, h, 280.0, 80.0, 6.0, 1.0);
        commands
            .entity(e)
            .insert((NpcInputRoot, Visibility::Hidden));
        e
    } else {
        commands
            .spawn((
                NpcInputRoot,
                Sprite {
                    image: white.clone(),
                    color: Color::srgba(0.1, 0.1, 0.14, 0.95),
                    custom_size: Some(Vec2::new(360.0, 140.0)),
                    ..default()
                },
                Anchor::TOP_LEFT,
                Transform::from_xyz(280.0, -80.0, 6.0),
                Visibility::Hidden,
            ))
            .id()
    };
    let _ = root;

    // 提示
    let prompt = spawn_ui_text(
        commands, &font, &format!("请输入（{}）:", page_name),
        300.0, 100.0, 14.0, Color::WHITE, 8.1,
    );
    commands.entity(prompt).insert(NpcInputRoot);

    // 输入框
    let field = commands
        .spawn((
            NpcInputRoot,
            TextInputField(0),
            TextInputRect(300.0, 130.0, 280.0, 22.0),
            Sprite {
                image: white.clone(),
                color: Color::srgba(0.2, 0.2, 0.25, 0.9),
                custom_size: Some(Vec2::new(280.0, 22.0)),
                ..default()
            },
            Anchor::TOP_LEFT,
            Transform::from_xyz(300.0, -130.0, 8.1),
        ))
        .id();
    commands.entity(field).with_children(|p| {
        p.spawn((
            TextInputDisplay(0),
            Text2d::new(String::new()),
            Anchor::TOP_LEFT,
            TextFont {
                font: FontSource::Handle(font.clone()),
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(Color::WHITE),
            Transform::from_xyz(3.0, -3.0, 8.2),
        ));
    });

    // 确定
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        commands, libs, images, cache,
        LibraryName::Prguse2, 360, 361, 362,
        560.0, 175.0, 8.2, 50.0, 22.0,
    ) {
        commands.entity(e).insert(NpcInputOk);
    }
}
