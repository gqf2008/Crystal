// ============================================================================
// 宠物对话框（M47）
// 参考：C# IntelligentCreatureDialog + ServerRust hero.rs 宠物系统
// 网络（ServerRust 实际 wire）：
//   C: RequestIntelligentCreatureUpdates[bool u8] / UpdateIntelligentCreature[type u8][pickup u8]
//   S: UpdateIntelligentCreatureList[count i32][per: type u8][pickup u8][enabled u8][hunger u8][name dotnet]
//     [active u8][filter 9×u8][grade u8][rules: minimal i32][mouse u8][mouseR i32][auto u8]
//     [autoR i32][semi u8][semiR i32][blackstone u8]（#2757 起，字段顺序见 `IntelligentCreatureRules`）
//     [icon i32][fullness i32][expire i64][blackstone_time i32]（#2761 起）
//     [creature_summoned u8][summoned_type u8][pearl_count i32]（#2761 起，C# 包尾三字段）
// ============================================================================

use bevy::prelude::*;
use mir2_shared::data::client_data::IntelligentCreatureRules;

use crate::game::dialogs::text_input::{
    TextInputDisplay, TextInputField, TextInputRect, TextInputState, TextInputSubmit,
};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_icon_button, spawn_image, spawn_label,
    spawn_label_center, spawn_panel, CloseButton, ImageButton,
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
    /// #2757 宠物规则（C# `IntelligentCreatureRules`，随 `UpdateIntelligentCreatureList` 下发）。
    /// 直接复用 `mir2_shared` 类型与其 `read_from`，与 ServerRust 侧 `write_to` 单一来源，
    /// 避免两端各写一份字段顺序（用于渲染 `CreatureInfo`/`CreatureInfo1`/`CreatureInfo2` 三行，
    /// C# `DrawCreatureAnimation`:727-735）。
    pub rules: IntelligentCreatureRules,
    /// #2761 图标（C# `IntelligentCreatureInfo.Icon` → `Prguse2[icon]`；0 = 无对应，跳过绘制）
    pub icon: i32,
    /// #2761 完整度（0..10000，C# `ClientIntelligentCreature.Fullness`）
    pub fullness: i32,
    /// #2761 到期剩余秒数（C# 客户端按 `Expire - Now` 渲染；0 = 永久 → `过期: 永不过期`）
    pub expire_secs: i64,
    /// #2761 黑曜石产出计时（秒；上限 `BLACKSTONE_PRODUCE_TIME` = 10800）
    pub blackstone_time: i32,
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
    /// #2761 是否有召唤中的宠物（C# `User.CreatureSummoned`，包尾下发）
    pub summoned: bool,
    /// #2761 召唤中的宠物种类（C# `User.SummonedCreatureType`；0 = 无）
    pub summoned_type: u8,
    /// #2761 玩家珍珠数（C# `User.PearlCount`，包尾下发 → `CreaturePearls` 标签）
    pub pearl_count: i32,
}

#[derive(Component)]
pub struct CreatureWidget;

#[derive(Component)]
pub struct CreatureClose;

#[derive(Component)]
pub struct CreatureRefresh;

/// 改名按钮（C# CreatureRenameButton Title[570-572]）
#[derive(Component)]
struct CreatureRenameBtn;

/// 解散按钮（C# DismissButton Title[580-582]）
#[derive(Component)]
struct CreatureDismissBtn;

/// 召唤按钮（C# SummonButton Title[576-578]，选中未激活宠物时显示）
#[derive(Component)]
struct CreatureSummonBtn;

/// 释放按钮（C# ReleaseButton Title[583-585]）
#[derive(Component)]
struct CreatureReleaseBtn;

/// 自动模式按钮（C# AutomaticModeButton）
#[derive(Component)]
struct CreatureAutoBtn;

/// 半自动模式按钮（C# SemiAutoModeButton）
#[derive(Component)]
struct CreatureSemiBtn;

/// 选项按钮（C# OptionsMenuButton Title[573-575]）
#[derive(Component)]
struct CreatureOptionsBtn;

/// 选项面板（C# IntelligentCreatureOptionsDialog：9 个过滤复选框 + 保存/取消）
#[derive(Component)]
struct CreatureOptionsWidget;

/// 选项行（0=全部 ... 8=其他）
#[derive(Component)]
struct CreatureOptionsLine(usize);

/// 选项保存
#[derive(Component)]
struct CreatureOptionsSave;

/// 选项取消
#[derive(Component)]
struct CreatureOptionsCancel;

/// 品质上一档（C# OptionsGradeDialog PrevButton）
#[derive(Component)]
struct CreatureGradePrev;

/// 品质下一档（C# OptionsGradeDialog NextButton）
#[derive(Component)]
struct CreatureGradeNext;

/// 品质显示（C# GradeLabel）
#[derive(Component)]
struct CreatureGradeText;

/// 改名输入框（TextInput id 33）
#[derive(Component)]
struct CreatureRenameInput;

/// 改名确认
#[derive(Component)]
struct CreatureRenameOk;

/// 释放验证输入框（TextInput id 34）
#[derive(Component)]
struct CreatureReleaseInput;

/// 释放确认
#[derive(Component)]
struct CreatureReleaseOk;

#[derive(Component)]
pub struct CreatureLine(usize);

#[derive(Component)]
struct CreatureSummary;

#[derive(Component)]
struct CreatureMessage;

/// C# `CreatureInfo`/`CreatureInfo1`/`CreatureInfo2` 三行（@(19,161)/(19,176)/(19,191)）：
/// `0`=拾取能力（`CanPickupItems`）、`1`=产黑石、`2`=产珍珠买召唤兽物品。
#[derive(Component)]
struct CreatureInfoLine(u8);

/// C# `FullnessBG`（`Prguse2[530]`，@185,129；常显）
#[derive(Component)]
struct CreatureFullnessBg;

/// C# `FullnessFG`（`Prguse2[531]`；按 `Fullness/10000` 裁切，仅选中时可见）
#[derive(Component)]
struct CreatureFullnessFg;

/// C# `FullnessMin`/`FullnessNow` 刻度（`0`=Min[532] 在 (179,118)、`1`=Now[533] 在 (179,143)）
#[derive(Component)]
struct CreatureBarMarker(u8);

/// C# `BlackStoneImageBG`（`Prguse2[428]`，@215,348；常显）
#[derive(Component)]
struct CreatureBlackStoneBg;

/// C# `BlackStoneImageFG`（`Prguse2[420]`，@242,353；按 `BlackstoneTime/10800` 裁切）
#[derive(Component)]
struct CreatureBlackStoneFg;

/// C# `HoverLabel`（完整度条刻度/条身、黑石条三处的悬停提示）
#[derive(Component)]
struct CreatureHover;

/// C# `CreatureButton.PetButton` 图标（`Prguse2[pet.Icon]` 36x32 @44+81i, 259/299）；
/// `loaded` 记住已加载的图标索引（0 = 无），避免每帧重建 Image 资产
#[derive(Component)]
struct CreatureSlotIcon {
    slot: usize,
    loaded: u16,
}

/// C# `CreatureButton.SelectionImage`（`Prguse2[535]` 40x34 @-2,-2）
#[derive(Component)]
struct CreatureSlotSelection(usize);

/// C# `CreatureName`（@170,50 166x21 居中）
#[derive(Component)]
struct CreatureNameLabel;

/// C# `CreatureDeadline`（@140,85 350x21）
#[derive(Component)]
struct CreatureDeadlineLabel;

/// C# `CreaturePearls`（@53,348，显示玩家珍珠数）
#[derive(Component)]
struct CreaturePearlsLabel;

/// C# `CreatureImage` 面板宠物动画（@50,110，帧表见 `creature_anim_frames`）
#[derive(Component)]
struct CreatureAnimImage {
    /// 已加载帧表对应的宠物类型（`0` = 未设置）
    loaded_type: u8,
    /// default / ex 两套帧句柄
    frames: (Vec<Handle<Image>>, Vec<Handle<Image>>),
    /// 当前是否播放 ex 套（C# `AnimSwitched`）
    switched: bool,
    frame: usize,
    acc: f32,
    /// 面板打开后的累计秒数与「允许切换」的时间点（C# `SwitchAnimTime`，8 秒交替）
    elapsed: f32,
    switch_at: f32,
}

/// C# `SummonButton` 的两套帧：`Title[576..578]`（可召唤）与 `Title[593..595]`
/// （「已召唤其它种类」时的禁用态，`:651-653`）
#[derive(Component)]
struct CreatureSummonFrames {
    base: (Handle<Image>, Handle<Image>, Handle<Image>),
    alt: (Handle<Image>, Handle<Image>, Handle<Image>),
    current_alt: bool,
}

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
            (
                creature_ui_system,
                creature_action_system,
                creature_options_system,
                creature_bars_system,
                creature_slots_system,
                creature_labels_system,
                creature_anim_system,
            )
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

const CREATURE_W: f32 = 452.0;
const CREATURE_H: f32 = 376.0;
/// #2892 批B：面板精灵（C# `IntelligentCreatureDialog.Index = 468; Library = Libraries.Title`）
pub const PANEL: (LibraryName, usize) = (LibraryName::Title, 468);
pub const PANEL_SIZE: (f32, f32) = (CREATURE_W, CREATURE_H);
const CREATURE_SLOT_X0: f32 = 44.0;
const CREATURE_SLOT_Y0: f32 = 259.0;
const CREATURE_SLOT_DX: f32 = 81.0;
const CREATURE_SLOT_DY: f32 = 40.0;
const CREATURE_SLOT_W: f32 = 76.0;
const CREATURE_SLOT_H: f32 = 32.0;

// ---- #2761 完整度条 / 黑石条（C# `IntelligentCreatureDialogs.cs:179-258 / 347-401`）----
/// C# `FullnessBG`/`FullnessFG`（`Prguse2[530]/[531]`）@(185,129)，原生 248x12
const CREATURE_FULLNESS_X: f32 = 185.0;
const CREATURE_FULLNESS_Y: f32 = 129.0;
const CREATURE_FULLNESS_W: f32 = 248.0;
const CREATURE_FULLNESS_H: f32 = 12.0;
/// C# `FullnessMin`（`Prguse2[532]`）16x24、`FullnessNow`（`Prguse2[533]`）16x9 的基准坐标
const CREATURE_MARKER_MIN_Y: f32 = 118.0;
const CREATURE_MARKER_NOW_Y: f32 = 143.0;
const CREATURE_MARKER_W: f32 = 16.0;
/// 刻度精灵的 x 偏移：C# `FG.X + 段宽 - 8`（16 宽精灵以其中心对齐段落右端）
const CREATURE_MARKER_OFFSET: f32 = 8.0;
/// C# `BlackStoneImageBG`（`Prguse2[428]`）@(215,348) 204x17 / `FG`（`Prguse2[420]`）@(242,353) 172x7
const CREATURE_BLACKSTONE_X: f32 = 215.0;
const CREATURE_BLACKSTONE_Y: f32 = 348.0;
const CREATURE_BLACKSTONE_FG_X: f32 = 242.0;
const CREATURE_BLACKSTONE_FG_Y: f32 = 353.0;
const CREATURE_BLACKSTONE_FG_W: f32 = 172.0;
const CREATURE_BLACKSTONE_FG_H: f32 = 7.0;
/// C# `IntelligentCreatureDialogs.blackstoneProduceTime = 10800`（3 小时，秒）
const BLACKSTONE_PRODUCE_TIME: f32 = 10800.0;
/// C# `CreatureName`（@170,50，166x21 居中）
const CREATURE_NAME_X: f32 = 170.0;
const CREATURE_NAME_Y: f32 = 50.0;
const CREATURE_NAME_W: f32 = 166.0;
/// C# `CreatureButton.NameLabel`（80x15，位于 `PetButton` 的 (-22,-12)）
const CREATURE_NAME_LABEL_W: f32 = 80.0;

