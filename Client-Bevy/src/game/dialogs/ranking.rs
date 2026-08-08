// ============================================================================
// 排名对话框（M9 第 3 批）
// 布局参考：macroquad ranking_dialog.rs
//   - 320x380 面板，(200,150)，10 行 28px
// 网络：Ranking 请求 → 服务器回排名 → 显示
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::controls::{spawn_checkbox, CheckBox};
use crate::ui::scroll_list::{spawn_scroll_bar, ScrollList};
use crate::ui::sprite_ui::{spawn_ui_text, UiButton, UiEntity, UiFont, UiImageCache};

/// 排名条目（服务端 Rankings 包）
#[derive(Debug, Clone, Default)]
pub struct RankEntry {
    pub rank: i32,
    pub player_name: String,
    pub class: u8,
    pub level: i32,
    pub experience: i64,
}

#[derive(Resource, Default)]
pub struct RankingState {
    pub visible: bool,
    pub entries: Vec<RankEntry>,
    /// 当前页签（0=All 1..5=职业，C# RankingDialog SelectRank）
    pub tab: u8,
    /// 仅在线（C# RankingDialog OnlineOnly）
    pub online_only: bool,
}

#[derive(Component)]
pub struct RankingWidget;

#[derive(Component)]
pub struct RankingClose;

#[derive(Component)]
pub struct RankingLine(usize);

/// 页签按钮（C# AllButton/WarButton/WizButton/TaoButton/SinButton/ArchButton）
#[derive(Component)]
pub struct RankingTab(pub u8);

/// 上一页（C# PrevButton）
#[derive(Component)]
pub struct RankingPrev;

/// 下一页（C# NextButton）
#[derive(Component)]
pub struct RankingNext;

/// 仅在线（C# OnlineOnlyButton）
#[derive(Component)]
pub struct RankingOnlineOnly;

/// 职业名（C# 排行榜职业页签）
pub fn rank_class_name(class: u8) -> &'static str {
    match class {
        0 => "战士",
        1 => "法师",
        2 => "道士",
        3 => "刺客",
        4 => "弓箭手",
        _ => "未知",
    }
}

/// 按页签过滤（0=全部，1..5=对应职业；服务端暂返回全职业，本地过滤对齐 C# 页签语义）
pub fn filter_rank_tab(entries: &[RankEntry], tab: u8) -> Vec<RankEntry> {
    if tab == 0 {
        return entries.to_vec();
    }
    entries
        .iter()
        .filter(|e| e.class + 1 == tab)
        .cloned()
        .collect()
}

pub struct RankingPlugin;

impl Plugin for RankingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RankingState>();
                app.add_systems(
            Update,
            ranking_server_events.run_if(in_state(AppState::Game)),
        );
