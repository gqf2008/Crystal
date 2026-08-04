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
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
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
        app.add_systems(OnEnter(AppState::Game), spawn_guild_territory);
        app.add_systems(OnExit(AppState::Game), cleanup_guild_territory);
        app.add_systems(
            Update,
            (guild_territory_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
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
    mut cache: ResMut<UiImageCache>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    // 背景 Prguse[680]（C# GuildTerritoryDialog.Index=680）
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 680) {
        let e = spawn_ui_sprite(&mut commands, h, 280.0, 80.0, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::GuildTerritory),
            GuildTerritoryWidget,
            Visibility::Hidden,
        ));
    }
    // 标题 Title[54]
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 54) {
        let e = spawn_ui_sprite(&mut commands, h, 298.0, 88.0, 6.2, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::GuildTerritory),
            GuildTerritoryWidget,
            Visibility::Hidden,
        ));
    }
    // 关闭（C# Prguse 361/362/363）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse, 361, 362, 363,
        280.0 + 480.0, 88.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            GuildTerritoryClose,
            DialogRoot(DialogKind::GuildTerritory),
            GuildTerritoryWidget,
        ));
    }
    // 领地列表 7 行（C# 每页 7 行）
    for i in 0..7usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            298.0, 125.0 + i as f32 * 22.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            GuildTerritoryLine(i),
            DialogRoot(DialogKind::GuildTerritory),
            GuildTerritoryWidget,
        ));
    }
    // 状态/页签/宣战结果行
    for i in 7..=9usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            298.0, 285.0 + (i - 7) as f32 * 18.0,
            12.0, Color::srgb(1.0, 0.9, 0.5), 8.0,
        );
        commands.entity(e).insert((
            GuildTerritoryLine(i),
            DialogRoot(DialogKind::GuildTerritory),
            GuildTerritoryWidget,
        ));
    }
    // 上一页/下一页（C# Prguse2 240/241/242, 243/244/245）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 240, 241, 242,
        300.0, 350.0, 8.3, 20.0, 16.0,
    ) {
        commands.entity(e).insert((
            GuildTerritoryPrev,
            DialogRoot(DialogKind::GuildTerritory),
            GuildTerritoryWidget,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 243, 244, 245,
        340.0, 350.0, 8.3, 20.0, 16.0,
    ) {
        commands.entity(e).insert((
            GuildTerritoryNext,
            DialogRoot(DialogKind::GuildTerritory),
            GuildTerritoryWidget,
        ));
    }
    // 购买按钮（C# Prguse 437/438/439）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse, 437, 438, 439,
        390.0, 345.0, 8.3, 60.0, 25.0,
    ) {
        commands.entity(e).insert((
            GuildTerritoryBuy,
            DialogRoot(DialogKind::GuildTerritory),
            GuildTerritoryWidget,
        ));
    }
    // 宣战：目标行会输入框（TextInput id 7）+ 宣战按钮
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    let war_box = commands
        .spawn((
            crate::ui::sprite_ui::UiEntity,
            DialogRoot(DialogKind::GuildTerritory),
            GuildTerritoryWidget,
            GuildTerritoryWarField,
            crate::game::dialogs::text_input::TextInputField(7),
            crate::game::dialogs::text_input::TextInputRect(298.0, 390.0, 180.0, 20.0),
            Sprite {
                image: white.clone(),
                color: Color::srgba(0.2, 0.2, 0.25, 0.9),
                custom_size: Some(Vec2::new(180.0, 20.0)),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(298.0, -390.0, 8.1),
            Visibility::Hidden,
        ))
        .id();
    commands.entity(war_box).with_children(|p| {
        p.spawn((
            crate::game::dialogs::text_input::TextInputDisplay(7),
            Text2d::new(String::new()),
            bevy::sprite::Anchor::TOP_LEFT,
            TextFont {
                font: FontSource::Handle(font.clone()),
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::srgb(1.0, 1.0, 1.0)),
            Transform::from_xyz(4.0, -2.0, 8.2),
        ));
    });
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse, 437, 438, 439,
        490.0, 388.0, 8.3, 60.0, 25.0,
    ) {
        commands.entity(e).insert((
            GuildTerritoryWar,
            DialogRoot(DialogKind::GuildTerritory),
            GuildTerritoryWidget,
        ));
    }
}

/// 显隐 + 渲染 + 请求/翻页/购买/宣战
#[allow(clippy::too_many_arguments)]
fn guild_territory_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<GuildTerritoryState>,
    net: Res<NetConnection>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    close: Query<&UiButton, With<GuildTerritoryClose>>,
    buy_btn: Query<&UiButton, With<GuildTerritoryBuy>>,
    war_btn: Query<&UiButton, With<GuildTerritoryWar>>,
    prev_btn: Query<&UiButton, With<GuildTerritoryPrev>>,
    next_btn: Query<&UiButton, With<GuildTerritoryNext>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut widgets: Query<&mut Visibility, With<GuildTerritoryWidget>>,
    mut lines: Query<(&mut Text2d, &GuildTerritoryLine)>,
    mut requested: Local<bool>,
) {
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
    for btn in &close {
        if btn.clicked {
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
                for i in 0..7usize {
                    let y = 125.0 + i as f32 * 22.0;
                    if cursor.x >= 298.0 && cursor.x <= 640.0 && cursor.y >= y && cursor.y <= y + 20.0 {
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
    for btn in &prev_btn {
        if btn.clicked && state.page > 0 {
            state.page -= 1;
            net.send_packet(&crate::network::GuildTerritoryPageWire {
                page: state.page as u32,
            });
        }
    }
    for btn in &next_btn {
        if btn.clicked && (state.page + 1) * 7 < state.rows.len() {
            state.page += 1;
            net.send_packet(&crate::network::GuildTerritoryPageWire {
                page: state.page as u32,
            });
        }
    }
    // 购买选中的无主领地（C# BuyButton → C.PurchaseGuildTerritory）
    for btn in &buy_btn {
        if btn.clicked {
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
    for btn in &war_btn {
        if btn.clicked {
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
