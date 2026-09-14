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
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont};
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
    /// 当前页首行在过滤后列表中的下标（翻页游标；跨系统共享，见 `ranking_row_click_system`）
    pub page_offset: usize,
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

/// C# `GameScene.InspectTime = CMain.Time + 500`（`RankingDialog.cs:376-378`）——排行榜行点击 500ms 节流
const RANK_INSPECT_COOLDOWN_SECS: f64 = 0.5;

/// #1225：当前页第 `line` 行对应的条目（C# `RankingRow` 是全局序号，本端按 `offset + line` 的页窗口取值）。
fn rank_row_entry<'a>(
    filtered: &'a [RankEntry],
    offset: usize,
    line: usize,
) -> Option<&'a RankEntry> {
    filtered.get(offset + line)
}

/// #1225：行点击节流判定（C# `if (CMain.Time <= GameScene.InspectTime) return;`——相等也算冷却中）
fn rank_inspect_allowed(now_secs: f64, cooldown_until_secs: f64) -> bool {
    now_secs > cooldown_until_secs
}

/// Interaction 边沿检测：仅当从非 Pressed → Pressed 那帧触发一次（bevy_ui 无 just_pressed）
fn edge(
    e: Entity,
    inter: &Interaction,
    prev: &mut std::collections::HashMap<Entity, Interaction>,
) -> bool {
    let was = prev.insert(e, *inter);
    *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
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
            (ranking_ui_system, ranking_row_click_system).run_if(in_state(AppState::Game)),
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
    mut cjk_font: ResMut<UiCjkFont>,
    mut libs: ResMut<GameLibraries>,
    ranking: Res<RankingState>,
) {
    // 面板内文案全是中文：#2775 与批17 同因（Arial 无 CJK 且 parley 的 Hani 回退只在
    // 实体首次排版生效）——本窗原先整屏豆腐，改用共享宋体主字体。
    let font = shared_cjk_font(&mut fonts, &mut cjk_font);

    // bevy_ui 面板 Title[728]（324x441 @ 200,150）——bevy_ui 迁移样板
    let Some(bg) =
        crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Title, 728)
    else {
        return;
    };
    let panel = crate::ui::theme::spawn_panel(&mut commands, bg, 200.0, 150.0, 324.0, 441.0, 40);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::Ranking), RankingWidget));

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
        // #2775：Hint 逐项取 C# `RankingDialog.cs:65/89/101/77/113/125` 的文案
        //（AllButton=总榜前 20、WarButton=战士前 20、WizButton=法师前 20、TaoButton=道士前 20、
        // SinButton=刺客前 20、ArchButton=弓箭手前 20；C# `SelectRank(i)` 的 i 即此处 RankingTab 值）
        let tabs: [(u8, &str, &str); 6] = [
            (0, "全部", "总榜前 20"),
            (1, "战士", "战士前 20"),
            (2, "法师", "法师前 20"),
            (3, "道士", "道士前 20"),
            (4, "刺客", "刺客前 20"),
            (5, "弓手", "弓箭手前 20"),
        ];
        for (i, (t, label, hint)) in tabs.iter().enumerate() {
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
            .insert((
                RankingTab(*t),
                Button,
                crate::ui::tooltip::UiHint {
                    text: (*hint).to_string(),
                },
            ));
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
            crate::ui::theme::spawn_icon_button(
                p,
                u.clone(),
                t.clone(),
                u,
                190.0,
                410.0,
                16.0,
                14.0,
                9,
            )
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
        // 10 行（bevy_ui 文本 + 可点击；C# `RankingDialog.cs:336` `RankingRow.Click → Inspect()`）
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
            .insert((RankingLine(i), Button));
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
    prev: Query<
        (Entity, &Interaction),
        (With<RankingPrev>, Without<RankingTab>, Without<RankingNext>),
    >,
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
) {
    let open = ranking.visible || mgr.is_open(DialogKind::Ranking);
    for mut vis in widgets.iter_mut() {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        *requested = false;
        ranking.page_offset = 0;
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
            ranking.page_offset = 0;
            net.send_packet(&mir2_shared::packets::client::misc::GetRanking {
                rank_index: t.0,
                online_only: ranking.online_only,
            });
            tracing::info!(
                "🏅 排行榜页签 {}",
                if t.0 == 0 {
                    "全部"
                } else {
                    rank_class_name(t.0 - 1)
                }
            );
        }
    }
    // 上一页 / 下一页
    for (e, inter) in &prev {
        if edge(e, inter, &mut prev_inter) {
            ranking.page_offset = ranking.page_offset.saturating_sub(10);
        }
    }
    for (e, inter) in &next {
        if edge(e, inter, &mut prev_inter) {
            ranking.page_offset = (ranking.page_offset + 10).min(max_offset);
        }
    }
    ranking.page_offset = ranking.page_offset.min(max_offset);
    // 仅在线（切换 + 帧同步）
    for (e, inter, ib, mut node) in &mut online {
        if edge(e, inter, &mut prev_inter) {
            ranking.online_only = !ranking.online_only;
            ranking.page_offset = 0;
            net.send_packet(&mir2_shared::packets::client::misc::GetRanking {
                rank_index: ranking.tab,
                online_only: ranking.online_only,
            });
            tracing::info!("🏅 排行榜仅在线 {}", ranking.online_only);
        }
        let want = if ranking.online_only {
            &ib.pressed
        } else {
            &ib.normal
        };
        if node.image != *want {
            node.image = want.clone();
        }
    }
    // 行文本
    for (mut text, line) in &mut lines {
        let idx = ranking.page_offset + line.0;
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
    let self_name = local_player
        .single()
        .map(|n| n.0.clone())
        .unwrap_or_default();
    for mut text in &mut my_rank_text {
        text.0 = if ranking.my_rank > 0 {
            format!("我的排名：第 {} 名", ranking.my_rank)
        } else {
            "我的排名：未上榜".to_string()
        };
    }
    let _ = self_name;
}

