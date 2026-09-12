// ============================================================================
// TrustMerchant 左列筛选树（对齐 C# `TrustMerchantDialog.SetupFilters/DrawFilters`）
//
// 参考 C#：
//   - `SetupFilters()`（TrustMerchantDialog.cs:676-723）：8 个主过滤器 + 子过滤器
//     （类型/形状范围，`Filter` 结构见 :1628-1637）
//   - `DrawFilters(index, subIndex)`（:725-878）：主按钮 20px 步进、展开子列表 +2、
//     子按钮 21px 步进；点击主项展开（有子项时不发搜索），点击子项发 `C.MarketSearch`
//   - 滚动条（:234-298）：Up `Prguse2[197..199]` @(108,60)、Down `[207..209]` @(108,429)、
//     PositionBar `[205/206]` @(108,73) 可拖动；`MaxLines = 19`、`PosMinY = 73`、`PosMaxY = 410`
//
// 线格式：过滤点击发 C# `C.MarketSearch{Match, Type, Usermode=false, MinShape, MaxShape, MarketType}`
// （Rust 网关 `ServerRust/src/gate/actor.rs:4390` 读的就是这个格式）。
// ============================================================================

use bevy::prelude::*;
use mir2_shared::enums::{ItemType, MarketPanelType};

use crate::game::dialogs::market::MarketState;
use crate::game::dialogs::text_input::TextInputState;
use crate::game::dialogs::{DialogKind, DialogManager};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::ui::theme::{load_lib_image, spawn_icon_button, spawn_label, ImageButton};

/// 主按钮 x（C# `btnx = 7`）
pub const FILTER_BTN_X: f32 = 7.0;
/// 起始 y（C# `btny = 60`）
pub const FILTER_BTN_Y: f32 = 60.0;
/// 主项步进（C# `btny += 20`）
pub const FILTER_MAIN_STEP: f32 = 20.0;
/// 子项步进（C# `btny += 21`）
pub const FILTER_SUB_STEP: f32 = 21.0;
/// 展开子列表前的额外间距（C# `btny += 2`）
pub const FILTER_SUB_GAP: f32 = 2.0;
/// 可视行数上限（C# `MaxLines = 19`，主+子合计）
pub const FILTER_MAX_LINES: usize = 19;
/// 标签锚点（C# 主 `(2,1)` / 子 `(10,1)`，99x18）
pub const FILTER_LABEL_MAIN_DX: f32 = 2.0;
pub const FILTER_LABEL_SUB_DX: f32 = 10.0;
pub const FILTER_LABEL_DY: f32 = 1.0;
pub const FILTER_LABEL_W: f32 = 99.0;
pub const FILTER_LABEL_H: f32 = 18.0;
/// 按钮精灵原始尺寸（`Prguse2[920..923]` 实测 100x22）
pub const FILTER_BTN_W: f32 = 100.0;
pub const FILTER_BTN_H: f32 = 22.0;
/// 滚动条 x（C# Up/Down/PositionBar 都在 x=108）
pub const FILTER_BAR_X: f32 = 108.0;
/// Up @(108,60)、Down @(108,429)、PositionBar @(108,73)
pub const FILTER_UP_Y: f32 = 60.0;
pub const FILTER_DOWN_Y: f32 = 429.0;
pub const FILTER_BAR_Y: f32 = 73.0;
/// C# `PosMinY = UpButton.Y + 13`、`PosMaxY = DownButton.Y - 19`
pub const FILTER_BAR_MIN_Y: f32 = FILTER_UP_Y + 13.0;
pub const FILTER_BAR_MAX_Y: f32 = FILTER_DOWN_Y - 19.0;
/// `Prguse2[197..199]`/`[207..209]` 实测 12x12；`[205/206]` 实测 12x18
pub const FILTER_ARROW_W: f32 = 12.0;
pub const FILTER_ARROW_H: f32 = 12.0;
pub const FILTER_HANDLE_W: f32 = 12.0;
pub const FILTER_HANDLE_H: f32 = 18.0;
/// C# `Title[786]` 面板尺寸（行/滚动条不得越界）
pub const FILTER_PANEL_H: f32 = 478.0;

