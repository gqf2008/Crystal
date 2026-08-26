// ============================================================================
// 排名对话框（M9 第 3 批）
// 布局参考：macroquad ranking_dialog.rs
//   - 背景 Title[728]（324x441），(200,150)，10 行 28px
// 网络：Ranking 请求 → 服务器回排名 → 显示
// ============================================================================

use bevy::prelude::*;

use crate::actor::{LocalPlayer, PlayerName};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiFont;
use crate::ui::theme::{load_lib_image, spawn_icon_button, spawn_label, spawn_panel, ImageButton};

/// 排名条目（服务端 Rankings 包）
#[derive(Debug, Clone, Default)]
pub struct RankEntry {
    pub rank: i32,
    /// 玩家 object_id（离线角色为 0；排行榜行点击查看用）
    pub player_id: u32,
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
    /// 我的排名（C# MyRank；0=未上榜）
    pub my_rank: i32,
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

/// 我的排名标签（C# MyRank）
#[derive(Component)]
pub struct RankingMyRank;

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
    ranking: Res<RankingState>,
) {
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    // bevy_ui 面板 Title[728]（324x441 @ 200,150）——bevy_ui 迁移样板
    let Some(bg) = crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Title, 728) else {
        return;
    };
    let panel = crate::ui::theme::spawn_panel(&mut commands, bg, 200.0, 150.0, 324.0, 441.0, 40);
    commands.entity(panel).insert((DialogRoot(DialogKind::Ranking), RankingWidget));

    commands.entity(panel).with_children(|p| {
        // 关闭 X（C# relative (289,3) → 面板内 (296,4)）
        if let (Some(n), Some(h), Some(pr)) = (
            crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            crate::ui::theme::spawn_icon_button(p, n, h, pr, 296.0, 4.0, 20.0, 20.0, 10)
                .insert(RankingClose);
        }
        // 标题
        crate::ui::theme::spawn_label(
            p,
            &font,
            "排行榜",
            130.0,
            0.0,
            16.0,
            Color::srgb(1.0, 1.0, 0.3),
            9,
        );
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
            crate::ui::theme::spawn_label(
                p,
                &font,
                label,
                10.0 + i as f32 * 46.0,
                18.0,
                12.0,
                Color::srgb(0.8, 0.9, 1.0),
                9,
            )
            .insert((RankingTab(*t), Button));
        }
        // 上一页 / 下一页
        crate::ui::theme::spawn_label(
            p,
            &font,
            "上一页",
            10.0,
            410.0,
            12.0,
            Color::srgb(0.8, 0.9, 1.0),
            9,
        )
        .insert((RankingPrev, Button));
        crate::ui::theme::spawn_label(
            p,
            &font,
            "下一页",
            80.0,
            410.0,
            12.0,
            Color::srgb(0.8, 0.9, 1.0),
            9,
        )
        .insert((RankingNext, Button));
        // 仅在线（Prguse 2086 未勾 / 2087 勾选）
        if let (Some(u), Some(t)) = (
            crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 2086),
            crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 2087),
        ) {
            crate::ui::theme::spawn_icon_button(p, u.clone(), t.clone(), u, 190.0, 410.0, 16.0, 14.0, 9)
                .insert(RankingOnlineOnly);
        }
        crate::ui::theme::spawn_label(
            p,
            &font,
            "仅在线",
            210.0,
            410.0,
            12.0,
            Color::srgb(0.8, 0.9, 1.0),
            9,
        );
        // 10 行（bevy_ui 文本；行点击查看暂缓，后续做成可点击行）
        for i in 0..10usize {
            crate::ui::theme::spawn_label(
                p,
                &font,
                "",
                10.0,
                98.0 + i as f32 * 28.0,
                13.0,
                Color::WHITE,
                9,
            )
            .insert(RankingLine(i));
        }
        // 我的排名
        crate::ui::theme::spawn_label(
            p,
            &font,
            "我的排名：--",
            10.0,
            388.0,
            12.0,
            Color::srgb(1.0, 0.9, 0.3),
            9,
        )
        .insert(RankingMyRank);
    });
}