fn creature_slot_rect(index: usize, ox: f32, oy: f32) -> (f32, f32, f32, f32) {
    let col = (index % 5) as f32;
    let row = (index / 5) as f32;
    (
        ox + CREATURE_SLOT_X0 + col * CREATURE_SLOT_DX,
        oy + CREATURE_SLOT_Y0 + row * CREATURE_SLOT_DY,
        CREATURE_SLOT_W,
        CREATURE_SLOT_H,
    )
}

/// C# `IntelligentCreatureDialogs.cs` 操作按钮：(marker, x, y, Title 首帧索引, 宽, 高)。
/// 尺寸取自 `Title.Lib`（Index/HoverIndex/PressedIndex 连续 3 帧）。
/// Summon/Dismiss 与 Automatic/SemiAuto 在 C# 中同坐标互斥显示。
const CREATURE_OP_BUTTONS: [(&str, f32, f32, usize, f32, f32); 7] = [
    ("rename", 344.0, 50.0, 570, 92.0, 25.0),
    ("dismiss", 113.0, 217.0, 580, 80.0, 25.0),
    ("summon", 113.0, 217.0, 576, 80.0, 25.0),
    ("release", 255.0, 217.0, 583, 80.0, 25.0),
    ("opts", 375.0, 160.0, 573, 60.0, 25.0),
    ("auto", 375.0, 187.0, 610, 60.0, 25.0),
    ("semi", 375.0, 187.0, 613, 60.0, 25.0),
];

/// 操作按钮种类（C# `RefreshUI`/`RefreshMode` 的 `Enabled` 开关）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CreatureOp {
    Rename,
    Dismiss,
    Summon,
    Release,
    Options,
    Auto,
    Semi,
}

/// C# `RefreshUI()`（IntelligentCreatureDialogs.cs:606-668）的 `Enabled` 语义：
/// 未选中宠物 → 全部 `Enabled = false`（但 **`Visible` 不变**，即按钮仍在、只是灰掉）；
/// 选中后 改名/选项/自动/半自动可用；召唤仅在未激活时可用；解散仅在该宠已激活（召唤中）时可用；
/// 释放仅在该宠未召唤时可用（C# :647 `ReleaseButton.Enabled = false`）。
pub fn creature_op_enabled(
    op: CreatureOp,
    has_selection: bool,
    is_active: bool,
    other_summoned: bool,
) -> bool {
    match op {
        CreatureOp::Dismiss => has_selection && is_active,
        // C# :649-656：已召唤其它种类时 `SummonButton.Enabled = false`（该键此时显示 593..595 帧）
        CreatureOp::Summon => has_selection && !is_active && !other_summoned,
        CreatureOp::Release => has_selection && !is_active,
        _ => has_selection,
    }
}

/// C# `RefreshMode()`：Automatic 模式显示「自动」按钮，其余（含非 0 值）显示「半自动」；
/// 未选中宠物时该方法**早返回**，两个按钮保持构造时的 `Visible = true`（都可见、都禁用，
/// 同坐标 (375,187) → 后建的 SemiAuto 覆盖在上面，与原版绘制顺序一致）。
fn creature_mode_buttons_visible(has_selection: bool, pickup_mode: u8) -> (bool, bool) {
    if !has_selection {
        (true, true)
    } else {
        (pickup_mode == 0, pickup_mode != 0)
    }
}

/// 完整度条比例（C# `FullnessForeGround_AfterDraw`：`percent = Fullness / 10000`，>1 钳 1）。
fn fullness_percent(fullness: i32) -> f32 {
    (fullness as f32 / 10000.0).clamp(0.0, 1.0)
}

/// 黑石产出条比例（C# `BlackStoneImageFG_AfterDraw`：`BlackstoneTime / 10800`，>1 钳 1）。
fn blackstone_percent(blackstone_time: i32) -> f32 {
    (blackstone_time.max(0) as f32 / BLACKSTONE_PRODUCE_TIME).clamp(0.0, 1.0)
}

/// 条段落宽度（C# `(int)(Size.Width * percent)`：截断取整，不是四舍五入）。
fn bar_section(width: f32, percent: f32) -> f32 {
    (width * percent).floor()
}

/// 刻度精灵左端（C# `FG.Location.X + 段宽 - 8`）。
fn marker_left(bar_x: f32, section: f32) -> f32 {
    bar_x + section - CREATURE_MARKER_OFFSET
}

// 时间格式移植统一在 `game::time_format`（#2767：技能栏 Hint 也要用同一份 C# `Functions`）
use crate::game::time_format::format_time_span;

/// C# `Control_MouseEnter`（:403-431）三处悬停的文案与 `HoverLabel` 尺寸/位置。
/// 返回 `(文案, 标签左上角 x, y, 标签宽, 标签高)`；未命中返回 `None`。
fn creature_hover_label(
    creature: &CreatureEntry,
    min_left: f32,
    cursor: (f32, f32),
) -> Option<(String, f32, f32, f32, f32)> {
    let (lx, ly) = cursor;
    let in_rect = |x: f32, y: f32, w: f32, h: f32| lx >= x && lx <= x + w && ly >= y && ly <= y + h;
    // 刻度命中优先（C# 中它是后建控件、压在同坐标的 FG 之上）
    if in_rect(min_left, CREATURE_MARKER_MIN_Y, CREATURE_MARKER_W, 24.0) {
        return Some((
            format!("需要 {}", creature.rules.minimal_fullness),
            min_left + CREATURE_MARKER_OFFSET - 75.0,
            CREATURE_FULLNESS_Y - 18.0,
            150.0,
            15.0,
        ));
    }
    if in_rect(
        CREATURE_FULLNESS_X,
        CREATURE_FULLNESS_Y,
        CREATURE_FULLNESS_W,
        CREATURE_FULLNESS_H,
    ) {
        return Some((
            format!("{} / 10000", creature.fullness),
            CREATURE_FULLNESS_X,
            CREATURE_FULLNESS_Y - 2.0,
            CREATURE_FULLNESS_W,
            CREATURE_FULLNESS_H,
        ));
    }
    if in_rect(CREATURE_BLACKSTONE_X, CREATURE_BLACKSTONE_Y, 204.0, 17.0) {
        return Some((
            format_time_span(BLACKSTONE_PRODUCE_TIME as f64 - creature.blackstone_time as f64),
            CREATURE_BLACKSTONE_X + 5.0,
            CREATURE_BLACKSTONE_Y - 2.0,
            204.0,
            17.0,
        ));
    }
    None
}

/// C# `CreatureName.Text = CustomName`（空名即空串）。
fn creature_name_text(selected: Option<&CreatureEntry>) -> String {
    selected.map(|c| c.name.clone()).unwrap_or_default()
}

/// C# `CreatureDeadline` 文案（`:738-747`）：`Expire == DateTime.MinValue` → `过期: 永不过期`，
/// 否则 `过期: {PrintTimeSpanFromSeconds(剩余秒)}`；未选中宠物为空串。
fn creature_deadline_text(selected: Option<&CreatureEntry>) -> String {
    match selected {
        Some(c) if c.expire_secs > 0 => {
            format!("过期: {}", format_time_span(c.expire_secs as f64))
        }
        Some(_) => "过期: 永不过期".to_string(),
        None => String::new(),
    }
}

/// 槽位图标索引（C# `PetButton.Index = pet.Icon`；无宠物或本端无对应图标 → 0 = 不绘制）。
fn creature_slot_icon_index(entry: Option<&CreatureEntry>) -> u16 {
    entry.map(|c| c.icon.max(0) as u16).unwrap_or(0)
}

/// C# `SetCreatureFrames()`（`IntelligentCreatureDialogs.cs:983-1126`）帧表：
/// `(默认起始索引, 默认帧数, 默认间隔 ms, ex 起始索引, ex 帧数, ex 间隔 ms)`。
/// 按名称对应 C# 的 `IntelligentCreatureType`；本端独有类型（Panda/Oma/Sheep/Gorilla/Custom）
/// 在 C# `switch` 里没有 case → 沿用 `CreatureButton` 构造默认值（540/6/400 + 550/5/400）。
fn creature_anim_frames(creature_type: u8) -> (usize, usize, f32, usize, usize, f32) {
    match creature_type {
        2 => (540, 6, 200.0, 550, 5, 300.0),  // BabyPig
        5 => (600, 6, 250.0, 610, 10, 200.0), // Kitten
        6 => (570, 4, 350.0, 580, 10, 200.0), // Chick
        4 => (630, 11, 200.0, 650, 7, 250.0), // BabySkeleton
        9 => (750, 6, 300.0, 760, 7, 250.0),  // BabyDragon
        0 => (539, 1, 0.0, 539, 1, 0.0),      // None：单帧占位
        _ => (540, 6, 400.0, 550, 5, 400.0),  // C# 构造默认（本端独有类型）
    }
}

/// 单步推进动画（C# `MirAnimatedControl` 帧推进 + `DrawCreatureAnimation`:776-790 的换套判定）：
/// 返回 `(switched, frame, acc_ms, switch_at)`。
fn anim_tick(
    switched: bool,
    frame: usize,
    acc: f32,
    count: usize,
    delay: f32,
    elapsed: f32,
    switch_at: f32,
    dt: f32,
) -> (bool, usize, f32, f32) {
    if count == 0 || delay <= 0.0 {
        return (switched, frame, acc, switch_at);
    }
    let mut switched = switched;
    let mut frame = frame;
    let mut acc = acc + dt * 1000.0;
    let mut switch_at = switch_at;
    while acc >= delay {
        acc -= delay;
        frame += 1;
        if frame >= count {
            frame = 0;
            // C#：动画播完 + 已过 8 秒 → 换套，并把下一次允许切换推到 +8 秒
            if elapsed >= switch_at {
                switched = !switched;
                switch_at = elapsed + 8.0;
            }
        }
    }
    (switched, frame, acc, switch_at)
}