/// C# `TrustMerchantDialog.Filter`（Index/Title/Type/MinShape/MaxShape + SubFilters）
#[derive(Debug, Clone, PartialEq)]
pub struct MarketFilter {
    /// C# `Filter.Index`
    pub index: i32,
    pub title: &'static str,
    /// C# `Filter.Type`；`None` = 只展开/高亮，不发 `MarketSearch`
    pub item_type: Option<ItemType>,
    pub min_shape: i16,
    pub max_shape: i16,
    pub subs: Vec<MarketFilter>,
}

impl MarketFilter {
    fn main(index: i32, title: &'static str, item_type: Option<ItemType>) -> Self {
        Self {
            index,
            title,
            item_type,
            min_shape: 0,
            max_shape: i16::MAX,
            subs: Vec::new(),
        }
    }

    fn sub(
        index: i32,
        title: &'static str,
        item_type: ItemType,
        min_shape: i16,
        max_shape: i16,
    ) -> Self {
        Self {
            index,
            title,
            item_type: Some(item_type),
            min_shape,
            max_shape,
            subs: Vec::new(),
        }
    }
}

/// C# `SetupFilters()`（TrustMerchantDialog.cs:676-723）逐项移植；标题取
/// `Client/Localization/Chinese.json` 的 `ClientTextKeys` 译文。
pub fn setup_filters() -> Vec<MarketFilter> {
    // 主过滤器（C#：`all`/`weapon` 有 Type，其余 `Type = null` → 只展开不搜索）
    let all = MarketFilter::main(0, "显示所有物品", Some(ItemType::Nothing));
    let weapon = MarketFilter::main(1, "武器类物品", Some(ItemType::Weapon));
    let mut drapery = MarketFilter::main(2, "衣服类物品", None);
    let mut accessory = MarketFilter::main(3, "饰品类物品", None);
    let mut consumable = MarketFilter::main(4, "消耗品", None);
    let mut enhancement = MarketFilter::main(5, "强化", None);
    let mut book = MarketFilter::main(6, "书籍", None);
    let mut crafting = MarketFilter::main(7, "制作材料", None);

    drapery.subs = vec![
        MarketFilter::sub(201, "护甲", ItemType::Armour, 0, i16::MAX),
        MarketFilter::sub(202, "头盔", ItemType::Helmet, 0, i16::MAX),
        MarketFilter::sub(203, "腰带", ItemType::Belt, 0, i16::MAX),
        MarketFilter::sub(204, "靴子", ItemType::Boots, 0, i16::MAX),
        MarketFilter::sub(205, "宝石/石头", ItemType::Stone, 0, i16::MAX),
    ];
    accessory.subs = vec![
        MarketFilter::sub(301, "项链", ItemType::Necklace, 0, i16::MAX),
        MarketFilter::sub(302, "手镯", ItemType::Bracelet, 0, i16::MAX),
        MarketFilter::sub(303, "戒指", ItemType::Ring, 0, i16::MAX),
    ];
    consumable.subs = vec![
        MarketFilter::sub(401, "恢复药水", ItemType::Potion, 0, 2),
        MarketFilter::sub(402, "增益药水", ItemType::Potion, 3, 4),
        MarketFilter::sub(403, "卷轴/药油", ItemType::Scroll, 0, i16::MAX),
        MarketFilter::sub(404, "杂项物品", ItemType::Script, 0, i16::MAX),
    ];
    enhancement.subs = vec![
        MarketFilter::sub(501, "宝石", ItemType::Potion, 3, 3),
        MarketFilter::sub(502, "法球", ItemType::Potion, 4, 4),
    ];
    book.subs = vec![
        MarketFilter::sub(601, "战士", ItemType::Book, 0, 30),
        MarketFilter::sub(602, "法师", ItemType::Book, 31, 60),
        MarketFilter::sub(603, "道士", ItemType::Book, 61, 90),
        MarketFilter::sub(604, "刺客", ItemType::Book, 91, 120),
        MarketFilter::sub(605, "弓箭手", ItemType::Book, 121, 150),
    ];
    crafting.subs = vec![
        MarketFilter::sub(701, "材料", ItemType::CraftingMaterial, 0, i16::MAX),
        MarketFilter::sub(703, "肉类", ItemType::Meat, 0, i16::MAX),
        MarketFilter::sub(704, "矿石", ItemType::Ore, 0, i16::MAX),
    ];

    vec![
        all,
        weapon,
        drapery,
        accessory,
        consumable,
        enhancement,
        book,
        crafting,
    ]
}