app.add_systems(OnEnter(AppState::Game), spawn_ranking);
        app.add_systems(OnExit(AppState::Game), cleanup_ranking);
        app.add_systems(
            Update,
            (ranking_ui_system,).run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_ranking(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_ranking(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
    mut libs: ResMut<GameLibraries>,
    mut cache: ResMut<UiImageCache>,
    ranking: Res<RankingState>,
) {
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    // 面板
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    let panel = commands
        .spawn((
            UiEntity,
            DialogRoot(DialogKind::Ranking),
            RankingWidget,
            Sprite {
                image: white.clone(),
                color: Color::srgba(0.12, 0.12, 0.16, 0.95),
                custom_size: Some(Vec2::new(320.0, 380.0)),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(200.0, -150.0, 8.0),
            Visibility::Hidden,
        ))
        .id();
    // #89 可滚动排行列表：10 行 × 28px
    let (track, thumb) = spawn_scroll_bar(&mut commands, &mut images, (490.0, 190.0, 4.0, 280.0), 8.3);
    commands.entity(track).insert((DialogRoot(DialogKind::Ranking), RankingWidget, Visibility::Visible));
    commands.entity(thumb).insert((
        DialogRoot(DialogKind::Ranking),
        RankingWidget,
        Visibility::Visible,
    ));
    commands.entity(panel).insert(ScrollList {
        rect_rel: (10.0, 40.0, 280.0, 280.0),
        row_h: 28.0,
        visible: 10,
        total: 0,
        offset: 0,
        step: 3,
        track_rel: (290.0, 40.0, 4.0, 280.0),
        thumb: Some(thumb),
        z: 9.0,
    });

    // 标题
    let t = spawn_ui_text(&mut commands, &font, "排行榜", 330.0, 158.0, 16.0, Color::srgb(1.0, 1.0, 0.3), 8.2);
    commands.entity(t).insert((DialogRoot(DialogKind::Ranking), RankingWidget));

    // 页签（C# RankingDialog：All/War/Wiz/Tao/Sin/Arch）
    let tabs: [(u8, &str); 6] = [
        (0, "全部"),
        (1, "战士"),
        (2, "法师"),
        (3, "道士"),
        (4, "刺客"),
        (5, "弓手"),
    ];
    for (i, (t, label)) in tabs.iter().enumerate() {
        let e = spawn_ui_text(
            &mut commands, &font, label,
            210.0 + i as f32 * 46.0, 168.0,
            12.0, Color::srgb(0.8, 0.9, 1.0), 8.2,
        );
        commands.entity(e).insert((
            RankingTab(*t),
            UiButton {
                rect: (210.0 + i as f32 * 46.0, 168.0, 44.0, 18.0),
                clicked: false,
            },
            DialogRoot(DialogKind::Ranking),
            RankingWidget,
        ));
    }
    // 翻页（C# PrevButton/NextButton；两种组件类型不同，分别生成）
    let prev = spawn_ui_text(
        &mut commands, &font, "上一页",
        210.0, 480.0,
        12.0, Color::srgb(0.8, 0.9, 1.0), 8.2,
    );
    commands.entity(prev).insert((
        RankingPrev,
        UiButton {
            rect: (210.0, 480.0, 66.0, 20.0),
            clicked: false,
        },
        DialogRoot(DialogKind::Ranking),
        RankingWidget,
    ));
    let next = spawn_ui_text(
        &mut commands, &font, "下一页",
        280.0, 480.0,
        12.0, Color::srgb(0.8, 0.9, 1.0), 8.2,
    );
    commands.entity(next).insert((
        RankingNext,
        UiButton {
            rect: (280.0, 480.0, 66.0, 20.0),
            clicked: false,
        },
        DialogRoot(DialogKind::Ranking),
        RankingWidget,
    ));

    // 仅在线（C# OnlineOnlyButton：Prguse 2086 未勾 / 2087 勾选）
    if let Some(e) = spawn_checkbox(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse,
        [2086, 2086, 2086],
        [2087, 2087, 2087],
        390.0, 502.0, 8.2, 16.0, 14.0,
        ranking.online_only,
    ) {
        commands.entity(e).insert((
            RankingOnlineOnly,
            DialogRoot(DialogKind::Ranking),
            RankingWidget,
        ));
    }
    let e = spawn_ui_text(
        &mut commands, &font, "仅在线",
        410.0, 502.0,
        12.0, Color::srgb(0.8, 0.9, 1.0), 8.2,
    );
    commands.entity(e).insert((
        DialogRoot(DialogKind::Ranking),
        RankingWidget,
    ));

    // 10 行
    for i in 0..10usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            210.0, 190.0 + i as f32 * 28.0,
            13.0, Color::WHITE, 8.2,
        );
        commands.entity(e).insert((
            RankingLine(i),
            DialogRoot(DialogKind::Ranking),
            RankingWidget,
        ));
    }
}

fn ranking_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut ranking: ResMut<RankingState>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    tabs: Query<(&UiButton, &RankingTab)>,
    prev: Query<&UiButton, (With<RankingPrev>, Without<RankingTab>)>,
    next: Query<&UiButton, (With<RankingNext>, Without<RankingTab>, Without<RankingPrev>)>,
    online_boxes: Query<&CheckBox, With<RankingOnlineOnly>>,
    mut widgets: Query<&mut Visibility, With<RankingWidget>>,
    mut lines: Query<(&mut Text2d, &RankingLine)>,
    mut scroll: Query<&mut ScrollList, With<RankingWidget>>,
    mut requested: Local<bool>,
) {
    let open = ranking.visible || mgr.is_open(DialogKind::Ranking);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        *requested = false;
        return;
    }
    // 打开瞬间请求排行榜（原版 C# RankingDialog.Show → GetRanking）
    if !*requested {
        *requested = true;
        net.send_packet(&mir2_shared::packets::client::misc::GetRanking {
            rank_index: ranking.tab,
            online_only: ranking.online_only,
        });
        tracing::info!("🏅 请求排行榜");
    }
    let filtered = filter_rank_tab(&ranking.entries, ranking.tab);
    let max_offset = filtered.len().saturating_sub(10);
    // 页签切换（C# SelectRank：本地按 class 过滤 + 重新请求）
    for (btn, t) in &tabs {
        if btn.clicked && ranking.tab != t.0 {
            ranking.tab = t.0;
            if let Ok(mut sl) = scroll.single_mut() {
                sl.offset = 0;
            }
            net.send_packet(&mir2_shared::packets::client::misc::GetRanking {
                rank_index: t.0,
                online_only: ranking.online_only,
            });
            tracing::info!("🏅 排行榜页签 {}", if t.0 == 0 { "全部" } else { rank_class_name(t.0 - 1) });
        }
    }
    // 仅在线（C# OnlineOnlyButton → 重置 + 重新请求）
    if let Ok(cb) = online_boxes.single() {
        if cb.checked != ranking.online_only {
            ranking.online_only = cb.checked;
            if let Ok(mut sl) = scroll.single_mut() {
                sl.offset = 0;
            }
            net.send_packet(&mir2_shared::packets::client::misc::GetRanking {
                rank_index: ranking.tab,
                online_only: ranking.online_only,
            });
            tracing::info!("🏅 排行榜仅在线 {}", ranking.online_only);
        }
    }
    // 翻页（C# PrevButton/NextButton）
    for btn in &prev {
        if btn.clicked {
            if let Ok(mut sl) = scroll.single_mut() {
                sl.offset = sl.offset.saturating_sub(10);
            }
        }
    }
    for btn in &next {
        if btn.clicked {
            if let Ok(mut sl) = scroll.single_mut() {
                sl.offset = (sl.offset + 10).min(max_offset);
            }
        }
    }
    {
        if let Ok(mut sl) = scroll.single_mut() {
            sl.offset = sl.offset.min(max_offset);
            sl.set_total(filtered.len());
        }
    }
    for (mut text, line) in &mut lines {
        let idx = scroll.single().map(|s| s.offset + line.0).unwrap_or(line.0);
        text.0 = match filtered.get(idx) {
            Some(e) => format!(
                "#{} {} ({} Lv.{})",
                e.rank,
                e.player_name,
                rank_class_name(e.class),
                e.level
            ),
            None => String::new(),
        };
    }
    // 点击右上角关闭
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    if mouse.just_pressed(MouseButton::Left) {
        let x = 200.0 + 320.0 - 24.0;
        let y = 150.0 + 4.0;
        if cursor.x >= x && cursor.x <= x + 20.0 && cursor.y >= y && cursor.y <= y + 20.0 {
            mgr.close(DialogKind::Ranking);
        }
    }
}


