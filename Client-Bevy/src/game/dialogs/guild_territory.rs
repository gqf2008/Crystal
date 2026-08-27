// ============================================================================
// 行会领地对话框（M36）
// 参考：C# GuildTerritoryDialog（Prguse[680] 背景 / Title[54] 标题）
//   - 7 行领地列表（GT# / 拥有行会 / 状态 / 价格）+ 上一页/下一页 + 购买 + 给会长发信
//   - 本实现：领地列表 + 翻页 + 购买 + 宣战（GuildWarReturn）
// 网络（ServerRust 实际 wire）：
//   C: GuildTerritoryPage[page u32] / PurchaseGuildTerritory[territory_id u32]
//      GuildWarReturn[guild_name 7-bit dotnet]
//   S: GuildTerritoryPage(276)[count i32][per: id i32][map i32][owner 7-bit][state u8]
//      GuildRequestWar(173)[guild_name 7-bit dotnet]
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_icon_button, spawn_label, spawn_panel,
};

/// 领地行（GuildTerritoryPage 写入）
#[derive(Debug, Clone, Default)]
pub struct TerritoryRow {
    pub id: i32,
    pub map_index: i32,
    pub owner: String,
    /// 0=Idle 1=Declared 2=InProgress 3=Ended
    pub state: u8,
}

/// 行会领地状态
#[derive(Resource, Default)]
pub struct GuildTerritoryState {
    pub rows: Vec<TerritoryRow>,
    pub page: usize,
    pub selected: Option<usize>,
    pub message: String,
    /// 最近宣战结果（GuildRequestWar 写入）
    pub war_message: String,
}

#[derive(Component)]
pub struct GuildTerritoryWidget;

#[derive(Component)]
pub struct GuildTerritoryClose;

#[derive(Component)]
pub struct GuildTerritoryBuy;

#[derive(Component)]
pub struct GuildTerritoryWar;

#[derive(Component)]
pub struct GuildTerritoryPrev;

#[derive(Component)]
pub struct GuildTerritoryNext;

#[derive(Component)]
pub struct GuildTerritoryLine(usize);

/// 宣战目标行会输入框（TextInput id 7）
#[derive(Component)]
pub struct GuildTerritoryWarField;

pub struct GuildTerritoryPlugin;

impl Plugin for GuildTerritoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GuildTerritoryState>();
                app.add_systems(
            Update,
            territory_server_events.run_if(in_state(AppState::Game)),
        );
app.add_systems(OnEnter(AppState::Game), spawn_guild_territory);
        app.add_systems(OnExit(AppState::Game), cleanup_guild_territory);
        app.add_systems(
            Update,
            guild_territory_ui_system.run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_guild_territory(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_guild_territory(
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

    // 面板 Prguse[680]（C# GuildTerritoryDialog Index=680，568x241 @ (280,80)）。
    // 加高到 340：旧 sprite 布局宣战输入/按钮在 rel y=308-333 悬空 241 高面板外
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 680) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, 280.0, 80.0, 568.0, 340.0, 30);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::GuildTerritory), GuildTerritoryWidget));

    commands.entity(panel).with_children(|p| {
        // 标题 Title[54] @(18,8)
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 54) {
            crate::ui::theme::spawn_image(p, h, 18.0, 8.0, 133.0, 15.0, 8);
        }
        // 关闭（C# Prguse 361/362/363 @(544,8)；旧 sprite rel(480,8) 保留）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 362),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 363),
        ) {
            spawn_icon_button(p, n, h, pr, 480.0, 8.0, 20.0, 20.0, 10)
                .insert(GuildTerritoryClose);
        }
        // 领地列表 7 行 @(18,45+22i)
        for i in 0..7usize {
            spawn_label(p, &cjk, "", 18.0, 45.0 + i as f32 * 22.0, 12.0, Color::WHITE, 9)
                .insert(GuildTerritoryLine(i));
        }
        // 状态/页签/宣战结果行 @(18,205+18i)
        for i in 7..=9usize {
            spawn_label(p, &cjk, "", 18.0, 205.0 + (i - 7) as f32 * 18.0, 12.0, Color::srgb(1.0, 0.9, 0.5), 9)
                .insert(GuildTerritoryLine(i));
        }
        // 上一页/下一页（C# Prguse2 240/241/242, 243/244/245）@(20/60,270)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 240),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 241),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 242),
        ) {
            spawn_icon_button(p, n, h, pr, 20.0, 270.0, 20.0, 16.0, 10)
                .insert(GuildTerritoryPrev);
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 243),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 244),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 245),
        ) {
            spawn_icon_button(p, n, h, pr, 60.0, 270.0, 20.0, 16.0, 10)
                .insert(GuildTerritoryNext);
        }
        // 购买按钮（C# Prguse 437/438/439）@(110,265)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 437),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 438),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 439),
        ) {
            spawn_icon_button(p, n.clone(), h.clone(), pr.clone(), 110.0, 265.0, 60.0, 25.0, 10)
                .insert(GuildTerritoryBuy);
            // 宣战按钮（同图）@(210,308)
            spawn_icon_button(p, n, h, pr, 210.0, 308.0, 60.0, 25.0, 10)
                .insert(GuildTerritoryWar);
        }
        // 宣战目标行会输入框（TextInput 7）@(18,310)，命中矩形 (298,390,180,20)
        spawn_container(p, 18.0, 310.0, 180.0, 20.0, 10)
            .insert((
                BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                crate::game::dialogs::text_input::TextInputField(7),
                crate::game::dialogs::text_input::TextInputRect(298.0, 390.0, 180.0, 20.0),
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
                    crate::game::dialogs::text_input::TextInputDisplay(7),
                ));
            });
    });
}