/// 一行可见项（主项或展开的子项）
#[derive(Debug, Clone, PartialEq)]
pub struct FilterRow {
    pub is_sub: bool,
    /// 主项在 `Filters` 中的下标
    pub main_pos: usize,
    /// 子项在 `SubFilters` 中的下标（主项行 = None）
    pub sub_pos: Option<usize>,
    /// C# `Filter.Index`
    pub index: i32,
    pub title: &'static str,
    pub item_type: Option<ItemType>,
    pub min_shape: i16,
    pub max_shape: i16,
    pub selected: bool,
    /// 行 y（相对面板，C# `btny` 递推结果）
    pub y: f32,
}

/// `DrawFilters` 的可见行计算（含 y 递推、Skip 跳过、MaxLines 截断）。
pub fn visible_rows(
    filters: &[MarketFilter],
    selected_index: i32,
    selected_sub_index: i32,
    skip: usize,
    max_lines: usize,
) -> Vec<FilterRow> {
    let mut out = Vec::new();
    let mut current = 0usize;
    let mut skipped = skip;
    let mut y = FILTER_BTN_Y;
    for (mi, item) in filters.iter().enumerate() {
        if skipped > 0 {
            skipped -= 1;
            continue;
        }
        if current >= max_lines {
            break;
        }
        current += 1;
        out.push(FilterRow {
            is_sub: false,
            main_pos: mi,
            sub_pos: None,
            index: item.index,
            title: item.title,
            item_type: item.item_type,
            min_shape: item.min_shape,
            max_shape: item.max_shape,
            selected: item.index == selected_index,
            y,
        });
        y += FILTER_MAIN_STEP;
        if item.index == selected_index {
            if !item.subs.is_empty() {
                y += FILTER_SUB_GAP;
            }
            for (si, sub) in item.subs.iter().enumerate() {
                if skipped > 0 {
                    skipped -= 1;
                    continue;
                }
                if current >= max_lines {
                    break;
                }
                current += 1;
                out.push(FilterRow {
                    is_sub: true,
                    main_pos: mi,
                    sub_pos: Some(si),
                    index: sub.index,
                    title: sub.title,
                    item_type: sub.item_type,
                    min_shape: sub.min_shape,
                    max_shape: sub.max_shape,
                    selected: sub.index == selected_sub_index,
                    y,
                });
                y += FILTER_SUB_STEP;
            }
        }
    }
    out
}

/// C# `PossibleTotal`：主项数 + 选中主项的子项数
pub fn possible_total(filters: &[MarketFilter], selected_index: i32) -> usize {
    let mut total = filters.len();
    if let Some(f) = filters.iter().find(|f| f.index == selected_index) {
        total += f.subs.len();
    }
    total
}

/// C# 滚动上限：`Skip + MaxLines < PossibleTotal`
pub fn max_skip(filters: &[MarketFilter], selected_index: i32, max_lines: usize) -> usize {
    possible_total(filters, selected_index).saturating_sub(max_lines)
}

/// PositionBar 的 y（C# `UpdatePositionBar`；不满一屏返回 `None` = 隐藏）
pub fn handle_y(skip: usize, total: usize, max_lines: usize) -> Option<f32> {
    if total <= max_lines {
        return None;
    }
    let interval = (FILTER_BAR_MAX_Y - FILTER_BAR_MIN_Y) / (total - max_lines) as f32;
    Some((FILTER_BAR_MIN_Y + skip as f32 * interval).clamp(FILTER_BAR_MIN_Y, FILTER_BAR_MAX_Y))
}