/// 消费服务端排行榜事件（网络层只广播 ServerEvent）
fn ranking_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut ranking: ResMut<RankingState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        match ev {
            ServerEvent::Rankings { entries } => {
                ranking.entries = entries.clone();
            }
            ServerEvent::RankingsCleared => {
                ranking.entries.clear();
            }
            _ => {}
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn entry(rank: i32, class: u8) -> RankEntry {
        RankEntry {
            rank,
            player_name: format!("p{}", rank),
            class,
            level: 10,
            experience: 0,
        }
    }

    #[test]
    fn rank_tab_filter() {
        let entries = vec![entry(1, 0), entry(2, 1), entry(3, 2), entry(4, 3), entry(5, 4)];
        assert_eq!(filter_rank_tab(&entries, 0).len(), 5);
        assert_eq!(filter_rank_tab(&entries, 1).len(), 1);
        assert_eq!(filter_rank_tab(&entries, 1)[0].rank, 1);
        assert_eq!(filter_rank_tab(&entries, 4)[0].rank, 4);
        assert_eq!(filter_rank_tab(&entries, 5)[0].rank, 5);
        assert!(filter_rank_tab(&entries, 6).is_empty());
    }

    #[test]
    fn rank_class_names() {
        assert_eq!(rank_class_name(0), "战士");
        assert_eq!(rank_class_name(4), "弓箭手");
        assert_eq!(rank_class_name(99), "未知");
    }
}