/// 槽位文字宽度估算（对话框 12px 字体：CJK 按 12px、半角按 6px）。
fn creature_text_width(text: &str) -> f32 {
    text.chars()
        .map(|ch| if (ch as u32) > 0x2E7F { 12.0 } else { 6.0 })
        .sum()
}

/// C# `CreatureButton.NameLabel`（80x15 定宽居中，仅名字）等价物：Bevy 无宠物头像
/// 资源，用名字占位；列距 81px、列宽 76px，标签自 `sx + 4` 起，故按 72px 截断，
/// 避免相邻槽文字互相压叠、最右列越出面板被裁。
fn creature_slot_label(creature: &CreatureEntry, selected: bool) -> String {
    let name = if creature.name.is_empty() {
        format!("#{}", creature.creature_type)
    } else {
        creature.name.clone()
    };
    let mut out = if selected {
        String::from(">")
    } else {
        String::new()
    };
    for ch in name.chars() {
        let mut candidate = out.clone();
        candidate.push(ch);
        if creature_text_width(&candidate) > CREATURE_NAME_LABEL_W {
            break;
        }
        out = candidate;
    }
    out
}

/// Bevy 扩展行（C# 无对应控件）：宠物数量 + 选中宠物名/拾取模式/饥饿度，
/// 放在面板底部空档（5x2 宠物槽之下），不占用 C# 的 (19,161)/(19,176)/(19,191) 三行信息位。
fn creature_summary_text(count: usize, selected: Option<&CreatureEntry>) -> String {
    let mut text = format!("宠物: {} 个", count);
    if let Some(c) = selected {
        text.push_str(&format!(
            " ｜ {} {} 饥饿:{}",
            if c.name.is_empty() {
                format!("#{}", c.creature_type)
            } else {
                c.name.clone()
            },
            if c.pickup_mode == 0 {
                "自动"
            } else {
                "半自动"
            },
            c.hunger
        ));
    }
    text
}

/// C# `IntelligentCreatureDialogs.cs:729-730` 的两段拼接：`semi`、`mouse`。
///
/// 含原版三处怪癖，逐字复刻（`SemiAutoPickupEnabled=false` 时两段皆空）：
/// - `semi` 的「NxN」用的是 **`AutoPickupRange`**（不是 `SemiAutoPickupRange`）；
///   后缀依次是 `auto/`（仅 `AutoPickupEnabled`）、`semi-auto`、`, `（仅 `MousePickupEnabled`）。
/// - `mouse` 段只由 `SemiAutoPickupEnabled` 决定，**与 `MousePickupEnabled` 无关**——
///   只开 Semi 的宠物（C# `BabyPig`/`Kitten`，`MousePickupRange=0`）会渲染出 `0x0 mouse`。
/// - 于是未开 `MousePickupEnabled` 时两段直接相接（缺 `, ` 分隔符），如
///   `可以拾取物品（0x0 semi-auto0x0 mouse）。`。
fn creature_pickup_parts(rules: &IntelligentCreatureRules) -> (String, String) {
    if !rules.semi_auto_pickup_enabled {
        return (String::new(), String::new());
    }
    let auto_prefix = if rules.auto_pickup_enabled {
        "auto/"
    } else {
        ""
    };
    let separator = if rules.mouse_pickup_enabled { ", " } else { "" };
    let semi = format!(
        "{}x{} {}{}{}",
        rules.auto_pickup_range, rules.auto_pickup_range, auto_prefix, "semi-auto", separator,
    );
    let mouse = format!(
        "{}x{} mouse",
        rules.mouse_pickup_range, rules.mouse_pickup_range
    );
    (semi, mouse)
}