/// 拖动 PositionBar 时由 y 反算 `Skip`（C# `PositionBar_OnMoving`）
pub fn skip_from_handle_y(y: f32, total: usize, max_lines: usize) -> usize {
    if total <= max_lines {
        return 0;
    }
    let interval = (FILTER_BAR_MAX_Y - FILTER_BAR_MIN_Y) / (total - max_lines) as f32;
    if interval <= 0.0 {
        return 0;
    }
    let y = y.clamp(FILTER_BAR_MIN_Y, FILTER_BAR_MAX_Y);
    (((y - FILTER_BAR_MIN_Y) / interval) as i64).clamp(0, (total - max_lines) as i64) as usize
}

/// 行按钮（`MarketFilterRow(槽位)`，槽位 0..FILTER_MAX_LINES）
#[derive(Component)]
pub struct MarketFilterRow(pub usize);

/// 行标签（C# 是按钮子控件；Bevy 用同级节点，缩进 2/10，z 高于按钮）
#[derive(Component)]
pub struct MarketFilterRowLabel(pub usize);

/// 上翻 / 下翻 / 拖动手柄
#[derive(Component)]
pub struct MarketFilterUpBtn;

#[derive(Component)]
pub struct MarketFilterDownBtn;

#[derive(Component)]
pub struct MarketFilterBar;

/// 行按钮四帧（`Prguse2[920..923]`）
#[derive(Resource)]
pub struct MarketFilterSprites {
    pub main: Handle<Image>,
    pub main_sel: Handle<Image>,
    pub sub: Handle<Image>,
    pub sub_sel: Handle<Image>,
}

/// 生成筛选树（父节点 = TrustMerchant 面板；由 `market.rs::spawn_market` 调用）。
/// 返回行按钮四帧句柄，调用方 `insert_resource`。
pub fn spawn_filter_tree(
    parent: &mut ChildSpawnerCommands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    font: &Handle<Font>,
) -> Option<MarketFilterSprites> {
    let (Some(main), Some(main_sel), Some(sub), Some(sub_sel)) = (
        load_lib_image(libs, images, LibraryName::Prguse2, 920),
        load_lib_image(libs, images, LibraryName::Prguse2, 921),
        load_lib_image(libs, images, LibraryName::Prguse2, 922),
        load_lib_image(libs, images, LibraryName::Prguse2, 923),
    ) else {
        return None;
    };
    // 19 个行槽（C# 每次 `DrawFilters` 重建按钮；Bevy 复用固定槽位，逐帧写位置/文本）
    for slot in 0..FILTER_MAX_LINES {
        spawn_icon_button(
            parent,
            main.clone(),
            main_sel.clone(),
            main_sel.clone(),
            FILTER_BTN_X,
            FILTER_BTN_Y,
            FILTER_BTN_W,
            FILTER_BTN_H,
            11,
        )
        .insert(MarketFilterRow(slot));
        spawn_label(
            parent,
            font,
            "",
            FILTER_BTN_X + FILTER_LABEL_MAIN_DX,
            FILTER_BTN_Y + FILTER_LABEL_DY,
            12.0,
            Color::WHITE,
            12,
        )
        .insert(MarketFilterRowLabel(slot));
    }
    // 滚动条（C# Up/Down/PositionBar 都在 x=108）
    if let (Some(n), Some(h), Some(pr)) = (
        load_lib_image(libs, images, LibraryName::Prguse2, 197),
        load_lib_image(libs, images, LibraryName::Prguse2, 198),
        load_lib_image(libs, images, LibraryName::Prguse2, 199),
    ) {
        spawn_icon_button(
            parent,
            n,
            h,
            pr,
            FILTER_BAR_X,
            FILTER_UP_Y,
            FILTER_ARROW_W,
            FILTER_ARROW_H,
            11,
        )
        .insert(MarketFilterUpBtn);
    }
    if let (Some(n), Some(h), Some(pr)) = (
        load_lib_image(libs, images, LibraryName::Prguse2, 207),
        load_lib_image(libs, images, LibraryName::Prguse2, 208),
        load_lib_image(libs, images, LibraryName::Prguse2, 209),
    ) {
        spawn_icon_button(
            parent,
            n,
            h,
            pr,
            FILTER_BAR_X,
            FILTER_DOWN_Y,
            FILTER_ARROW_W,
            FILTER_ARROW_H,
            11,
        )
        .insert(MarketFilterDownBtn);
    }
    if let (Some(n), Some(h)) = (
        load_lib_image(libs, images, LibraryName::Prguse2, 205),
        load_lib_image(libs, images, LibraryName::Prguse2, 206),
    ) {
        spawn_icon_button(
            parent,
            n,
            h.clone(),
            h,
            FILTER_BAR_X,
            FILTER_BAR_Y,
            FILTER_HANDLE_W,
            FILTER_HANDLE_H,
            12,
        )
        .insert(MarketFilterBar);
    }
    Some(MarketFilterSprites {
        main,
        main_sel,
        sub,
        sub_sel,
    })
}