/// 显隐 + 渲染 + 请求/翻页/购买/宣战
#[allow(clippy::too_many_arguments)]
fn guild_territory_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<GuildTerritoryState>,
    net: Res<NetConnection>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    close: Query<(Entity, &Interaction), With<GuildTerritoryClose>>,
    buy_btn: Query<(Entity, &Interaction), With<GuildTerritoryBuy>>,
    war_btn: Query<(Entity, &Interaction), With<GuildTerritoryWar>>,
    prev_btn: Query<(Entity, &Interaction), With<GuildTerritoryPrev>>,
    next_btn: Query<(Entity, &Interaction), With<GuildTerritoryNext>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut widgets: Query<&mut Visibility, With<GuildTerritoryWidget>>,
    mut lines: Query<(&mut Text, &GuildTerritoryLine)>,
    mut requested: Local<bool>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
    panel_origin: Query<&Node, With<GuildTerritoryWidget>>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    let open = mgr.is_open(DialogKind::GuildTerritory);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        *requested = false;
        return;
    }
    // 打开瞬间请求第 0 页（C# GuildTerritoryDialog.Show → C.GuildTerritoryPage{Page=0}）
    if !*requested {
        *requested = true;
        state.page = 0;
        net.send_packet(&crate::network::GuildTerritoryPageWire { page: 0 });
        tracing::info!("🏯 请求行会领地列表");
    }
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::GuildTerritory);
        }
    }
    // 渲染
    for (mut text, line) in &mut lines {
        text.0 = match line.0 {
            i if i < 7 => match state.rows.get(state.page * 7 + i) {
                Some(r) => {
                    let status = match r.state {
                        1 => "已宣战",
                        2 => "战斗中",
                        3 => "已结束",
                        _ => {
                            if r.owner.is_empty() {
                                "无主"
                            } else {
                                "已占领"
                            }
                        }
                    };
                    format!(
                        "GT#{} {}（{}）状态:{}",
                        r.id,
                        if r.owner.is_empty() { "无主" } else { &r.owner },
                        r.map_index,
                        status
                    )
                }
                None => String::new(),
            },
            7 => format!("第 {}/{} 页", state.page + 1, ((state.rows.len() + 6) / 7).max(1)),
            8 => state.message.clone(),
            9 => state.war_message.clone(),
            _ => String::new(),
        };
    }
    // 行点击选中（购买目标）
    if mouse.just_pressed(MouseButton::Left) {
        if let Ok(window) = windows.single() {
            if let Some(cursor) = window.cursor_position() {
                let (ox, oy) = panel_origin
                    .single()
                    .map(|n| crate::ui::theme::node_origin(n, (280.0, 80.0)))
                    .unwrap_or((280.0, 80.0));
                for i in 0..7usize {
                    let y = oy + 45.0 + i as f32 * 22.0;
                    if cursor.x >= ox + 18.0 && cursor.x <= ox + 360.0 && cursor.y >= y && cursor.y <= y + 20.0 {
                        let idx = state.page * 7 + i;
                        if idx < state.rows.len() {
                            state.selected = Some(idx);
                            tracing::info!("🏯 选中领地 #{}", state.rows[idx].id);
                        }
                        break;
                    }
                }
            }
        }
    }
    // 翻页
    for (e, inter) in &prev_btn {
        if edge(e, inter, &mut prev_inter) && state.page > 0 {
            state.page -= 1;
            net.send_packet(&crate::network::GuildTerritoryPageWire {
                page: state.page as u32,
            });
        }
    }
    for (e, inter) in &next_btn {
        if edge(e, inter, &mut prev_inter) && (state.page + 1) * 7 < state.rows.len() {
            state.page += 1;
            net.send_packet(&crate::network::GuildTerritoryPageWire {
                page: state.page as u32,
            });
        }
    }
    // 购买选中的无主领地（C# BuyButton → C.PurchaseGuildTerritory）
    for (e, inter) in &buy_btn {
        if edge(e, inter, &mut prev_inter) {
            if let Some(idx) = state.selected {
                let r = &state.rows[idx];
                if r.owner.is_empty() {
                    net.send_packet(&crate::network::PurchaseGuildTerritoryWire {
                        territory_id: r.id as u32,
                    });
                    tracing::info!("🏯 购买领地 #{}", r.id);
                    // 购买后稍后刷新（服务端无广播）
                    net.send_packet(&crate::network::GuildTerritoryPageWire { page: 0 });
                } else {
                    state.message = "该领地已被占领".to_string();
                }
            } else {
                state.message = "请先点击选中一个领地".to_string();
            }
        }
    }
    // 宣战（输入目标行会名 → C.GuildWarReturn）
    for (e, inter) in &war_btn {
        if edge(e, inter, &mut prev_inter) {
            let name = input.texts.get(7).cloned().unwrap_or_default();
            let name = name.trim().to_string();
            if !name.is_empty() {
                net.send_packet(&mir2_shared::packets::client::guild::GuildWarReturn {
                    guild_name: name.clone(),
                });
                tracing::info!("🏯 向行会 {} 宣战", name);
                input.texts[7].clear();
                input.active = None;
            }
        }
    }
}


/// 消费服务端领地事件（网络层只广播 ServerEvent；文案在此构造）
fn territory_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut territory: ResMut<GuildTerritoryState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        match ev {
            ServerEvent::TerritoryList { rows } => {
                territory.rows = rows.clone();
            }
            ServerEvent::TerritoryWar { guild_name } => {
                territory.war_message = format!("已向 {} 行会宣战", guild_name);
            }
            _ => {}
        }
    }
}