/// C# `CreatureInfo`(@19,161)/`CreatureInfo1`(@19,176)/`CreatureInfo2`(@19,191) 三行文案
/// （`IntelligentCreatureDialogs.cs:733-735`）。文案取自 `Client/Localization/Chinese.json`：
/// `CanPickupItems`（含 `{0}{1}` 两个占位）、`CanProduceBlackStones`、
/// `CanProducePearlsBuyCreatureItems`（后两行仅在 `CanProduceBlackStone` 时非空）。
/// 未选中宠物时三行皆空——C# `RefreshUI` 另置 `Visible=false`，Bevy 下空文本同样不绘制。
fn creature_info_texts(selected: Option<&CreatureEntry>) -> [String; 3] {
    let Some(c) = selected else {
        return [String::new(), String::new(), String::new()];
    };
    let (semi, mouse) = creature_pickup_parts(&c.rules);
    let blackstone = c.rules.can_produce_black_stone;
    [
        format!("可以拾取物品（{semi}{mouse}）。"),
        if blackstone {
            "可以产出黑石。".to_string()
        } else {
            String::new()
        },
        if blackstone {
            "可以产出珍珠，用于购买召唤兽物品。".to_string()
        } else {
            String::new()
        },
    ]
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

    // 面板 C# Title[468]（原生 452x376；此前用 Prguse[170] 244x207 拉伸到面板尺寸会变形）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 468) else {
        return;
    };
    let (px, py) = crate::game::dialogs::center_origin(CREATURE_W, CREATURE_H);
    let panel = spawn_panel(&mut commands, bg, px, py, CREATURE_W, CREATURE_H, 30);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::Creature), CreatureWidget));

    commands.entity(panel).with_children(|p| {
        // 关闭 C# CloseButton：Prguse2[360/361/362] @(Size.Width-25, 3)，精灵 24x21
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 427.0, 3.0, 24.0, 21.0, 10)
                .insert((CreatureClose, CloseButton));
        }
        // C# 三行信息（`CreatureInfo`/`CreatureInfo1`/`CreatureInfo2`，@19,161/176/191）
        for (i, y) in [(0u8, 161.0), (1, 176.0), (2, 191.0)] {
            spawn_label(p, &cjk, "", 19.0, y, 12.0, Color::WHITE, 9).insert(CreatureInfoLine(i));
        }
        // #2761 C# 完整度条：BG[530]/FG[531] @(185,129) 248x12，Min[532] 16x24 @(179,118)，
        // Now[533] 16x9 @(179,143)；FG/刻度仅「选中宠物」时可见（C# `BeforeAfterDraw`）
        if let Some(img) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 530) {
            spawn_image(
                p,
                img,
                CREATURE_FULLNESS_X,
                CREATURE_FULLNESS_Y,
                CREATURE_FULLNESS_W,
                CREATURE_FULLNESS_H,
                10,
            )
            .insert(CreatureFullnessBg);
        }
        if let Some(img) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 531) {
            spawn_image(
                p,
                img,
                CREATURE_FULLNESS_X,
                CREATURE_FULLNESS_Y,
                CREATURE_FULLNESS_W,
                CREATURE_FULLNESS_H,
                11,
            )
            .insert((CreatureFullnessFg, Visibility::Hidden));
        }
        if let Some(img) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 532) {
            spawn_image(
                p,
                img,
                CREATURE_FULLNESS_X - CREATURE_MARKER_OFFSET,
                CREATURE_MARKER_MIN_Y,
                CREATURE_MARKER_W,
                24.0,
                12,
            )
            .insert((CreatureBarMarker(0), Visibility::Hidden));
        }
        if let Some(img) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 533) {
            spawn_image(
                p,
                img,
                CREATURE_FULLNESS_X - CREATURE_MARKER_OFFSET,
                CREATURE_MARKER_NOW_Y,
                CREATURE_MARKER_W,
                9.0,
                12,
            )
            .insert((CreatureBarMarker(1), Visibility::Hidden));
        }
        // #2761 C# 黑石产出条：BG[428] @(215,348) 204x17（常显）+ FG[420] @(242,353) 172x7
        if let Some(img) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 428) {
            spawn_image(
                p,
                img,
                CREATURE_BLACKSTONE_X,
                CREATURE_BLACKSTONE_Y,
                204.0,
                17.0,
                10,
            )
            .insert(CreatureBlackStoneBg);
        }
        if let Some(img) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 420) {
            spawn_image(
                p,
                img,
                CREATURE_BLACKSTONE_FG_X,
                CREATURE_BLACKSTONE_FG_Y,
                CREATURE_BLACKSTONE_FG_W,
                CREATURE_BLACKSTONE_FG_H,
                11,
            )
            .insert(CreatureBlackStoneFg);
        }
        // C# `HoverLabel`：单标签、按被悬停控件定位（本批接完整度条与黑石条三处）
        spawn_label(p, &cjk, "", 0.0, 0.0, 12.0, Color::WHITE, 20).insert((
            CreatureHover,
            TextLayout::justify(Justify::Center),
            Visibility::Hidden,
        ));
        // #2761 C# `CreatureName`(170,50 166x21 居中) / `CreatureDeadline`(140,85 350x21)
        spawn_label_center(
            p,
            &cjk,
            "",
            CREATURE_NAME_X + CREATURE_NAME_W / 2.0,
            CREATURE_NAME_Y,
            CREATURE_NAME_W,
            12.0,
            Color::WHITE,
            9,
        )
        .insert(CreatureNameLabel);
        spawn_label(p, &cjk, "", 140.0, 85.0, 12.0, Color::WHITE, 9).insert(CreatureDeadlineLabel);
        // #2761 C# `PearlImage`(`Prguse2[427]` @29,348 144x17) + `CreaturePearls`(@53,348)
        if let Some(img) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 427) {
            spawn_image(p, img, 29.0, 348.0, 144.0, 17.0, 10);
        }
        spawn_label(p, &cjk, "0", 53.0, 348.0, 12.0, Color::WHITE, 11).insert(CreaturePearlsLabel);
        // #2761 C# `CreatureImage`（@50,110，`Prguse2` 帧动画，8 秒在默认/ex 两套间交替）
        spawn_image(
            p,
            images.add(crate::map_renderer::make_image(vec![0, 0, 0, 0], 1, 1)),
            50.0,
            110.0,
            72.0,
            68.0,
            8,
        )
        .insert(CreatureAnimImage {
            loaded_type: 0,
            frames: (Vec::new(), Vec::new()),
            switched: false,
            frame: 0,
            acc: 0.0,
            elapsed: 0.0,
            switch_at: 8.0,
        });
        // Bevy 扩展行（C# 无对应控件）：紧随 C# 三行信息之后的同间距第四行（191+15=206）放数量
        // 摘要；操作反馈放宠物槽底与面板底纹之间的空档（第二行图标底 331、黑石条顶 348）。
        // （#2761 起槽位名字标签移到 C# `NameLabel` 位置 @(sx-22, sy-12)，占用了原 243 行。）
        spawn_label(p, &cjk, "", 19.0, 206.0, 12.0, Color::WHITE, 9).insert(CreatureSummary);
        spawn_label(p, &cjk, "", 19.0, 333.0, 12.0, Color::WHITE, 9).insert(CreatureMessage);
        // C# CreatureButton 5x2 网格：x=44+81*col，y=259/299。
        for i in 0..10usize {
            let (sx, sy, _, _) = creature_slot_rect(i, 0.0, 0.0);
            // #2761 C# `PetButton` 图标（36x32，`Prguse2[pet.Icon]`）+ `SelectionImage`（40x34 @-2,-2）
            // 占位图（1x1 全透明）：有图标时由系统换成 `Prguse2[pet.Icon]`
            spawn_image(
                p,
                images.add(crate::map_renderer::make_image(vec![0, 0, 0, 0], 1, 1)),
                sx,
                sy,
                36.0,
                32.0,
                9,
            )
            .insert((CreatureSlotIcon { slot: i, loaded: 0 }, Visibility::Hidden));
            if let Some(img) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 535) {
                spawn_image(p, img, sx - 2.0, sy - 2.0, 40.0, 34.0, 12)
                    .insert((CreatureSlotSelection(i), Visibility::Hidden));
            }
            // C# `NameLabel`（@-22,-12 80x15 居中；C# 仅悬停显示，Bevy 常显作扩展）
            spawn_label_center(
                p,
                &cjk,
                "",
                sx + 18.0,
                sy - 12.0,
                80.0,
                12.0,
                Color::WHITE,
                12,
            )
            .insert(CreatureLine(i));
        }
        // Bevy 扩展：刷新按钮（C# 无此控件）放右侧操作列，不覆盖 5x2 宠物槽。
        // 用中文文本按钮而非通用 Title[206..208]（该精灵在原版是 MessageBox 的「YES」）。
        spawn_container(p, 375.0, 217.0, 70.0, 22.0, 10)
            .insert((
                Button,
                CreatureRefresh,
                BackgroundColor(Color::srgba(0.15, 0.15, 0.2, 0.85)),
            ))
            .with_children(|c| {
                spawn_label(c, &cjk, "刷新", 4.0, 3.0, 12.0, Color::WHITE, 11);
            });
        // C# 操作按钮：使用 Title 精灵（含中文贴图）与 C# 原生坐标/尺寸。
        for (marker, x, y, index, w, h) in CREATURE_OP_BUTTONS {
            let (Some(n), Some(hv), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, index),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, index + 1),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, index + 2),
            ) else {
                continue;
            };
            let mut cmds = spawn_icon_button(p, n.clone(), hv.clone(), pr.clone(), x, y, w, h, 10);
            match marker {
                "rename" => {
                    cmds.insert(CreatureRenameBtn);
                }
                "dismiss" => {
                    cmds.insert(CreatureDismissBtn);
                }
                "summon" => {
                    // C# `RefreshUI`:651-653 / 663-665：召唤键在「已召唤其它种类」时切到 593..595
                    let alt = (
                        load_lib_image(&mut libs, &mut images, LibraryName::Title, 593),
                        load_lib_image(&mut libs, &mut images, LibraryName::Title, 594),
                        load_lib_image(&mut libs, &mut images, LibraryName::Title, 595),
                    );
                    cmds.insert((CreatureSummonBtn, Visibility::Hidden));
                    if let (Some(a), Some(b), Some(c)) = alt {
                        cmds.insert(CreatureSummonFrames {
                            base: (n.clone(), hv.clone(), pr.clone()),
                            alt: (a, b, c),
                            current_alt: false,
                        });
                    }
                }
                "release" => {
                    cmds.insert(CreatureReleaseBtn);
                }
                "auto" => {
                    cmds.insert((CreatureAutoBtn, Visibility::Hidden));
                }
                "semi" => {
                    cmds.insert((CreatureSemiBtn, Visibility::Hidden));
                }
                _ => {
                    cmds.insert(CreatureOptionsBtn);
                }
            }
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
            .with_children(|c| {
                spawn_label(c, &font, "保存", 0.0, 5.0, 12.0, Color::WHITE, 11);
            });
        spawn_container(p, 80.0, 255.0, 44.0, 22.0, 10)
            .insert((
                Button,
                CreatureOptionsWidget,
                CreatureOptionsCancel,
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                Visibility::Hidden,
            ))
            .with_children(|c| {
                spawn_label(c, &font, "取消", 0.0, 5.0, 12.0, Color::WHITE, 11);
            });
        spawn_label(p, &cjk, "品质:全部", 20.0, 280.0, 12.0, Color::WHITE, 11).insert((
            CreatureOptionsWidget,
            CreatureGradeText,
            Visibility::Hidden,
        ));
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
            cmds.with_children(|c| {
                spawn_label(c, &font, text, 0.0, 4.0, 12.0, Color::WHITE, 11);
            });
        }
        // 改名/释放输入框（TextInput id 33/34，C# MirInputBox 语义）@(18,270) + 确认 @(145,270)
        spawn_creature_input(
            p,
            &mut images,
            &font,
            33,
            CreatureRenameInput,
            "确认改名",
            CreatureRenameOk,
        );
        spawn_creature_input(
            p,
            &mut images,
            &font,
            34,
            CreatureReleaseInput,
            "确认释放",
            CreatureReleaseOk,
        );
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
    spawn_container(parent, 130.0, 140.0, 180.0, 20.0, 10)
        .insert((
            input_comp,
            crate::game::dialogs::text_input::TextInputField(id),
            crate::game::dialogs::text_input::TextInputRect(416.0, 336.0, 180.0, 20.0),
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
    spawn_container(parent, 315.0, 140.0, 70.0, 20.0, 10)
        .insert((
            Button,
            ok_comp,
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            Visibility::Hidden,
        ))
        .with_children(|c| {
            spawn_label(c, font, ok_label, 0.0, 4.0, 12.0, Color::WHITE, 11);
        });
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
    mut lines: Query<
        (&mut Text, &CreatureLine),
        (
            Without<CreatureSummary>,
            Without<CreatureMessage>,
            Without<CreatureInfoLine>,
        ),
    >,
    mut summary: Query<
        &mut Text,
        (
            With<CreatureSummary>,
            Without<CreatureLine>,
            Without<CreatureMessage>,
            Without<CreatureInfoLine>,
        ),
    >,
    mut messages: Query<
        &mut Text,
        (
            With<CreatureMessage>,
            Without<CreatureLine>,
            Without<CreatureSummary>,
            Without<CreatureInfoLine>,
        ),
    >,
    // C# `CreatureInfo`/`CreatureInfo1`/`CreatureInfo2` 三行文案（#2757）
    mut info_lines: Query<
        (&mut Text, &CreatureInfoLine),
        (
            Without<CreatureLine>,
            Without<CreatureSummary>,
            Without<CreatureMessage>,
        ),
    >,
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
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
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
                    .map(|n| {
                        crate::ui::theme::node_origin(
                            n,
                            crate::game::dialogs::center_origin(CREATURE_W, CREATURE_H),
                        )
                    })
                    .unwrap_or(crate::game::dialogs::center_origin(CREATURE_W, CREATURE_H));
                for i in 0..10usize {
                    let (x, y, w, h) = creature_slot_rect(i, ox, oy);
                    if cursor.x >= x && cursor.x <= x + w && cursor.y >= y && cursor.y <= y + h {
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
    let selected = state.creatures.get(state.selected).cloned();
    for (mut text, line) in &mut lines {
        text.0 = match state.creatures.get(line.0) {
            Some(c) => creature_slot_label(c, state.selected == line.0),
            None => String::new(),
        };
    }
    if let Ok(mut text) = summary.single_mut() {
        text.0 = creature_summary_text(state.creatures.len(), selected.as_ref());
    }
    if let Ok(mut text) = messages.single_mut() {
        text.0 = state.message.clone();
    }
    // C# `CreatureInfo`/`CreatureInfo1`/`CreatureInfo2`（`DrawCreatureAnimation`:733-735）
    let info = creature_info_texts(selected.as_ref());
    for (mut text, line) in &mut info_lines {
        text.0 = info.get(line.0 as usize).cloned().unwrap_or_default();
    }
    for (e, inter) in &refresh_btn {
        if edge(e, inter, &mut prev_inter) {
            net.send_packet(&crate::network::CreatureRequestWire { request: true });
            state.message = "已请求刷新".to_string();
            tracing::info!("🐾 刷新宠物列表");
        }
    }
}

/// #2761：完整度条 / 黑石条 / 悬停提示（C# `FullnessForeGround_AfterDraw`:347-377、
/// `BlackStoneImageFG_AfterDraw`:378-401、`Control_MouseEnter`:403-431）。
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn creature_bars_system(
    mgr: Res<DialogManager>,
    state: Res<CreatureState>,
    windows: Query<&Window>,
    panel: Query<&Node, With<CreatureWidget>>,
    mut fullness_fg: Query<
        (&mut Node, &mut ImageNode, &mut Visibility),
        (
            With<CreatureFullnessFg>,
            Without<CreatureBlackStoneFg>,
            Without<CreatureBarMarker>,
            Without<CreatureHover>,
            // 与只读的 `panel: Query<&Node, With<CreatureWidget>>` 证明不相交（B0001）
            Without<CreatureWidget>,
        ),
    >,
    mut blackstone_fg: Query<
        (&mut Node, &mut ImageNode, &mut Visibility),
        (
            With<CreatureBlackStoneFg>,
            Without<CreatureFullnessFg>,
            Without<CreatureBarMarker>,
            Without<CreatureHover>,
            Without<CreatureWidget>,
        ),
    >,
    mut markers: Query<
        (&mut Node, &mut Visibility, &CreatureBarMarker),
        (
            Without<CreatureFullnessFg>,
            Without<CreatureBlackStoneFg>,
            Without<CreatureHover>,
            Without<CreatureWidget>,
        ),
    >,
    mut hover: Query<
        (&mut Node, &mut Text, &mut Visibility),
        (
            With<CreatureHover>,
            Without<CreatureFullnessFg>,
            Without<CreatureBlackStoneFg>,
            Without<CreatureBarMarker>,
            Without<CreatureWidget>,
        ),
    >,
) {
    if !mgr.is_open(DialogKind::Creature) {
        return;
    }
    let selected = state.creatures.get(state.selected);

    // C# `BeforeAfterDraw`：无选中 → FG/两个刻度隐藏（BG 常显）
    let visible = if selected.is_some() {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    let min_section = bar_section(
        CREATURE_FULLNESS_W,
        selected
            .map(|c| (c.rules.minimal_fullness as f32 / 10000.0).clamp(0.0, 1.0))
            .unwrap_or(0.0),
    );
    let fullness_section = bar_section(
        CREATURE_FULLNESS_W,
        selected
            .map(|c| fullness_percent(c.fullness))
            .unwrap_or(0.0),
    );
    let blackstone_section = bar_section(
        CREATURE_BLACKSTONE_FG_W,
        selected
            .map(|c| blackstone_percent(c.blackstone_time))
            .unwrap_or(0.0),
    );
    let min_left = marker_left(CREATURE_FULLNESS_X, min_section);

    if let Ok((mut node, mut image, mut vis)) = fullness_fg.single_mut() {
        node.width = Val::Px(fullness_section);
        image.rect = Some(Rect::new(0.0, 0.0, fullness_section, CREATURE_FULLNESS_H));
        *vis = visible;
    }
    if let Ok((mut node, mut image, mut vis)) = blackstone_fg.single_mut() {
        node.width = Val::Px(blackstone_section);
        image.rect = Some(Rect::new(
            0.0,
            0.0,
            blackstone_section,
            CREATURE_BLACKSTONE_FG_H,
        ));
        *vis = visible;
    }
    for (mut node, mut vis, marker) in &mut markers {
        *vis = visible;
        let x = if marker.0 == 0 {
            min_left
        } else if fullness_section <= 0.0 {
            // C#：`percent <= 0` 时 `FullnessNow` 复位到构造坐标 (179,143)
            CREATURE_FULLNESS_X - CREATURE_MARKER_OFFSET
        } else {
            marker_left(CREATURE_FULLNESS_X, fullness_section)
        };
        node.left = Val::Px(x);
    }

    // 悬停提示（C# `Control_MouseEnter`/`MouseLeave`）
    let cursor_local = windows
        .single()
        .ok()
        .and_then(|w| w.cursor_position())
        .map(|cur| {
            let (ox, oy) = panel
                .single()
                .map(|n| {
                    crate::ui::theme::node_origin(
                        n,
                        crate::game::dialogs::center_origin(CREATURE_W, CREATURE_H),
                    )
                })
                .unwrap_or(crate::game::dialogs::center_origin(CREATURE_W, CREATURE_H));
            (cur.x - ox, cur.y - oy)
        });
    let label = selected
        .zip(cursor_local)
        .and_then(|(c, cursor)| creature_hover_label(c, min_left, cursor));
    if let Ok((mut node, mut text, mut vis)) = hover.single_mut() {
        match label {
            Some((content, x, y, w, _h)) => {
                text.0 = content;
                node.left = Val::Px(x);
                node.top = Val::Px(y);
                node.width = Val::Px(w);
                *vis = Visibility::Visible;
            }
            None => {
                text.0.clear();
                *vis = Visibility::Hidden;
            }
        }
    }
}

/// #2761：槽位图标（`Prguse2[pet.Icon]`）+ 选中框（`Prguse2[535]`）刷新。
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn creature_slots_system(
    mgr: Res<DialogManager>,
    state: Res<CreatureState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut icons: Query<
        (&mut CreatureSlotIcon, &mut ImageNode, &mut Visibility),
        (Without<CreatureSlotSelection>, Without<CreatureHover>),
    >,
    mut selections: Query<
        (&CreatureSlotSelection, &mut Visibility),
        (Without<CreatureSlotIcon>, Without<CreatureHover>),
    >,
) {
    if !mgr.is_open(DialogKind::Creature) {
        return;
    }
    for (mut slot_icon, mut image, mut vis) in &mut icons {
        let entry = state.creatures.get(slot_icon.slot);
        // #2761：图标索引随列表下发（C# `IntelligentCreatureInfo.Icon`，0 = 无对应）
        let icon = creature_slot_icon_index(entry);
        if icon != slot_icon.loaded {
            slot_icon.loaded = icon;
            if icon > 0 {
                if let Some(handle) =
                    load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, icon as usize)
                {
                    image.image = handle;
                }
            }
        }
        *vis = if entry.is_some() && icon > 0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    // C# `SelectButton(bool)`：选中槽显示 `SelectionImage`
    for (slot, mut vis) in &mut selections {
        *vis = if state.creatures.get(slot.0).is_some() && state.selected == slot.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// #2761：面板宠物动画（C# `DrawCreatureAnimation`:754-790 + `SetCreatureFrames`:983-1126）。
///
/// C# 用 `MirAnimatedControl` 播当前动画套；每当动画播完且已过 8 秒，就在 default/ex 两套间切换
/// （`SwitchAnimTime = CMain.Time + 8000`）。本系统同语义：帧推进按 `SetCreatureFrames` 的 ms 间隔，
/// 播完一轮时若 `elapsed >= switch_at` 则切套并把 `switch_at` 推到 +8 秒。
fn creature_anim_system(
    mgr: Res<DialogManager>,
    state: Res<CreatureState>,
    time: Res<Time>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut anim: Query<(&mut CreatureAnimImage, &mut ImageNode, &mut Node)>,
) {
    if !mgr.is_open(DialogKind::Creature) {
        return;
    }
    let dt = time.delta_secs();
    let creature_type = state
        .creatures
        .get(state.selected)
        .map(|c| c.creature_type)
        .unwrap_or(0);
    for (mut anim, mut image, mut node) in &mut anim {
        anim.elapsed += dt;
        if anim.loaded_type != creature_type {
            // 换宠物：按 `SetCreatureFrames` 帧表重建两套帧句柄
            let (idx, count, _d0, ex_idx, ex_count, _d1) = creature_anim_frames(creature_type);
            let mut load = |start: usize, n: usize| {
                let mut out = Vec::with_capacity(n);
                for k in 0..n {
                    if let Some(h) =
                        load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, start + k)
                    {
                        out.push(h);
                    }
                }
                out
            };
            let frames = (load(idx, count), load(ex_idx, ex_count));
            if let Some(first) = frames.0.first() {
                image.image = first.clone();
            }
            if let Some(info) = libs.0.get_image(LibraryName::Prguse2, idx) {
                node.width = Val::Px(info.width.max(1) as f32);
                node.height = Val::Px(info.height.max(1) as f32);
            }
            anim.frames = frames;
            anim.loaded_type = creature_type;
            anim.switched = false;
            anim.frame = 0;
            anim.acc = 0.0;
            anim.switch_at = anim.elapsed + 8.0;
        }
        let (_, _, delay0, _, _, delay1) = creature_anim_frames(creature_type);
        let (count, delay) = if anim.switched {
            (anim.frames.1.len(), delay1)
        } else {
            (anim.frames.0.len(), delay0)
        };
        let (switched, frame, acc, switch_at) = anim_tick(
            anim.switched,
            anim.frame,
            anim.acc,
            count,
            delay,
            anim.elapsed,
            anim.switch_at,
            dt,
        );
        anim.switched = switched;
        anim.frame = frame;
        anim.acc = acc;
        anim.switch_at = switch_at;
        let set = if anim.switched {
            &anim.frames.1
        } else {
            &anim.frames.0
        };
        if let Some(handle) = set.get(anim.frame) {
            if image.image != *handle {
                image.image = handle.clone();
            }
        }
    }
}

/// #2761：C# `CreatureName`(170,50) / `CreatureDeadline`(140,85) / `CreaturePearls`(53,348) 文案
/// （`DrawCreatureAnimation`:732-746 与 `RefreshDialog`:569）。
fn creature_labels_system(
    mgr: Res<DialogManager>,
    state: Res<CreatureState>,
    mut name: Query<
        &mut Text,
        (
            With<CreatureNameLabel>,
            Without<CreatureDeadlineLabel>,
            Without<CreaturePearlsLabel>,
        ),
    >,
    mut deadline: Query<
        &mut Text,
        (
            With<CreatureDeadlineLabel>,
            Without<CreatureNameLabel>,
            Without<CreaturePearlsLabel>,
        ),
    >,
    mut pearls: Query<
        &mut Text,
        (
            With<CreaturePearlsLabel>,
            Without<CreatureNameLabel>,
            Without<CreatureDeadlineLabel>,
        ),
    >,
) {
    if !mgr.is_open(DialogKind::Creature) {
        return;
    }
    let selected = state.creatures.get(state.selected);
    if let Ok(mut text) = name.single_mut() {
        // C# `CreatureName.Text = CustomName`（空名即空）
        text.0 = creature_name_text(selected);
    }
    if let Ok(mut text) = deadline.single_mut() {
        // C# `CreatureDeadline`：`Expire == DateTime.MinValue` → `过期: 永不过期`，否则剩余时间
        text.0 = creature_deadline_text(selected);
    }
    if let Ok(mut text) = pearls.single_mut() {
        // C# `CreaturePearls.Text = User.PearlCount.ToString()`
        text.0 = state.pearl_count.to_string();
    }
}

/// 宠物操作（C# IntelligentCreatureDialog ButtonClick：改名/召唤/解散/释放/自动/半自动）
#[allow(clippy::too_many_arguments)]
fn creature_action_system(
    mut state: ResMut<CreatureState>,
    net: Res<NetConnection>,
    mut input: ResMut<TextInputState>,
    mut submit: MessageReader<TextInputSubmit>,
    mut buttons: Query<
        (
            Entity,
            &Interaction,
            &mut ImageNode,
            &mut ImageButton,
            Option<&mut CreatureSummonFrames>,
            Has<CreatureRenameBtn>,
            Has<CreatureDismissBtn>,
            Has<CreatureSummonBtn>,
            Has<CreatureReleaseBtn>,
            Has<CreatureAutoBtn>,
            Has<CreatureSemiBtn>,
            Has<CreatureOptionsBtn>,
            Has<CreatureRenameOk>,
            Has<CreatureReleaseOk>,
        ),
        // 循环体对每个匹配按钮无条件写 node.color=WHITE——裸查询会踩全 app 的
        // ImageButton（同 #2954 char_skill 的踩法）；SummonFrames 恒与 SummonBtn 同挂
        Or<(
            With<CreatureRenameBtn>,
            With<CreatureDismissBtn>,
            With<CreatureSummonBtn>,
            With<CreatureReleaseBtn>,
            With<CreatureAutoBtn>,
            With<CreatureSemiBtn>,
            With<CreatureOptionsBtn>,
            With<CreatureRenameOk>,
            With<CreatureReleaseOk>,
            With<CreatureSummonFrames>,
        )>,
    >,
    // #1299：Bevy B0001——两个 &mut Visibility Query 冲突，用 ParamSet 顺序访问（#1298 合并后启动 panic）
    mut vis: ParamSet<(
        Query<
            (
                &mut Visibility,
                Has<CreatureDismissBtn>,
                Has<CreatureSummonBtn>,
                Has<CreatureAutoBtn>,
                Has<CreatureSemiBtn>,
            ),
            // 裸查询匹配全 world 的 Visibility 实体（同 #2954 char_skill 的踩法）
            Or<(
                With<CreatureDismissBtn>,
                With<CreatureSummonBtn>,
                With<CreatureAutoBtn>,
                With<CreatureSemiBtn>,
            )>,
        >,
        Query<
            (
                &mut Visibility,
                Has<CreatureRenameInput>,
                Has<CreatureReleaseInput>,
                Has<CreatureRenameOk>,
                Has<CreatureReleaseOk>,
            ),
            Or<(
                With<CreatureRenameInput>,
                With<CreatureReleaseInput>,
                With<CreatureRenameOk>,
                With<CreatureReleaseOk>,
            )>,
        >,
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
    let sel_name = selected
        .as_ref()
        .map(|c| c.name.clone())
        .unwrap_or_default();
    // C# `RefreshUI`:640-657：已召唤**其它种类**宠物时召唤键禁用并换成 593..595 帧
    let other_summoned = state.summoned && creature_type != state.summoned_type;

    // 解散仅对激活宠物显示；召唤对未激活的选中宠物显示（C# Summon/Dismiss 同位置切换）
    let (auto_visible, semi_visible) = creature_mode_buttons_visible(selected.is_some(), pet_mode);
    for (mut vis, is_dismiss, is_summon, is_auto, is_semi) in &mut vis.p0() {
        if is_dismiss {
            *vis = if is_active {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        } else if is_summon {
            // C# `RefreshUI`：无选中时 `SummonButton.Enabled = false` 但 **Visible 不变**（灰化）；
            // 选中且该宠已召唤时由 Dismiss 顶替（两者同坐标）
            *vis = if !is_active {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        } else if is_auto {
            // C# RefreshMode：Automatic 模式只显示「自动」按钮
            *vis = if auto_visible {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        } else if is_semi {
            // 非 Automatic（含其它非 0 值）只显示「半自动」按钮，两者不叠加
            *vis = if semi_visible {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
    for (mut vis, is_ri, is_reli, is_rok, is_relok) in &mut vis.p1() {
        if is_ri {
            *vis = if state.rename_open {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
        if is_reli {
            *vis = if state.release_open {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
        if is_rok {
            *vis = if state.rename_open {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
        if is_relok {
            *vis = if state.release_open {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }

    let submits: Vec<usize> = submit.read().map(|s| s.0).collect();
    let mut rename_confirm = false;
    let mut release_confirm = false;
    for (
        e,
        inter,
        mut node,
        mut frame_button,
        summon_frames,
        is_rename,
        is_dismiss,
        is_summon,
        is_release,
        is_auto,
        is_semi,
        is_opts,
        is_rok,
        is_relok,
    ) in &mut buttons
    {
        // C# `RefreshUI`：未选中宠物时按钮保持可见但 `Enabled = false`（灰化且不响应点击）
        let op = if is_rename {
            Some(CreatureOp::Rename)
        } else if is_dismiss {
            Some(CreatureOp::Dismiss)
        } else if is_summon {
            Some(CreatureOp::Summon)
        } else if is_release {
            Some(CreatureOp::Release)
        } else if is_auto {
            Some(CreatureOp::Auto)
        } else if is_semi {
            Some(CreatureOp::Semi)
        } else if is_opts {
            Some(CreatureOp::Options)
        } else {
            None
        };
        let enabled = match op {
            Some(op) => creature_op_enabled(op, selected.is_some(), is_active, other_summoned),
            // 改名/释放确认键与其它按钮（关闭等）不参与 C# 的 `Enabled` 开关
            None => true,
        };
        // C# `RefreshUI`:640-657：已召唤「其它种类」时召唤键切到 `Title[593..595]`（禁用态帧）
        if is_summon {
            if let Some(mut frames) = summon_frames {
                if frames.current_alt != other_summoned {
                    let (n, hv, pr) = if other_summoned {
                        frames.alt.clone()
                    } else {
                        frames.base.clone()
                    };
                    frame_button.normal = n;
                    frame_button.hover = hv;
                    frame_button.pressed = pr;
                    frames.current_alt = other_summoned;
                }
            }
        }
        // C# `MirButton` 禁用时 `Index` 回落 `base.Index`（`DisabledIndex` 未设 = -1），且
        // `IntelligentCreatureDialog` 从未设置 `GrayScale` → **禁用态外观与可用态相同**，
        // 只是点击被 `MirControl` 的 `!Enabled` 拦掉（:831-885）。此前用 `ImageNode.color`
        // 暗化是自造视觉，按原版回退为原色。
        let want_color = Color::WHITE;
        if node.color != want_color {
            node.color = want_color;
        }
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        if !enabled {
            continue;
        }
        if is_rename {
            state.rename_open = true;
            state.release_open = false;
            input.active = Some(33);
        } else if is_dismiss {
            net.send_packet(
                &mir2_shared::packets::client::misc::UpdateIntelligentCreature {
                    creature_type,
                    pet_mode,
                    custom_name: String::new(),
                    summon_me: false,
                    unsummon_me: true,
                    release_me: false,
                    filter: [0; 9],
                    grade: 0,
                    options_save: false,
                },
            );
            state.message = "已解散宠物".to_string();
        } else if is_summon {
            net.send_packet(
                &mir2_shared::packets::client::misc::UpdateIntelligentCreature {
                    creature_type,
                    pet_mode,
                    custom_name: String::new(),
                    summon_me: true,
                    unsummon_me: false,
                    release_me: false,
                    filter: [0; 9],
                    grade: 0,
                    options_save: false,
                },
            );
            state.message = format!("已召唤宠物 {}", sel_name);
        } else if is_release {
            state.release_open = true;
            state.rename_open = false;
            input.active = Some(34);
        } else if is_auto {
            net.send_packet(
                &mir2_shared::packets::client::misc::UpdateIntelligentCreature {
                    creature_type,
                    pet_mode: 0,
                    custom_name: String::new(),
                    summon_me: false,
                    unsummon_me: false,
                    release_me: false,
                    filter: [0; 9],
                    grade: 0,
                    options_save: false,
                },
            );
            state.message = "切换到自动模式".to_string();
        } else if is_semi {
            net.send_packet(
                &mir2_shared::packets::client::misc::UpdateIntelligentCreature {
                    creature_type,
                    pet_mode: 1,
                    custom_name: String::new(),
                    summon_me: false,
                    unsummon_me: false,
                    release_me: false,
                    filter: [0; 9],
                    grade: 0,
                    options_save: false,
                },
            );
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
            net.send_packet(
                &mir2_shared::packets::client::misc::UpdateIntelligentCreature {
                    creature_type,
                    pet_mode,
                    custom_name: name.clone(),
                    summon_me: false,
                    unsummon_me: false,
                    release_me: false,
                    filter: [0; 9],
                    grade: 0,
                    options_save: false,
                },
            );
            state.message = format!("已改名为 {}", name);
        }
        state.rename_open = false;
        if input.texts.len() > 33 {
            input.texts[33].clear();
        }
    }
    // 释放确认（C# ReleaseButton → 输入宠物名验证 → ReleaseMe）
    if release_confirm && state.release_open {
        let name = input.texts.get(34).cloned().unwrap_or_default();
        let name = name.trim().to_string();
        if name.eq_ignore_ascii_case(&sel_name) {
            net.send_packet(
                &mir2_shared::packets::client::misc::UpdateIntelligentCreature {
                    creature_type,
                    pet_mode,
                    custom_name: String::new(),
                    summon_me: false,
                    unsummon_me: false,
                    release_me: true,
                    filter: [0; 9],
                    grade: 0,
                    options_save: false,
                },
            );
            state.message = "宠物已释放".to_string();
        } else {
            state.message = "验证失败：名字不匹配".to_string();
        }
        state.release_open = false;
        if input.texts.len() > 34 {
            input.texts[34].clear();
        }
    }
}
/// 选项标签（C# IntelligentCreatureOptionsDialog.OptionNames）
const CREATURE_OPTION_LABELS: [&str; 9] = [
    "全部", "金币", "武器", "盔甲", "头盔", "靴子", "腰带", "饰品", "其他",
];
/// 品质标签（C# OptionsGradeDialog GradeStrings：全部/普通/稀有/神话/传说/英雄）
const CREATURE_GRADE_LABELS: [&str; 6] = ["全部", "普通", "稀有", "神话", "传说", "英雄"];

/// 品质循环（dir>0 下一档，否则上一档；0..5，对齐 C# Prev/Next）
fn creature_grade_cycle(grade: u8, dir: i8) -> u8 {
    if dir > 0 {
        (grade + 1) % 6
    } else {
        (grade + 5) % 6
    }
}

/// 过滤切换（对齐 C# IntelligentCreatureItemFilter.SetItemFilter）
fn creature_filter_toggle(f: &mut [bool; 9], idx: usize) {
    match idx {
        0 => {
            f[0] = true;
            for i in 1..9 {
                f[i] = false;
            }
        }
        1..=8 => {
            f[0] = false;
            f[idx] = !f[idx];
        }
        _ => {}
    }
    if (1..9).all(|i| f[i]) {
        f[0] = true;
        for i in 1..9 {
            f[i] = false;
        }
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
    grade_btns: Query<(
        Entity,
        &Interaction,
        Has<CreatureGradePrev>,
        Has<CreatureGradeNext>,
    )>,
    panel_origin: Query<&Node, With<CreatureWidget>>,
    mut lines: Query<
        (
            &mut Text,
            Option<&CreatureOptionsLine>,
            Has<CreatureGradeText>,
        ),
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
                    for i in 0..9 {
                        state.options[i] = f[i] != 0;
                    }
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
                    .map(|n| {
                        crate::ui::theme::node_origin(
                            n,
                            crate::game::dialogs::center_origin(CREATURE_W, CREATURE_H),
                        )
                    })
                    .unwrap_or(crate::game::dialogs::center_origin(CREATURE_W, CREATURE_H));
                for i in 0..9usize {
                    let y = oy + 40.0 + i as f32 * 22.0;
                    if cursor.x >= ox + 20.0
                        && cursor.x <= ox + 220.0
                        && cursor.y >= y
                        && cursor.y <= y + 20.0
                    {
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
            for i in 0..9 {
                filter[i] = if state.options[i] { 1 } else { 0 };
            }
            let selected = state.creatures.get(state.selected).cloned();
            let creature_type = selected.as_ref().map(|c| c.creature_type).unwrap_or(0);
            let pet_mode = selected.as_ref().map(|c| c.pickup_mode).unwrap_or(0);
            net.send_packet(
                &mir2_shared::packets::client::misc::UpdateIntelligentCreature {
                    creature_type,
                    pet_mode,
                    custom_name: String::new(),
                    summon_me: false,
                    unsummon_me: false,
                    release_me: false,
                    filter,
                    grade: state.grade,
                    options_save: true,
                },
            );
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
            format!(
                "{} {}",
                if state.options[l.0] { "■" } else { "□" },
                CREATURE_OPTION_LABELS[l.0]
            )
        } else if is_grade {
            format!(
                "品质:{}",
                CREATURE_GRADE_LABELS[(state.grade as usize).min(5)]
            )
        } else {
            String::new()
        };
    }
    for mut vis in widgets.iter_mut() {
        *vis = if state.options_open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for mut vis in line_vis.iter_mut() {
        *vis = if state.options_open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for mut vis in save_vis.iter_mut() {
        *vis = if state.options_open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for mut vis in cancel_vis.iter_mut() {
        *vis = if state.options_open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// 消费服务端宠物列表事件（网络层只广播 ServerEvent）
fn creature_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut creature: ResMut<CreatureState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::CreatureList {
            creatures,
            summoned,
            summoned_type,
            pearl_count,
        } = ev
        {
            creature.creatures = creatures.clone();
            // #2761：C# `UpdateIntelligentCreatureList` 尾部三字段
            creature.summoned = *summoned;
            creature.summoned_type = *summoned_type;
            creature.pearl_count = *pearl_count;
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
    /// 表征（#2954 同类防护）：creature_action_system 的 buttons/vis 查询限定
    /// 宠物部件后，无标记按钮的 `node.color` 与 Visibility 均不得被触碰；
    /// CreatureRenameInput 显隐仍跟随 rename_open。
    #[test]
    fn creature_action_system_does_not_stomp_unrelated_widgets() {
        use bevy::prelude::*;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<super::CreatureState>();
        app.insert_resource(crate::network::NetConnection::default());
        app.init_resource::<crate::game::dialogs::text_input::TextInputState>();
        app.add_message::<crate::game::dialogs::text_input::TextInputSubmit>();
        app.add_systems(Update, super::creature_action_system);

        // 无标记按钮：非白 color + Visible——两个维度都不得被踩
        let tint = Color::srgb(0.5, 0.5, 0.5);
        let mut decoy_node = ImageNode::default();
        decoy_node.color = tint;
        let decoy = app
            .world_mut()
            .spawn((
                Button,
                Interaction::None,
                Node::default(),
                decoy_node,
                crate::ui::theme::ImageButton {
                    normal: Handle::default(),
                    hover: Handle::default(),
                    pressed: Handle::default(),
                },
                Visibility::Visible,
            ))
            .id();
        let rename_input = app
            .world_mut()
            .spawn((Visibility::Hidden, super::CreatureRenameInput))
            .id();

        app.world_mut()
            .resource_mut::<super::CreatureState>()
            .rename_open = true;
        app.update();
        assert_eq!(
            app.world().entity(decoy).get::<Visibility>().unwrap(),
            &Visibility::Visible,
            "无标记按钮 Visibility 不得被 creature_action_system 触碰"
        );
        assert_eq!(
            app.world().entity(decoy).get::<ImageNode>().unwrap().color,
            tint,
            "无标记按钮 node.color 不得被改写成 WHITE"
        );
        assert_eq!(
            app.world()
                .entity(rename_input)
                .get::<Visibility>()
                .unwrap(),
            &Visibility::Visible,
            "rename_open=true → CreatureRenameInput Visible"
        );
    }

    use super::*;

    /// #2736：C# `RefreshUI()` 的 `Enabled` 语义——未选中宠物时全部按钮禁用（但保持可见），
    /// 选中后按召唤状态区分（C# IntelligentCreatureDialogs.cs:606-668）
    #[test]
    fn creature_op_enabled_matches_csharp() {
        use CreatureOp::*;
        // 未选中：全部禁用
        for op in [Rename, Dismiss, Summon, Release, Options, Auto, Semi] {
            assert!(
                !creature_op_enabled(op, false, false, false),
                "{op:?} 未选中宠物时应禁用"
            );
        }
        // 选中且未召唤：改名/选项/自动/半自动/召唤/释放可用，解散禁用
        for op in [Rename, Options, Auto, Semi, Summon, Release] {
            assert!(
                creature_op_enabled(op, true, false, false),
                "{op:?} 选中未召唤时应可用"
            );
        }
        assert!(!creature_op_enabled(Dismiss, true, false, false));
        // 选中且已召唤（激活）：解散可用；召唤/释放禁用（C# :647 ReleaseButton.Enabled = false）
        assert!(creature_op_enabled(Dismiss, true, true, false));
        assert!(!creature_op_enabled(Summon, true, true, false));
        assert!(!creature_op_enabled(Release, true, true, false));
        assert!(creature_op_enabled(Rename, true, true, false));
        assert!(creature_op_enabled(Options, true, true, false));
    }

    /// #2736：C# `RefreshMode()` 早返回 → 未选中宠物时两个模式按钮都保持可见（都禁用）
    #[test]
    fn creature_mode_buttons_keep_visible_without_selection() {
        assert_eq!(creature_mode_buttons_visible(false, 0), (true, true));
        assert_eq!(creature_mode_buttons_visible(false, 1), (true, true));
        // 选中后按模式二选一（C# `RefreshMode` 的 Visible 切换）
        assert_eq!(creature_mode_buttons_visible(true, 0), (true, false));
        assert_eq!(creature_mode_buttons_visible(true, 1), (false, true));
        assert_eq!(creature_mode_buttons_visible(true, 2), (false, true));
    }

    #[test]
    fn filter_toggle_all() {
        let mut f = [false; 9];
        creature_filter_toggle(&mut f, 0);
        assert!(f[0]);
        for i in 1..9 {
            assert!(!f[i]);
        }
    }

    #[test]
    fn filter_toggle_categories_and_auto_all() {
        let mut f = [false; 9];
        creature_filter_toggle(&mut f, 1);
        assert!(!f[0] && f[1]);
        for i in 2..9 {
            creature_filter_toggle(&mut f, i);
        }
        // 8 类全开 → 自动回退为「全部」
        assert!(f[0]);
        for i in 1..9 {
            assert!(!f[i]);
        }
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
    #[test]
    fn creature_origin_is_csharp_center() {
        assert_eq!(
            crate::game::dialogs::center_origin(CREATURE_W, CREATURE_H),
            (286.0, 196.0)
        );
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;

    #[test]
    fn creature_slots_match_csharp_grid() {
        assert_eq!(creature_slot_rect(0, 0.0, 0.0), (44.0, 259.0, 76.0, 32.0));
        assert_eq!(creature_slot_rect(4, 0.0, 0.0), (368.0, 259.0, 76.0, 32.0));
        assert_eq!(creature_slot_rect(5, 0.0, 0.0), (44.0, 299.0, 76.0, 32.0));
        assert_eq!(creature_slot_rect(9, 0.0, 0.0), (368.0, 299.0, 76.0, 32.0));
    }

    #[test]
    fn creature_slots_offset_with_panel_origin() {
        // C# Center 原点 (286,196) + 槽位 5（第二行首列）
        assert_eq!(
            creature_slot_rect(5, 286.0, 196.0),
            (330.0, 495.0, 76.0, 32.0)
        );
    }

    /// C# 锚点（IntelligentCreatureDialogs.cs）：按钮坐标与 Title 精灵尺寸。
    #[test]
    fn creature_op_buttons_match_csharp_anchors() {
        let spec = |m: &str| {
            CREATURE_OP_BUTTONS
                .iter()
                .find(|b| b.0 == m)
                .map(|b| (b.1, b.2, b.3, b.4, b.5))
                .expect("marker 必须存在")
        };
        assert_eq!(spec("rename"), (344.0, 50.0, 570, 92.0, 25.0));
        assert_eq!(spec("dismiss"), (113.0, 217.0, 580, 80.0, 25.0));
        assert_eq!(spec("summon"), (113.0, 217.0, 576, 80.0, 25.0));
        assert_eq!(spec("release"), (255.0, 217.0, 583, 80.0, 25.0));
        assert_eq!(spec("opts"), (375.0, 160.0, 573, 60.0, 25.0));
        assert_eq!(spec("auto"), (375.0, 187.0, 610, 60.0, 25.0));
        assert_eq!(spec("semi"), (375.0, 187.0, 613, 60.0, 25.0));
        // Summon/Dismiss、Auto/SemiAuto 在 C# 中共用同一坐标，靠显隐互斥
        assert_eq!(spec("dismiss").0, spec("summon").0);
        assert_eq!(spec("auto").0, spec("semi").0);
        assert_eq!(spec("dismiss").1, spec("summon").1);
        assert_eq!(spec("auto").1, spec("semi").1);
    }

    /// 自动/半自动按钮在「选中宠物」时按模式互斥；未选中宠物时按 C# `RefreshMode`（早返回 +
    /// 构造默认 `Visible = true`）两者都保持可见、但都禁用（同坐标 → 后建的 SemiAuto 覆盖）。
    #[test]
    fn creature_mode_buttons_are_mutually_exclusive() {
        assert_eq!(creature_mode_buttons_visible(false, 0), (true, true));
        assert_eq!(creature_mode_buttons_visible(false, 1), (true, true));
        assert_eq!(creature_mode_buttons_visible(true, 0), (true, false));
        assert_eq!(creature_mode_buttons_visible(true, 1), (false, true));
        // C# 非 Automatic 一律按 SemiAuto 显示
        assert_eq!(creature_mode_buttons_visible(true, 7), (false, true));
        // 未选中时两者都禁用（`Enabled = false`），故不会误点
        assert!(!creature_op_enabled(CreatureOp::Auto, false, false, false));
        assert!(!creature_op_enabled(CreatureOp::Semi, false, false, false));
    }

    /// 槽位标签必须放得进 76px 列（自 sx+4 起，72px 内），否则相邻槽互相压叠。
    #[test]
    fn creature_slot_label_fits_column() {
        let mut creature = CreatureEntry::default();
        creature.name = "很长的宠物名字七个字".to_string();
        let label = creature_slot_label(&creature, true);
        assert!(label.starts_with('>'), "选中前缀保留：{label}");
        let width = creature_text_width(&label);
        // C# `CreatureButton.NameLabel` 80x15（@PetButton 的 -22,-12）
        assert!(width <= CREATURE_NAME_LABEL_W, "标签宽度 {width} 超出 80px");
        // 标签自 sx-22 起、宽 80 → 止于 sx+58，不越过下一槽图标起点 sx+81
        assert!(-22.0 + CREATURE_NAME_LABEL_W <= CREATURE_SLOT_DX);

        creature.name = "小狗".to_string();
        assert_eq!(creature_slot_label(&creature, false), "小狗");

        creature.name.clear();
        creature.creature_type = 12;
        assert_eq!(creature_slot_label(&creature, false), "#12");
    }

    /// Bevy 扩展行：选中宠物的模式/饥饿度摘要（不占用 C# 三行信息位）。
    #[test]
    fn creature_summary_reports_selected_pet() {
        assert_eq!(creature_summary_text(0, None), "宠物: 0 个");
        let mut creature = CreatureEntry::default();
        creature.name = "小狗".to_string();
        creature.pickup_mode = 1;
        creature.hunger = 42;
        assert_eq!(
            creature_summary_text(1, Some(&creature)),
            "宠物: 1 个 ｜ 小狗 半自动 饥饿:42"
        );
    }

    fn entry_with_rules(rules: IntelligentCreatureRules) -> CreatureEntry {
        CreatureEntry {
            rules,
            ..Default::default()
        }
    }

    /// #2757：C# `CreatureInfo`（`CanPickupItems` 两个占位）逐字复刻——含 semi 串用
    /// `AutoPickupRange`、mouse 串只在 `SemiAutoPickupEnabled` 时产出的两处原版怪癖。
    #[test]
    fn creature_info_pickup_text_matches_csharp() {
        // C# `Chick` 行：Auto 7 + Mouse 11 → "7x7 auto/semi-auto, " + "11x11 mouse"
        let chick = entry_with_rules(IntelligentCreatureRules {
            mouse_pickup_enabled: true,
            mouse_pickup_range: 11,
            auto_pickup_enabled: true,
            auto_pickup_range: 7,
            semi_auto_pickup_enabled: true,
            semi_auto_pickup_range: 7,
            can_produce_black_stone: true,
            ..Default::default()
        });
        let info = creature_info_texts(Some(&chick));
        assert_eq!(info[0], "可以拾取物品（7x7 auto/semi-auto, 11x11 mouse）。");
        assert_eq!(info[1], "可以产出黑石。");
        assert_eq!(info[2], "可以产出珍珠，用于购买召唤兽物品。");

        // C# `BabyPig` 行：只开 Semi 3（Auto 关闭 → 无 `auto/`；Mouse 关闭 → 无 `, ` 分隔符，
        // 但 `mouse` 段仍按 `SemiAutoPickupEnabled` 产出 → `0x0 semi-auto0x0 mouse`）
        let pig = entry_with_rules(IntelligentCreatureRules {
            minimal_fullness: 4000,
            semi_auto_pickup_enabled: true,
            semi_auto_pickup_range: 3,
            ..Default::default()
        });
        let info = creature_info_texts(Some(&pig));
        assert_eq!(info[0], "可以拾取物品（0x0 semi-auto0x0 mouse）。");
        assert_eq!(info[1], "");
        assert_eq!(info[2], "");
    }

    /// #2757：`SemiAutoPickupEnabled=false`（C# 表中无对应的本端独有类型）→ semi/mouse 两段皆空，
    /// 模板仍渲染「可以拾取物品（）。」；未选中宠物时三行全空。
    #[test]
    fn creature_info_empty_cases() {
        let none = entry_with_rules(IntelligentCreatureRules {
            minimal_fullness: 1000,
            ..Default::default()
        });
        let info = creature_info_texts(Some(&none));
        assert_eq!(info[0], "可以拾取物品（）。");
        assert_eq!((info[1].as_str(), info[2].as_str()), ("", ""));

        let no_selection = creature_info_texts(None);
        assert_eq!(no_selection, [String::new(), String::new(), String::new()]);
    }

    /// #2757：mouse 段仅在置位时出现（`BabyDragon` 行 Mouse 7 / Auto 5 / Semi 5）。
    #[test]
    fn creature_info_mouse_range_uses_its_own_range() {
        let dragon = entry_with_rules(IntelligentCreatureRules {
            minimal_fullness: 7000,
            mouse_pickup_enabled: true,
            mouse_pickup_range: 7,
            auto_pickup_enabled: true,
            auto_pickup_range: 5,
            semi_auto_pickup_enabled: true,
            semi_auto_pickup_range: 5,
            ..Default::default()
        });
        assert_eq!(
            creature_info_texts(Some(&dragon))[0],
            "可以拾取物品（5x5 auto/semi-auto, 7x7 mouse）。"
        );
    }

    /// #2761：完整度/黑石条比例与刻度定位按 C# 公式（`Fullness/10000`、`BlackstoneTime/10800`，
    /// 段落宽 `(int)(W*percent)` 截断取整，刻度 x = `FG.X + 段宽 - 8`）。
    #[test]
    fn creature_bars_match_csharp_percent_and_marker() {
        assert_eq!(fullness_percent(4000), 0.4);
        assert_eq!(fullness_percent(10000), 1.0);
        assert_eq!(fullness_percent(12345), 1.0); // >1 钳 1
        assert_eq!(fullness_percent(0), 0.0);
        assert_eq!(blackstone_percent(5400), 0.5);
        assert_eq!(blackstone_percent(10800), 1.0);

        // C# BabyPig：MinimalFullness 4000 → 段宽 99（248*0.4=99.2 截断），刻度 x = 185+99-8
        let section = bar_section(CREATURE_FULLNESS_W, 0.4);
        assert_eq!(section, 99.0);
        assert_eq!(marker_left(CREATURE_FULLNESS_X, section), 276.0);
        // C# 满值 → 整条
        assert_eq!(bar_section(CREATURE_FULLNESS_W, 1.0), 248.0);
        // C# `percent <= 0` 时 `FullnessNow` 复位到 (179,143)（= FG.X - 6）
        assert_eq!(CREATURE_FULLNESS_X - CREATURE_MARKER_OFFSET, 177.0);
    }

    /// #2761：悬停文案三处（C# `Control_MouseEnter`:403-431）——
    /// 刻度→`需要 {MinimalFullness}`、条身→`{Fullness} / 10000`、黑石条→剩余时间。
    #[test]
    fn creature_hover_label_matches_csharp() {
        let c = entry_with_rules(IntelligentCreatureRules {
            minimal_fullness: 4000,
            ..Default::default()
        });
        let mut c = c;
        c.fullness = 7500;
        c.blackstone_time = 3600;
        let min_left = marker_left(CREATURE_FULLNESS_X, 99.0);

        // 刻度命中（Min 精灵 16x24 @(179,118)）
        let (text, x, y, w, h) =
            creature_hover_label(&c, min_left, (min_left + 1.0, 130.0)).unwrap();
        assert_eq!(text, "需要 4000");
        assert_eq!((x, y, w, h), (276.0 + 8.0 - 75.0, 111.0, 150.0, 15.0));

        // 条身命中（@185,129 248x12；避开刻度 x 区间）
        let (text, x, y, w, h) = creature_hover_label(&c, min_left, (400.0, 135.0)).unwrap();
        assert_eq!(text, "7500 / 10000");
        assert_eq!((x, y, w, h), (185.0, 127.0, 248.0, 12.0));

        // 黑石条命中（BG @215,348 204x17）
        let (text, x, y, w, h) = creature_hover_label(&c, min_left, (300.0, 355.0)).unwrap();
        assert_eq!(text, "2h 00m 00s"); // 10800 - 3600 = 7200 秒
        assert_eq!((x, y, w, h), (220.0, 346.0, 204.0, 17.0));

        // 未命中
        assert!(creature_hover_label(&c, min_left, (10.0, 300.0)).is_none());
    }

    /// #2761：`CreatureName`/`CreatureDeadline`/槽位图标索引（C# `:732`/`:738-747`/`SetButtonInfo`）。
    #[test]
    fn creature_name_deadline_and_slot_icon_match_csharp() {
        let mut c = CreatureEntry::default();
        c.name = "小鸡".to_string();
        c.icon = 501;

        assert_eq!(creature_name_text(Some(&c)), "小鸡");
        assert_eq!(creature_name_text(None), "");
        assert_eq!(creature_slot_icon_index(Some(&c)), 501);
        assert_eq!(creature_slot_icon_index(None), 0);

        // 永久（0）→ `过期: 永不过期`；否则 `过期: {PrintTimeSpanFromSeconds}`
        assert_eq!(creature_deadline_text(Some(&c)), "过期: 永不过期");
        c.expire_secs = 604_800;
        assert_eq!(creature_deadline_text(Some(&c)), "过期: 7d 00h 00m 00s");
        c.expire_secs = 3661;
        assert_eq!(creature_deadline_text(Some(&c)), "过期: 1h 01m 01s");
        assert_eq!(creature_deadline_text(None), "");
    }

    /// #2761：动画帧表按 C# `SetCreatureFrames`（名称对应），本端独有类型取 `CreatureButton`
    /// 构造默认值（540/6/400 + 550/5/400）。
    #[test]
    fn creature_anim_frames_match_csharp() {
        assert_eq!(creature_anim_frames(2), (540, 6, 200.0, 550, 5, 300.0)); // BabyPig
        assert_eq!(creature_anim_frames(5), (600, 6, 250.0, 610, 10, 200.0)); // Kitten
        assert_eq!(creature_anim_frames(6), (570, 4, 350.0, 580, 10, 200.0)); // Chick
        assert_eq!(creature_anim_frames(4), (630, 11, 200.0, 650, 7, 250.0)); // BabySkeleton
        assert_eq!(creature_anim_frames(9), (750, 6, 300.0, 760, 7, 250.0)); // BabyDragon
        assert_eq!(creature_anim_frames(0), (539, 1, 0.0, 539, 1, 0.0)); // None
        for t in [1u8, 3, 7, 8, 100] {
            assert_eq!(
                creature_anim_frames(t),
                (540, 6, 400.0, 550, 5, 400.0),
                "类型 {t} 应取 C# 构造默认帧表"
            );
        }
    }

    /// #2761：帧推进 + 8 秒换套（C# `MirAnimatedControl` + `:776-790`）。
    #[test]
    fn creature_anim_tick_advances_and_switches() {
        // 4 帧 @350ms：350ms 走 1 帧，未到换套时间不切
        let (sw, f, acc, sw_at) = anim_tick(false, 0, 0.0, 4, 350.0, 1.0, 8.0, 0.35);
        assert_eq!((sw, f, sw_at), (false, 1, 8.0));
        assert!(acc.abs() < 1e-3);
        // 播完一轮（第 4 帧后回 0）且已过 switch 时间 → 换套并把 switch_at 推到 elapsed+8
        let (sw, f, _acc, sw_at) = anim_tick(false, 3, 0.0, 4, 350.0, 9.0, 8.0, 0.35);
        assert_eq!((sw, f), (true, 0));
        assert_eq!(sw_at, 17.0);
        // 播完但未到 switch 时间 → 不换套
        let (sw, f, _, sw_at) = anim_tick(false, 3, 0.0, 4, 350.0, 5.0, 8.0, 0.35);
        assert_eq!((sw, f, sw_at), (false, 0, 8.0));
        // 单帧/零间隔（C# `None` 分支）不推进
        let (sw, f, _, _) = anim_tick(false, 0, 0.0, 1, 0.0, 100.0, 8.0, 1.0);
        assert_eq!((sw, f), (false, 0));
    }

    /// #2761：`CreatureSummonBtn` 在「已召唤其它种类」时禁用（C# `:649-656`）。
    #[test]
    fn creature_summon_disabled_when_other_type_summoned() {
        use CreatureOp::*;
        // 其它种类召唤中：召唤禁用（该键显示 593..595 帧），解散仍按本宠是否召唤
        assert!(!creature_op_enabled(Summon, true, false, true));
        assert!(!creature_op_enabled(Dismiss, true, false, true));
        // 没有其它种类召唤：与既有语义一致
        assert!(creature_op_enabled(Summon, true, false, false));
        assert!(!creature_op_enabled(Summon, true, true, false));
    }
}