/// 筛选树主系统：显隐（页签）、行视觉/文本、点击搜索、上下翻、拖动条。
/// 独立系统（不并入 `market_ui_system`，避免 Bevy 16 参数上限）。
#[allow(clippy::too_many_arguments)]
pub fn market_filter_system(
    mgr: Res<DialogManager>,
    mut market: ResMut<MarketState>,
    net: Res<NetConnection>,
    input: Res<TextInputState>,
    sprites: Option<Res<MarketFilterSprites>>,
    mut rows: Query<
        (
            Entity,
            &Interaction,
            &MarketFilterRow,
            &mut ImageButton,
            &mut Node,
            &mut Visibility,
        ),
        Without<MarketFilterRowLabel>,
    >,
    mut labels: Query<
        (&MarketFilterRowLabel, &mut Text, &mut Node, &mut Visibility),
        (Without<MarketFilterRow>, Without<MarketFilterBar>),
    >,
    up_btn: Query<(Entity, &Interaction), (With<MarketFilterUpBtn>, Without<MarketFilterRow>)>,
    down_btn: Query<(Entity, &Interaction), (With<MarketFilterDownBtn>, Without<MarketFilterRow>)>,
    mut bar: Query<
        (&Interaction, &mut Node, &mut Visibility),
        (
            With<MarketFilterBar>,
            Without<MarketFilterRow>,
            Without<MarketFilterRowLabel>,
        ),
    >,
    windows: Query<&Window>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
    mut grab: Local<Option<f32>>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    if !mgr.is_open(DialogKind::Market) {
        return;
    }
    // C# 筛选树只在 Market / GameShop 页签可见（寄售/拍卖页签整列隐藏）
    let show = matches!(
        market.panel,
        MarketPanelType::Market | MarketPanelType::GameShop
    );
    let filters = setup_filters();
    let total = possible_total(&filters, market.filter_index);
    let max_line_skip = max_skip(&filters, market.filter_index, FILTER_MAX_LINES);
    if market.filter_skip > max_line_skip {
        market.filter_skip = max_line_skip;
    }
    let visible = visible_rows(
        &filters,
        market.filter_index,
        market.filter_sub_index.unwrap_or(-1),
        market.filter_skip,
        FILTER_MAX_LINES,
    );
    // 行：位置 / 精灵 / 可见性 / 点击
    let mut clicked: Option<usize> = None;
    for (e, inter, row, mut btn, mut node, mut vis) in &mut rows {
        let Some(data) = visible.get(row.0) else {
            *vis = Visibility::Hidden;
            prev_inter.insert(e, *inter);
            continue;
        };
        *vis = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if node.top != Val::Px(data.y) {
            node.top = Val::Px(data.y);
        }
        if let Some(sp) = sprites.as_ref() {
            let (normal, hi) = if data.is_sub {
                (
                    if data.selected {
                        sp.sub_sel.clone()
                    } else {
                        sp.sub.clone()
                    },
                    sp.sub_sel.clone(),
                )
            } else {
                (
                    if data.selected {
                        sp.main_sel.clone()
                    } else {
                        sp.main.clone()
                    },
                    sp.main_sel.clone(),
                )
            };
            btn.normal = normal;
            btn.hover = hi.clone();
            btn.pressed = hi;
        }
        if edge(e, inter, &mut prev_inter) {
            clicked = Some(row.0);
        }
    }
    // 标签：缩进 / 文本 / 可见性（与行同槽）
    for (label, mut text, mut node, mut vis) in &mut labels {
        let data = visible.get(label.0);
        *vis = match data {
            Some(_) if show => Visibility::Visible,
            _ => Visibility::Hidden,
        };
        let Some(data) = data else {
            continue;
        };
        let left = FILTER_BTN_X
            + if data.is_sub {
                FILTER_LABEL_SUB_DX
            } else {
                FILTER_LABEL_MAIN_DX
            };
        if node.left != Val::Px(left) {
            node.left = Val::Px(left);
        }
        let top = data.y + FILTER_LABEL_DY;
        if node.top != Val::Px(top) {
            node.top = Val::Px(top);
        }
        if text.0 != data.title {
            text.0 = data.title.to_string();
        }
    }
    // 点击：主项展开（有子项不发搜索）；无子项主项/子项发搜索（C# `DrawFilters` 内 Click）
    if let Some(slot) = clicked {
        if let Some(data) = visible.get(slot).cloned() {
            if data.is_sub {
                market.filter_index = filters[data.main_pos].index;
                market.filter_sub_index = Some(data.index);
                if let Some(t) = data.item_type {
                    send_filter_search(&net, &input, &market, t, data.min_shape, data.max_shape);
                }
            } else {
                let has_subs = filters
                    .get(data.main_pos)
                    .map(|f| !f.subs.is_empty())
                    .unwrap_or(false);
                market.filter_index = data.index;
                market.filter_sub_index = None;
                if !has_subs {
                    if let Some(t) = data.item_type {
                        send_filter_search(
                            &net,
                            &input,
                            &market,
                            t,
                            data.min_shape,
                            data.max_shape,
                        );
                    }
                }
            }
        }
    }
    // 上翻/下翻（C# `UpButton/DownButton.Click`）
    for (e, inter) in &up_btn {
        if edge(e, inter, &mut prev_inter) && market.filter_skip > 0 {
            market.filter_skip -= 1;
        }
    }
    for (e, inter) in &down_btn {
        if edge(e, inter, &mut prev_inter) && market.filter_skip < max_line_skip {
            market.filter_skip += 1;
        }
    }
    // PositionBar：位置随 Skip，按住可拖动（C# `PositionBar_OnMoving`）
    let target_y = handle_y(market.filter_skip, total, FILTER_MAX_LINES);
    for (inter, mut node, mut vis) in &mut bar {
        let Some(y) = target_y else {
            *vis = Visibility::Hidden;
            continue;
        };
        *vis = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        let mut dragged = false;
        if *inter == Interaction::Pressed && show {
            if let Ok(window) = windows.single() {
                if let Some(cursor) = window.cursor_position() {
                    let off = *grab.get_or_insert(cursor.y - y);
                    let moved = (cursor.y - off).clamp(FILTER_BAR_MIN_Y, FILTER_BAR_MAX_Y);
                    market.filter_skip = skip_from_handle_y(moved, total, FILTER_MAX_LINES);
                    node.top = Val::Px(moved);
                    dragged = true;
                }
            }
        }
        if !dragged {
            if !mouse.pressed(MouseButton::Left) {
                *grab = None;
            }
            if node.top != Val::Px(y) {
                node.top = Val::Px(y);
            }
        }
    }
}