fn ranking_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut ranking: ResMut<RankingState>,
    net: Res<NetConnection>,
    local_player: Query<&PlayerName, With<LocalPlayer>>,
    mut widgets: Query<&mut Visibility, With<RankingWidget>>,
    close: Query<(Entity, &Interaction), (With<RankingClose>, Without<RankingTab>)>,
    tabs: Query<(Entity, &Interaction, &RankingTab)>,
    prev: Query<(Entity, &Interaction), (With<RankingPrev>, Without<RankingTab>, Without<RankingNext>)>,
    next: Query<
        (Entity, &Interaction),
        (With<RankingNext>, Without<RankingTab>, Without<RankingPrev>),
    >,
    mut online: Query<
        (Entity, &Interaction, &ImageButton, &mut ImageNode),
        (With<RankingOnlineOnly>, Without<RankingTab>),
    >,
    mut my_rank_text: Query<&mut Text, (With<RankingMyRank>, Without<RankingLine>)>,
    mut lines: Query<(&mut Text, &RankingLine), Without<RankingMyRank>>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
    mut requested: Local<bool>,
    mut offset: Local<usize>,
) {
    // Interaction 边沿检测：仅当从非 Pressed → Pressed 那帧触发一次（bevy_ui 无 just_pressed）
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }

    let open = ranking.visible || mgr.is_open(DialogKind::Ranking);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        *requested = false;
        *offset = 0;
        return;
    }

    // 关闭（点 X → 关闭排行榜）
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::Ranking);
        }
    }
    // 打开瞬间请求排行榜（C# RankingDialog.Show → GetRanking）
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
    // 页签切换
    for (e, inter, t) in &tabs {
        if edge(e, inter, &mut prev_inter) && ranking.tab != t.0 {
            ranking.tab = t.0;
            *offset = 0;
            net.send_packet(&mir2_shared::packets::client::misc::GetRanking {
                rank_index: t.0,
                online_only: ranking.online_only,
            });
            tracing::info!(
                "🏅 排行榜页签 {}",
                if t.0 == 0 { "全部" } else { rank_class_name(t.0 - 1) }
            );
        }
    }
    // 上一页 / 下一页
    for (e, inter) in &prev {
        if edge(e, inter, &mut prev_inter) {
            *offset = offset.saturating_sub(10);
        }
    }
    for (e, inter) in &next {
        if edge(e, inter, &mut prev_inter) {
            *offset = (*offset + 10).min(max_offset);
        }
    }
    *offset = (*offset).min(max_offset);
    // 仅在线（切换 + 帧同步）
    for (e, inter, ib, mut node) in &mut online {
        if edge(e, inter, &mut prev_inter) {
            ranking.online_only = !ranking.online_only;
            *offset = 0;
            net.send_packet(&mir2_shared::packets::client::misc::GetRanking {
                rank_index: ranking.tab,
                online_only: ranking.online_only,
            });
            tracing::info!("🏅 排行榜仅在线 {}", ranking.online_only);
        }
        let want = if ranking.online_only { &ib.pressed } else { &ib.normal };
        if node.image != *want {
            node.image = want.clone();
        }
    }
    // 行文本
    for (mut text, line) in &mut lines {
        let idx = *offset + line.0;
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
    // 我的排名
    let self_name = local_player.single().map(|n| n.0.clone()).unwrap_or_default();
    for mut text in &mut my_rank_text {
        text.0 = if ranking.my_rank > 0 {
            format!("我的排名：第 {} 名", ranking.my_rank)
        } else {
            "我的排名：未上榜".to_string()
        };
    }
    let _ = self_name; // 行点击查看（Inspect）暂缓迁移
}

/// 消费服务端排行榜事件（网络层只广播 ServerEvent）
fn ranking_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut ranking: ResMut<RankingState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        match ev {
            ServerEvent::Rankings { entries, my_rank } => {
                ranking.entries = entries.clone();
                ranking.my_rank = *my_rank;
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
            player_id: 0,
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