/// 行点击查看（C# `RankingDialog.cs:336` `RankingRow.Click → Inspect()`；`:374-380`：
/// `if (CMain.Time <= GameScene.InspectTime) return;` 500ms 节流 → `InspectDialog.InspectID = Index`
/// → `C.Inspect{ObjectID, Ranking=true}`）
///
/// 独立成系统：`ranking_ui_system` 参数已达 Bevy 16 上限（加查询即编译失败须拆独立系统）。
fn ranking_row_click_system(
    mut mgr: ResMut<DialogManager>,
    ranking: Res<RankingState>,
    net: Res<NetConnection>,
    rows: Query<(Entity, &Interaction, &RankingLine)>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
    time: Res<Time>,
    mut inspect_cooldown_until: Local<f64>,
) {
    if !(ranking.visible || mgr.is_open(DialogKind::Ranking)) {
        return;
    }
    let filtered = filter_rank_tab(&ranking.entries, ranking.tab);
    for (e, inter, line) in &rows {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        let Some(entry) = rank_row_entry(&filtered, ranking.page_offset, line.0) else {
            continue;
        };
        let now = time.elapsed_secs_f64();
        if !rank_inspect_allowed(now, *inspect_cooldown_until) {
            continue;
        }
        *inspect_cooldown_until = now + RANK_INSPECT_COOLDOWN_SECS;
        if !mgr.is_open(DialogKind::Inspect) {
            mgr.open(DialogKind::Inspect);
        }
        net.send_packet(&mir2_shared::packets::client::chat::Inspect {
            object_id: entry.player_id,
            ranking: true,
            name: entry.player_name.clone(),
        });
        tracing::info!(
            "🔍 查看排行榜玩家 {} (id={})",
            entry.player_name,
            entry.player_id
        );
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
        let entries = vec![
            entry(1, 0),
            entry(2, 1),
            entry(3, 2),
            entry(4, 3),
            entry(5, 4),
        ];
        assert_eq!(filter_rank_tab(&entries, 0).len(), 5);
        assert_eq!(filter_rank_tab(&entries, 1).len(), 1);
        assert_eq!(filter_rank_tab(&entries, 1)[0].rank, 1);
        assert_eq!(filter_rank_tab(&entries, 4)[0].rank, 4);
        assert_eq!(filter_rank_tab(&entries, 5)[0].rank, 5);
        assert!(filter_rank_tab(&entries, 6).is_empty());
    }

    /// #1225：行点击 → 当前页条目映射 + 500ms 节流（C# `RankingDialog.cs:374-380`）
    #[test]
    fn rank_row_click_window_and_throttle() {
        let entries: Vec<RankEntry> = (1..=25).map(|i| entry(i, 0)).collect();
        let filtered = filter_rank_tab(&entries, 0);
        // 第一页第 0 行 = 第 1 名
        assert_eq!(rank_row_entry(&filtered, 0, 0).map(|e| e.rank), Some(1));
        // 第二页（offset=10）第 0 行 = 第 11 名、第 4 行 = 第 15 名
        assert_eq!(rank_row_entry(&filtered, 10, 0).map(|e| e.rank), Some(11));
        assert_eq!(rank_row_entry(&filtered, 10, 4).map(|e| e.rank), Some(15));
        // 越界行（列表不足）→ None，不发包
        assert!(rank_row_entry(&filtered, 20, 5).is_none());
        // 节流：冷却期内不允许；`<=` 边界同样算冷却中（C# `CMain.Time <= InspectTime`）
        assert!(rank_inspect_allowed(1.0, 0.0));
        assert!(rank_inspect_allowed(1.51, 1.0));
        assert!(!rank_inspect_allowed(1.4, 1.5));
        assert!(!rank_inspect_allowed(1.5, 1.5));
    }

    /// #1225 行为级：真实 `App` + 真实 [`ranking_row_click_system`] —— 按下行 → 发
    /// `Inspect{object_id, ranking:true}` + 打开查看窗；500ms 冷却内再按不发包，冷却过后恢复。
    #[test]
    fn rank_row_click_emits_inspect_with_cooldown() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<DialogManager>();
        app.insert_resource(NetConnection::default());
        app.add_systems(Update, ranking_row_click_system);
        // 第二页（offset=10）第 1 行 → 第 12 名（player_id=120，仅 id 递增便于断言）
        let mut ranking = RankingState {
            visible: true,
            ..Default::default()
        };
        ranking.entries = (1..=12)
            .map(|i| RankEntry {
                player_id: i as u32 * 10,
                ..entry(i, 0)
            })
            .collect();
        ranking.page_offset = 10;
        app.insert_resource(ranking);
        let (tx, rx) = crossbeam_channel::unbounded::<Vec<u8>>();
        app.world_mut().resource_mut::<NetConnection>().to_server = Some(tx);
        let row = app
            .world_mut()
            .spawn((RankingLine(1), Interaction::None))
            .id();
        // 让 `Time` 走过 1s，脱离「启动瞬间 = 0」以免被 C# 同款 `<=` 边界挡住
        advance_time(&mut app, 1.0);

        // 帧 1：按下 → 发包 + 打开查看窗
        app.world_mut().entity_mut(row).insert(Interaction::Pressed);
        app.update();
        let raw = rx.try_recv().expect("按下排行榜行后应发出 Inspect 包");
        let mut cur = std::io::Cursor::new(raw);
        let inspect: mir2_shared::packets::client::chat::Inspect =
            mir2_shared::packets::base::deserialize_packet(&mut cur).expect("应为 Inspect 包");
        assert_eq!(
            inspect.object_id, 120,
            "应查看「当前页第 1 行」的条目（页窗口 offset + line）"
        );
        assert!(inspect.ranking, "排行榜查看须置 Ranking=true");
        assert_eq!(inspect.name, "p12");
        assert!(
            app.world()
                .resource::<DialogManager>()
                .is_open(DialogKind::Inspect),
            "行点击应打开查看窗"
        );

        // 帧 2：持续按下（无边沿）→ 不重复发包
        app.update();
        assert!(rx.try_recv().is_err(), "持续按下不应连续发包");

        // 帧 3-4：松开再按下 → 冷却中仍不发包
        app.world_mut().entity_mut(row).insert(Interaction::None);
        app.update();
        app.world_mut().entity_mut(row).insert(Interaction::Pressed);
        app.update();
        assert!(
            rx.try_recv().is_err(),
            "500ms 冷却内再按不发包（C# `CMain.Time <= GameScene.InspectTime`）"
        );

        // 冷却过后（>500ms）恢复响应
        advance_time(&mut app, 1.0);
        app.world_mut().entity_mut(row).insert(Interaction::None);
        app.update();
        app.world_mut().entity_mut(row).insert(Interaction::Pressed);
        app.update();
        assert!(rx.try_recv().is_ok(), "冷却过后应恢复响应");
    }

    /// 测试用：把虚拟时间推进 `secs`（`Time` 由 `MinimalPlugins` 的 `TimePlugin` 从它派生）。
    fn advance_time(app: &mut App, secs: f64) {
        app.world_mut()
            .resource_mut::<bevy::time::Time<bevy::time::Virtual>>()
            .advance_by(std::time::Duration::from_secs_f64(secs));
    }

    #[test]
    fn rank_class_names() {
        assert_eq!(rank_class_name(0), "战士");
        assert_eq!(rank_class_name(4), "弓箭手");
        assert_eq!(rank_class_name(99), "未知");
    }
}