/// C# 过滤点击发 `C.MarketSearch`（`Usermode = false`，`Match` = 搜索框内容）
fn send_filter_search(
    net: &NetConnection,
    input: &TextInputState,
    market: &MarketState,
    item_type: ItemType,
    min_shape: i16,
    max_shape: i16,
) {
    let match_text = input.texts.get(5).cloned().unwrap_or_default();
    net.send_packet(&mir2_shared::packets::client::market::MarketSearch {
        match_text: match_text.clone(),
        item_type,
        user_mode: false,
        min_shape,
        max_shape,
        market_type: market.panel,
    });
    tracing::info!(
        "🏪 筛选搜索: type={:?} shape=[{},{}] match={}",
        item_type,
        min_shape,
        max_shape,
        match_text
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2720：筛选树锚点与 C# 字面值一致（TrustMerchantDialog.cs:234-298, 725-878）
    #[test]
    fn filter_tree_layout_matches_csharp() {
        assert_eq!(FILTER_BTN_X, 7.0); // btnx
        assert_eq!(FILTER_BTN_Y, 60.0); // btny
        assert_eq!(FILTER_MAIN_STEP, 20.0); // btny += 20
        assert_eq!(FILTER_SUB_STEP, 21.0); // btny += 21
        assert_eq!(FILTER_SUB_GAP, 2.0); // btny += 2
        assert_eq!(FILTER_MAX_LINES, 19); // MaxLines
        assert_eq!(FILTER_LABEL_MAIN_DX, 2.0); // 主标签 (2,1)
        assert_eq!(FILTER_LABEL_SUB_DX, 10.0); // 子标签 (10,1)
        assert_eq!((FILTER_LABEL_W, FILTER_LABEL_H), (99.0, 18.0));
        assert_eq!(FILTER_BAR_X, 108.0); // Up/Down/PositionBar x
        assert_eq!(
            (FILTER_UP_Y, FILTER_DOWN_Y, FILTER_BAR_Y),
            (60.0, 429.0, 73.0)
        );
        assert_eq!(FILTER_BAR_MIN_Y, 73.0); // Up.Y + 13
        assert_eq!(FILTER_BAR_MAX_Y, 410.0); // Down.Y - 19
        assert_eq!((FILTER_BTN_W, FILTER_BTN_H), (100.0, 22.0)); // Prguse2[920] 实测
        assert!(FILTER_BTN_X + FILTER_BTN_W <= FILTER_BAR_X);
        // 面板内（C# Title[786] 492x478）
        assert!(FILTER_DOWN_Y + FILTER_ARROW_H <= FILTER_PANEL_H);
    }

    /// #2720：`SetupFilters` 主/子项与 C# 一一对应（Index/Type/形状范围）
    #[test]
    fn filter_tree_matches_csharp_setup() {
        let f = setup_filters();
        assert_eq!(f.len(), 8);
        assert_eq!(
            f.iter().map(|x| x.index).collect::<Vec<_>>(),
            vec![0, 1, 2, 3, 4, 5, 6, 7]
        );
        assert_eq!(f[0].item_type, Some(ItemType::Nothing));
        assert_eq!(f[1].item_type, Some(ItemType::Weapon));
        // C# 其余主项 Type = null（只展开不搜索）
        for i in 2..8 {
            assert_eq!(f[i].item_type, None, "主项 {} 应为 null Type", i);
        }
        assert_eq!(
            f[2].subs.iter().map(|s| s.index).collect::<Vec<_>>(),
            vec![201, 202, 203, 204, 205]
        );
        assert_eq!(
            f[6].subs.iter().map(|s| s.index).collect::<Vec<_>>(),
            vec![601, 602, 603, 604, 605]
        );
        // 形状范围（C# 消耗品/强化/书籍）
        assert_eq!((f[4].subs[0].min_shape, f[4].subs[0].max_shape), (0, 2));
        assert_eq!((f[4].subs[1].min_shape, f[4].subs[1].max_shape), (3, 4));
        assert_eq!((f[5].subs[0].min_shape, f[5].subs[0].max_shape), (3, 3));
        assert_eq!((f[5].subs[1].min_shape, f[5].subs[1].max_shape), (4, 4));
        assert_eq!((f[6].subs[0].min_shape, f[6].subs[0].max_shape), (0, 30));
        assert_eq!((f[6].subs[4].min_shape, f[6].subs[4].max_shape), (121, 150));
        // 子项 Type 全部有值（点击即搜索）
        assert!(f[2].subs.iter().all(|s| s.item_type.is_some()));
    }

    /// #2720：`DrawFilters` 行递推（主 20 / 展开 +2 / 子 21）与选中标记
    #[test]
    fn visible_rows_layout_and_selection() {
        let f = setup_filters();
        // 未展开：8 行主项，y = 60 + 20i
        let rows = visible_rows(&f, 0, -1, 0, FILTER_MAX_LINES);
        assert_eq!(rows.len(), 8);
        assert_eq!(rows[0].y, 60.0);
        assert_eq!(rows[1].y, 80.0);
        assert!(rows[0].selected && !rows[1].selected);
        // 展开 index=2（衣服）：主项后 +2，再 21 步进
        let rows = visible_rows(&f, 2, -1, 0, FILTER_MAX_LINES);
        assert_eq!(rows[2].y, 100.0);
        assert!(rows[2].selected);
        assert_eq!(rows[3].y, 122.0); // 100 + 20 + 2
        assert_eq!(rows[4].y, 143.0); // +21
        assert_eq!(rows[3].index, 201);
        assert!(rows.iter().all(|r| !r.selected || r.index == 2));
        // 选中子项 → 高亮该子项
        let rows = visible_rows(&f, 2, 203, 0, FILTER_MAX_LINES);
        assert!(rows.iter().any(|r| r.index == 203 && r.selected));
    }

    /// #2720：Skip/MaxLines/PossibleTotal 与 C# 一致
    #[test]
    fn visible_rows_skip_and_max_lines() {
        let f = setup_filters();
        assert_eq!(possible_total(&f, 0), 8);
        assert_eq!(possible_total(&f, 2), 13); // 8 + 5 子项
        assert_eq!(max_skip(&f, 0, FILTER_MAX_LINES), 0);
        assert_eq!(max_skip(&f, 2, FILTER_MAX_LINES), 0);
        // 展开 + MaxLines=19 时 13 行仍在屏内
        assert_eq!(visible_rows(&f, 2, -1, 0, FILTER_MAX_LINES).len(), 13);
        // Skip=4 → 前 4 个可见项（主项 0..3）被跳过
        let rows = visible_rows(&f, 0, -1, 4, FILTER_MAX_LINES);
        assert_eq!(rows[0].index, 4);
        assert_eq!(rows.len(), 4);
        // MaxLines 截断（缩小上限验证）
        assert_eq!(visible_rows(&f, 2, -1, 0, 5).len(), 5);
        // 不满一屏 → PositionBar 隐藏；超出 → 位置随 Skip 单调且不离轨
        assert_eq!(handle_y(0, 8, FILTER_MAX_LINES), None);
        assert_eq!(handle_y(0, 25, FILTER_MAX_LINES), Some(FILTER_BAR_MIN_Y));
        let y_mid = handle_y(3, 25, FILTER_MAX_LINES).unwrap();
        let y_max = handle_y(6, 25, FILTER_MAX_LINES).unwrap();
        assert!(y_mid > FILTER_BAR_MIN_Y && y_mid < y_max && y_max <= FILTER_BAR_MAX_Y);
        // 反算：拖到最底 → Skip = 上限
        assert_eq!(
            skip_from_handle_y(FILTER_BAR_MAX_Y, 25, FILTER_MAX_LINES),
            6
        );
        assert_eq!(
            skip_from_handle_y(FILTER_BAR_MIN_Y, 25, FILTER_MAX_LINES),
            0
        );
    }
}
