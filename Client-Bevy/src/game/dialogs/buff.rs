// ============================================================================
// 状态/增益对话框（C# `BuffDialog` 对齐；#2791 单元④）
// 参考：C# `Client/MirScenes/Dialogs/BuffDialog.cs`
// 网络（本端简化 wire，补 C# `ClientBuff` 里渲染所需的字段）：
//   S: AddBuff[tag u8][remaining_ms u32][paused u8][value_count u8][values i32…]
//      RemoveBuff[tag u8]
// 布局（C# `BuffDialog`：`Index = 20`、`Location = (ScreenWidth-170, 0)` = (854,0)）：
//   - 面板 `Prguse2[20..30]`：`count <= 10 → 20 + count - 1`，否则 `30`（两行）
//   - 图标 `BuffIcon[BuffImage(tag)]` 24x24：`x = panel_w - 10 - 23 - i*23 + 230*(i/10)`、
//     `y = 6 + 24*(i/10)`（C# 公式）
//   - 收起态：`Prguse2[20]` 44x34 + 黄色数量标签，只显示 i==0 的图标
//   - 悬停面板矩形才显形（C# `Opacity 0→1`；本端直接显隐，不逐帧复刻渐隐动画）
//   - 剩余 ≤5s：每 500ms 闪烁（C# `image.Index = -1`）
// 文案（C# `BuffString` / `CombinedBuffText`）：名称（C# `BuffType`/`PoisonType` 本地化）→
//   类型描述 → 属性行（`增加/降低 <Stat> ：值[%]`）→ 过期时间；收起态只显全量属性合计。
//
// 说明：C# 图标布局用的是 `Size.Width`，而展开态它被改写成 `count*23`（比对应面板精灵窄 ~21px），
// 本端按**面板精灵自然宽度**布局（与 art 对齐，逐槽 23px 右对齐；1 格时 C# 会把图标排到面板左缘外
// 10px）。近似清单见 `UI_COMPONENTS.md` §7。
// ============================================================================

use std::collections::HashMap;

use bevy::prelude::*;
use mir2_shared::enums::Stat;

use crate::game::dialogs::{DialogKind, DialogRoot};
use crate::game::player_state::StatusFlags;
use crate::game::sets::GameSet;
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{load_lib_image, spawn_icon_button, spawn_image, spawn_label, spawn_panel};

/// 面板精灵 `Prguse2[20..=30]` 的实测尺寸（`w, h`）——C# 面板 art 决定布局基准
const PANEL_SIZES: [(f32, f32); 11] = [
    (44.0, 34.0),
    (68.0, 34.0),
    (92.0, 34.0),
    (112.0, 34.0),
    (136.0, 34.0),
    (160.0, 34.0),
    (184.0, 34.0),
    (204.0, 34.0),
    (228.0, 34.0),
    (252.0, 34.0),
    (252.0, 58.0),
];

/// 面板**右边**锚点（C# `Location = (ScreenWidth - 170, 0)` 且展开时 `newX = Location.X -
/// Size.Width + oldWidth` → 右缘恒在 `854 + 44 = 898`，向左长；正好贴着小地图左缘）
const PANEL_RIGHT: f32 = 898.0;
const PANEL_Y: f32 = 0.0;
/// 收起态宽度（C# `Size(44, 34)`）
const PANEL_COLLAPSED_W: f32 = 44.0;

/// 图标槽上限（C# 两行 × 10）
pub(crate) const BUFF_ICON_SLOTS: usize = 20;
/// 图标尺寸（`BuffIcon` 库实测 24x24）
const ICON_SIZE: f32 = 24.0;

/// Buff 条目（服务端 `S.AddBuff`）
#[derive(Debug, Clone, Default)]
pub struct BuffEntry {
    /// 服务端 `buff_tag`（Rust 侧 BuffType 判别值）
    pub tag: u8,
    /// 剩余时长（ms）。服务端下发的是**时长**，本端逐帧递减用于倒计时与 ≤5s 闪烁
    pub remaining_ms: u32,
    /// C# `Buff.Paused`（暂停期间不倒计时）
    pub paused: bool,
    /// 数字参数（对应 C# `ClientBuff.Values`，用于「增加/降低 …：值」行）
    pub values: Vec<i32>,
}

/// Buff 状态
#[derive(Resource, Default)]
pub struct BuffState {
    pub buffs: Vec<BuffEntry>,
    pub message: String,
    /// 展开/收起（C# `BuffDialog.ExpandedBuffWindow`，[Game] 段持久化）
    pub expanded: bool,
}

impl BuffState {
    /// 从 Mir2Config.ini 解析展开状态（C# Settings.ExpandedBuffWindow；缺失默认展开）
    pub fn from_ini(content: &str) -> Self {
        use crate::game::dialogs::settings_file::ini_bool;
        Self {
            buffs: Vec::new(),
            message: String::new(),
            expanded: ini_bool(content, "Game", "ExpandedBuffWindow", true),
        }
    }

    /// 启动时加载（C# Settings.Load）
    pub fn load() -> Self {
        Self::from_ini(&crate::game::dialogs::settings_file::load_ini())
    }

    /// 保存展开状态（C# Settings.Save；merge 写回）
    pub fn save_expanded(&self) {
        use crate::game::dialogs::settings_file::{set_ini_value, write_ini};
        let content = crate::game::dialogs::settings_file::load_ini();
        let content = set_ini_value(
            &content,
            "Game",
            "ExpandedBuffWindow",
            &self.expanded.to_string(),
        );
        write_ini(&content);
        tracing::debug!("⚙️ Buff 窗口展开状态已保存: {}", self.expanded);
    }
}

/// tag → 显示名（与服务端 `buff_tag` 对应；自动化脚本/日志用）
pub fn buff_name(tag: u8) -> &'static str {
    buff_display(tag).name
}

/// tag 的 C# 展示信息（图标/名称/描述/属性键）。
///
/// Rust 端 BuffType 是粗粒度模型（`AttackBoost` 同时覆盖 C# `Rage` 与 Buff 药水 `Impact`；
/// `Invisibility` 覆盖 `Hiding`/`MoonLight`/`DarkBody`），此处取**代表来源**：
/// 药水系（DC/MC/SC/攻速/HP/MP/AC/MAC/负重）→ C# `Impact/Magic/Taoist/Storm/HealthAid/ManaAid/
/// Defence/MagicDefence/BagWeight`；法术系 → `Rage/SwiftFeet/LightBody/Concentration/…`；
/// 毒类（`Poison/Slow/Frozen/Stun`）→ C# `PoisonType` 的图标与名称（C# 归 `PoisonBuffDialog`）。
#[derive(Debug, Clone, Copy)]
pub(crate) struct BuffDisplay {
    pub name: &'static str,
    /// C# `BuffString` 的类型描述段（逐字，含换行；无描述 = ""）
    pub description: &'static str,
    /// C# `BuffImage(type)` 图标 index（`BuffIcon` 库）
    pub icon: usize,
    /// 值与属性键的对齐表（空 = 该类型不显示属性行）
    pub stats: &'static [Stat],
    /// 属性值是否带 `%` 后缀（C#：`Stat` 名含 `Percent`）
    pub percent: bool,
}

const D_HP: &[Stat] = &[Stat::HP];
const D_MP: &[Stat] = &[Stat::MP];
const D_MAXDC: &[Stat] = &[Stat::MaxDC];
const D_MAXMC: &[Stat] = &[Stat::MaxMC];
const D_MAXSC: &[Stat] = &[Stat::MaxSC];
const D_MAXAC: &[Stat] = &[Stat::MaxAC];
const D_MAXMAC: &[Stat] = &[Stat::MaxMAC];
const D_ATKSPD: &[Stat] = &[Stat::AttackSpeed];
const D_AGI: &[Stat] = &[Stat::Agility];
const D_CRIT: &[Stat] = &[Stat::CriticalRate];
const D_REFLECT: &[Stat] = &[Stat::Reflect];
const D_SPELLREC: &[Stat] = &[Stat::SpellRecovery];
const D_BAGWEIGHT: &[Stat] = &[Stat::BagWeight];
const D_DMG_RED: &[Stat] = &[Stat::DamageReductionPercent];
const D_TELE_PENALTY: &[Stat] = &[Stat::TeleportManaPenaltyPercent];
const D_CRIT_RATE: &[Stat] = &[Stat::MaxDCRatePercent];
const D_RHINO: &[Stat] = &[Stat::MaxDC, Stat::MaxMC, Stat::MaxSC];
const D_NONE: &[Stat] = &[];

pub(crate) fn buff_display(tag: u8) -> BuffDisplay {
    // 名称逐字来自 `Client/Localization/Chinese.json` 的 `Enum` 段（`BuffType_*` / `PoisonType_*`）
    match tag {
        // ===== 药水系（C# `PlayerObject.UseItem` Buff 药水分支 :5849-5879）=====
        0 => BuffDisplay {
            name: "生命增益",
            description: "",
            icon: 161,
            stats: D_HP,
            percent: false,
        },
        1 => BuffDisplay {
            name: "法力增益",
            description: "",
            icon: 169,
            stats: D_MP,
            percent: false,
        },
        2 => BuffDisplay {
            name: "攻击加成",
            description: "",
            icon: 249,
            stats: D_MAXDC,
            percent: false,
        },
        3 => BuffDisplay {
            name: "物防加成",
            description: "",
            icon: 166,
            stats: D_MAXAC,
            percent: false,
        },
        11 => BuffDisplay {
            name: "风暴",
            description: "",
            icon: 170,
            stats: D_ATKSPD,
            percent: false,
        },
        16 => BuffDisplay {
            name: "法力增益",
            description: "",
            icon: 169,
            stats: D_MP,
            percent: false,
        },
        21 => BuffDisplay {
            name: "魔法加成",
            description: "",
            icon: 165,
            stats: D_MAXMC,
            percent: false,
        },
        22 => BuffDisplay {
            name: "道术加成",
            description: "",
            icon: 250,
            stats: D_MAXSC,
            percent: false,
        },
        27 => BuffDisplay {
            name: "负重提升",
            description: "",
            icon: 235,
            stats: D_BAGWEIGHT,
            percent: false,
        },
        28 => BuffDisplay {
            name: "生命增益",
            description: "",
            icon: 161,
            stats: D_HP,
            percent: false,
        },
        // ===== 法术系 =====
        4 => BuffDisplay {
            name: "神圣战甲术",
            description: "",
            icon: 14,
            stats: D_MAXAC,
            percent: false,
        },
        5 => BuffDisplay {
            name: "幽灵盾",
            description: "",
            icon: 13,
            stats: D_MAXMAC,
            percent: false,
        },
        6 => BuffDisplay {
            name: "魔法盾",
            description: "",
            icon: 30,
            stats: D_DMG_RED,
            percent: true,
        },
        10 => BuffDisplay {
            name: "月影隐身",
            description: "在远距离时对玩家及多数怪物隐形。\n",
            icon: 65,
            stats: D_NONE,
            percent: false,
        },
        12 => BuffDisplay {
            name: "迅足",
            description: "",
            icon: 67,
            stats: D_NONE,
            percent: false,
        },
        13 => BuffDisplay {
            name: "轻身术",
            description: "",
            icon: 68,
            stats: D_AGI,
            percent: false,
        },
        14 => BuffDisplay {
            name: "怒气",
            description: "",
            icon: 49,
            stats: D_CRIT,
            percent: false,
        },
        15 => BuffDisplay {
            name: "专注",
            description: "增加元素提取几率。\n",
            icon: 96,
            stats: D_SPELLREC,
            percent: false,
        },
        17 => BuffDisplay {
            name: "能量护盾",
            description: "",
            icon: 57,
            stats: D_REFLECT,
            percent: true,
        },
        23 => BuffDisplay {
            name: "变身",
            description: "伪装你的外观。\n",
            icon: 241,
            stats: D_NONE,
            percent: false,
        },
        24 => BuffDisplay {
            name: "时空涌动",
            description: "",
            icon: 261,
            stats: D_TELE_PENALTY,
            percent: true,
        },
        25 => BuffDisplay {
            name: "诅咒",
            description: "",
            icon: 45,
            stats: D_CRIT_RATE,
            percent: true,
        },
        26 => BuffDisplay {
            name: "犀牛祭司减益",
            description: "",
            icon: 217,
            stats: D_RHINO,
            percent: false,
        },
        // ===== 毒类（C# `PoisonBuffDialog` 的图标/名称；不臆造 tick 数字）=====
        7 => BuffDisplay {
            name: "绿毒",
            description: "",
            icon: 221,
            stats: D_NONE,
            percent: false,
        },
        19 => BuffDisplay {
            name: "缓慢",
            description: "降低移动速度。\n",
            icon: 225,
            stats: D_NONE,
            percent: false,
        },
        20 => BuffDisplay {
            name: "冰冷",
            description: "禁止施法、移动以及攻击。\n",
            icon: 223,
            stats: D_NONE,
            percent: false,
        },
        9 => BuffDisplay {
            name: "昏眩",
            description: "",
            icon: 224,
            stats: D_NONE,
            percent: false,
        },
        // 沉默/嘲讽：C# 无对应 BuffType（沉默是技能效果、嘲讽只在怪物 AI），取 `Curse` 图标占位
        8 | 18 => BuffDisplay {
            name: "诅咒",
            description: "",
            icon: 45,
            stats: D_NONE,
            percent: false,
        },
        _ => BuffDisplay {
            name: "未知",
            description: "",
            icon: 0,
            stats: D_NONE,
            percent: false,
        },
    }
}

/// C# `BuffType` 判别值 → 本端 tag（仅供 `S.PauseBuff` 事件反查：服务端只对 `Transform` 发暂停
/// ——`PlayerActor::ToggleTransform`；mock 的 `--recipe-test` 用 `Haste`）
pub(crate) fn tag_from_csharp(buff_type: u8) -> Option<u8> {
    match buff_type {
        109 => Some(23), // C# BuffType.Transform → tag 23
        6 => Some(11),   // C# BuffType.Haste → tag 11（AttackSpeedBoost）
        _ => None,
    }
}

/// C# `Stat` 本地化名（`Chinese.json` `Enum.Stat_*`，逐字；仅列本端用到的键）
pub(crate) fn stat_name(stat: Stat) -> &'static str {
    match stat {
        Stat::MinAC => "最小物防",
        Stat::MaxAC => "最大物防",
        Stat::MinMAC => "最小魔防",
        Stat::MaxMAC => "最大魔防",
        Stat::MinDC => "最小攻击",
        Stat::MaxDC => "最大攻击",
        Stat::MinMC => "最小魔法",
        Stat::MaxMC => "最大魔法",
        Stat::MinSC => "最小道术",
        Stat::MaxSC => "最大道术",
        Stat::Accuracy => "准确",
        Stat::Agility => "敏捷",
        Stat::HP => "生命值",
        Stat::MP => "魔法值",
        Stat::AttackSpeed => "攻速",
        Stat::Luck => "幸运",
        Stat::BagWeight => "包裹负重",
        Stat::Reflect => "反射",
        Stat::SpellRecovery => "魔法恢复",
        Stat::CriticalRate => "暴击",
        Stat::MaxDCRatePercent => "最大攻击",
        Stat::DamageReductionPercent => "伤害减免",
        Stat::TeleportManaPenaltyPercent => "传送魔法惩罚",
        _ => "未知",
    }
}

/// C# `ClientTextKeys.BuffEffect`（`Chinese.json` Text.BuffEffect）= `{0} {1} ：{2}{3}\n`：
/// `{0}` = 增加/降低、`{1}` = Stat 本地化名、`{2}` = 值、`{3}` = `%`（Percent 类）
fn stat_line(stat: Stat, value: i32, percent: bool) -> String {
    let c = if value < 0 { "降低" } else { "增加" };
    let sign = if percent { "%" } else { "" };
    format!("{c} {} ：{value}{sign}\n", stat_name(stat))
}

/// C# `ClientTextKeys.Expire` / `ExpirePaused`：`过期: {时长}`（`PrintTimeSpanFromSeconds`）
fn expire_line(entry: &BuffEntry) -> String {
    if entry.paused {
        "过期: 已暂停".to_string()
    } else {
        let secs = (entry.remaining_ms as f64 / 1000.0).round();
        format!("过期: {}", crate::game::time_format::format_time_span(secs))
    }
}

/// C# `BuffDialog.BuffString(buff)`（展开态单条 Hint）：名称行 + 类型描述 + 属性行 + 过期
/// （C# 还有「施法者」行，但 `ClientBuff` 不序列化 `Caster`，该行在 C# 也永不出现）
pub(crate) fn buff_hint(entry: &BuffEntry) -> String {
    let d = buff_display(entry.tag);
    let mut t = String::with_capacity(64);
    t.push_str(d.name);
    t.push('\n');
    t.push_str(d.description);
    for (i, stat) in d.stats.iter().enumerate() {
        if let Some(v) = entry.values.get(i) {
            t.push_str(&stat_line(*stat, *v, d.percent));
        }
    }
    t.push_str(&expire_line(entry));
    t
}

/// C# `BuffDialog.CombinedBuffText()`（收起态）：`当前增益效果\n` + 各 buff 属性合计
/// （C# 用 `Stats.Add` 累加同键）
pub(crate) fn combined_buff_text(buffs: &[BuffEntry]) -> String {
    let mut agg: Vec<(Stat, i32, bool)> = Vec::new();
    for b in buffs {
        let d = buff_display(b.tag);
        for (i, stat) in d.stats.iter().enumerate() {
            let Some(v) = b.values.get(i) else { continue };
            if let Some(slot) = agg.iter_mut().find(|(s, _, _)| s == stat) {
                slot.1 += *v;
            } else {
                agg.push((*stat, *v, d.percent));
            }
        }
    }
    let mut t = String::from("当前增益效果\n");
    for (stat, v, percent) in agg {
        t.push_str(&stat_line(stat, v, percent));
    }
    t
}

/// C# `BuffDialog.UpdateWindow` 的面板 index：`count <= 10 → 20 + count - 1`，否则 `30`
pub(crate) fn panel_index(count: usize) -> usize {
    if count <= 10 {
        20 + count.max(1) - 1
    } else {
        30
    }
}

/// C# 收起态固定 `Index = 20`（44x34）；只有展开态才按 count 取面板
pub(crate) fn panel_index_for(count: usize, expanded: bool) -> usize {
    if expanded {
        panel_index(count)
    } else {
        20
    }
}

/// C# 图标位置公式：`x = panel_w - 10 - 23 - i*23 + 230*(i/10)`、`y = 6 + 24*(i/10)`
pub(crate) fn icon_offset(i: usize, panel_w: f32) -> (f32, f32) {
    let x = panel_w - 10.0 - 23.0 - i as f32 * 23.0 + 230.0 * (i / 10) as f32;
    let y = 6.0 + (i / 10) as f32 * 24.0;
    (x, y)
}

/// C# 快到期的闪烁：`round((ExpireTime - now)/100ms) % 10 < 5` → `image.Index = -1`（不画）；
/// `Paused` 时 C# 提前 `continue`，不闪
pub(crate) fn blink_hidden(remaining_ms: u32, paused: bool) -> bool {
    if paused || remaining_ms > 5_000 {
        return false;
    }
    let units = (remaining_ms as f32 / 100.0).round() as i64;
    units.rem_euclid(10) < 5
}

#[derive(Component)]
pub struct BuffWidget;

/// 面板本体（C# `Size(44,34)` 收起 / 展开用 `Prguse2[20+count-1]`）
#[derive(Component)]
pub struct BuffPanel;

/// 展开/收起按钮（C# `_expandCollapseButton`，`Prguse2 7/8/9`，16x15）
#[derive(Component)]
pub struct BuffExpand;

/// 收起时显示的状态数量标签（C# `_buffCountLabel`）
#[derive(Component)]
pub struct BuffCount;

/// 第 `i` 个图标槽（C# `_buffList[i]`）
#[derive(Component)]
pub struct BuffIcon(pub usize);

/// 面板/图标贴图（按 count / icon index 取）
#[derive(Resource, Default)]
pub struct BuffAssets {
    pub panels: [Option<Handle<Image>>; 11],
    pub icons: HashMap<usize, Handle<Image>>,
}

pub struct BuffPlugin;

impl Plugin for BuffPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(BuffState::load());
        app.init_resource::<BuffAssets>();
        app.add_systems(
            Update,
            buff_server_events
                .run_if(in_state(AppState::Game))
                .in_set(GameSet::PlayerState),
        );
        app.add_systems(OnEnter(AppState::Game), spawn_buff);
        app.add_systems(OnExit(AppState::Game), cleanup_buff);
        app.add_systems(Update, buff_ui_system.run_if(in_state(AppState::Game)));
    }
}

fn cleanup_buff(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_buff(
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
    let cjk = shared_cjk_font(&mut fonts, &mut cjk_font);

    // 面板贴图池（Prguse2[20..=30]）+ 显示表里用到的全部图标（BuffIcon）
    let mut assets = BuffAssets::default();
    for (i, _) in PANEL_SIZES.iter().enumerate() {
        assets.panels[i] = load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 20 + i);
    }
    for tag in 0..=28u8 {
        let icon = buff_display(tag).icon;
        if !assets.icons.contains_key(&icon) {
            if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::BuffIcon, icon) {
                assets.icons.insert(icon, h);
            }
        }
    }
    commands.insert_resource(assets);

    let (pw, ph) = PANEL_SIZES[0];
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 20) else {
        return;
    };
    let panel = spawn_panel(
        &mut commands,
        bg,
        PANEL_RIGHT - PANEL_COLLAPSED_W,
        PANEL_Y,
        pw,
        ph,
        30,
    );
    // `AlwaysVisible`：C# BuffDialog 不由对话框栈驱动（`Opacity` 悬停渐隐），本端走状态驱动，
    // 避免 `enforce_dialog_visibility` 的「未 open → Hidden」把面板每帧按下去
    commands.entity(panel).insert((
        DialogRoot(DialogKind::Buff),
        BuffWidget,
        BuffPanel,
        crate::game::dialogs::AlwaysVisible,
    ));

    commands.entity(panel).with_children(|p| {
        // 图标槽（C# 每 buff 一个 MirImageControl；本端固定 20 槽按 count 显隐）
        for i in 0..BUFF_ICON_SLOTS {
            let (x, y) = icon_offset(i, pw);
            if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::BuffIcon, 0) {
                spawn_image(p, h, x, y, ICON_SIZE, ICON_SIZE, 5).insert((
                    BuffIcon(i),
                    crate::ui::tooltip::UiHint {
                        text: String::new(),
                    },
                    Visibility::Hidden,
                ));
            }
        }
        // 收起时的数量标签（C# `_buffCountLabel`：黄色粗体）
        spawn_label(p, &cjk, "", 18.0, 9.0, 12.0, Color::srgb(1.0, 1.0, 0.0), 10).insert(BuffCount);
        // 展开/收起按钮（C# `_expandCollapseButton`：`Prguse2[7/8/9]`，16x15 @(panel_w-15, 0)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 7),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 8),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 9),
        ) {
            spawn_icon_button(p, n, h, pr, pw - 15.0, 0.0, 16.0, 15.0, 10).insert(BuffExpand);
        }
    });
}

/// 显隐 + 图标行渲染 + Hint + 展开/收起 + 倒计时/闪烁（#2791 单元④）
#[allow(clippy::too_many_arguments)]
fn buff_ui_system(
    time: Res<Time>,
    probe: Res<crate::control::CursorProbe>,
    windows: Query<&Window>,
    mut state: ResMut<BuffState>,
    assets: Res<BuffAssets>,
    mut panels: Query<
        (
            &mut Visibility,
            &mut Node,
            &mut ImageNode,
            Option<&BuffPanel>,
            Option<&BuffIcon>,
            Option<&mut crate::ui::tooltip::UiHint>,
        ),
        (Without<BuffExpand>, Without<BuffCount>),
    >,
    mut count_label: Query<
        (&mut Visibility, &mut Text),
        (
            With<BuffCount>,
            Without<BuffPanel>,
            Without<BuffIcon>,
            Without<BuffExpand>,
        ),
    >,
    mut expand: Query<
        (Entity, &Interaction, &mut Node),
        (
            With<BuffExpand>,
            Without<BuffPanel>,
            Without<BuffIcon>,
            Without<BuffCount>,
        ),
    >,
    mut prev_inter: Local<HashMap<Entity, Interaction>>,
) {
    fn edge(e: Entity, inter: &Interaction, prev: &mut HashMap<Entity, Interaction>) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }

    // 倒计时（服务端到期也会发 RemoveBuff；这里只驱动显示与闪烁）
    let dt_ms = (time.delta_secs() * 1000.0) as u32;
    for b in state.buffs.iter_mut() {
        if !b.paused {
            b.remaining_ms = b.remaining_ms.saturating_sub(dt_ms);
        }
    }

    let count = state.buffs.len();
    let expanded = state.expanded;
    let index = panel_index_for(count, expanded);
    let (pw, ph) = PANEL_SIZES[index - 20];

    // 悬停面板矩形才显形（C# `Process` 的 Opacity 渐隐；本端直接显隐）
    let cursor = crate::control::resolve_cursor(
        probe.pos,
        windows.single().ok().and_then(|w| w.cursor_position()),
    );
    let panel_left = PANEL_RIGHT - pw;
    let hovered = cursor
        .map(|c| c.x >= panel_left && c.x <= PANEL_RIGHT && c.y >= PANEL_Y && c.y <= PANEL_Y + ph)
        .unwrap_or(false);
    let visible = count > 0 && hovered;

    // 每个可见槽的目标状态（显隐 / 位置 / 图标 / Hint）
    let mut icon_states: Vec<(bool, f32, f32, usize, String)> =
        Vec::with_capacity(count.min(BUFF_ICON_SLOTS));
    for i in 0..count.min(BUFF_ICON_SLOTS) {
        let entry = &state.buffs[i];
        let d = buff_display(entry.tag);
        let (x, y) = icon_offset(i, pw);
        // C#：展开态全部显示；收起态只显示 i==0；≤5s 闪烁
        let shown = if expanded {
            !blink_hidden(entry.remaining_ms, entry.paused)
        } else {
            i == 0 && !blink_hidden(entry.remaining_ms, entry.paused)
        };
        let hint = if expanded {
            buff_hint(entry)
        } else {
            combined_buff_text(&state.buffs)
        };
        icon_states.push((shown, x, y, d.icon, hint));
    }
    let panel_vis = if visible {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    let panel_image = if count > 0 {
        assets.panels[index - 20].clone()
    } else {
        None
    };
    let count_text = format!("{count}");
    let count_vis = if visible && !expanded {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    for (mut vis, mut node, mut image, panel, icon, hint) in &mut panels {
        if panel.is_some() {
            *vis = panel_vis;
            node.left = Val::Px(panel_left);
            node.width = Val::Px(pw);
            node.height = Val::Px(ph);
            if let Some(h) = panel_image.clone() {
                if image.image != h {
                    image.image = h;
                }
            }
            continue;
        }
        if let Some(slot) = icon {
            match icon_states.get(slot.0) {
                Some((shown, x, y, icon_index, hint_text)) => {
                    *vis = if visible && *shown {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    };
                    node.left = Val::Px(*x);
                    node.top = Val::Px(*y);
                    node.width = Val::Px(ICON_SIZE);
                    node.height = Val::Px(ICON_SIZE);
                    if let Some(h) = assets.icons.get(icon_index) {
                        if image.image != *h {
                            image.image = h.clone();
                        }
                    }
                    if let Some(mut hint) = hint {
                        if hint.text != *hint_text {
                            hint.text = hint_text.clone();
                        }
                    }
                }
                None => {
                    *vis = Visibility::Hidden;
                    if let Some(mut hint) = hint {
                        hint.text.clear();
                    }
                }
            }
        }
    }

    for (mut vis, mut text) in &mut count_label {
        *vis = count_vis;
        if text.0 != count_text {
            text.0 = count_text.clone();
        }
    }

    // 展开/收起按钮（C#：`_expandCollapseButton` @(panel_w - 15, 0)；1 个 buff 时点击必展开）
    for (e, inter, mut node) in &mut expand {
        node.left = Val::Px(pw - 15.0);
        node.top = Val::Px(0.0);
        if !visible || !edge(e, inter, &mut prev_inter) {
            continue;
        }
        if state.buffs.len() == 1 {
            state.expanded = true;
        } else {
            state.expanded = !state.expanded;
        }
        state.save_expanded();
        tracing::info!(
            "🩹 Buff 窗口{}",
            if state.expanded { "展开" } else { "收起" }
        );
    }
}

/// 消费服务端状态事件（网络层只广播 ServerEvent）
/// #2633 批次4 步9：sprint/sneaking 直写 `StatusFlags` 组件（hud.* 双写已删）；
/// 组件写 `single_mut()` 失败（实体未生成）跳过不 panic（R1 同理）。
pub(crate) fn buff_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut buff: ResMut<BuffState>,
    mut flags_q: Query<&mut StatusFlags, With<crate::actor::LocalPlayer>>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        match ev {
            ServerEvent::BuffAdded {
                tag,
                remaining_ms,
                paused,
                values,
            } => {
                // C# `GameScene.AddBuff`：同类型替换（不叠加）
                if let Some(e) = buff.buffs.iter_mut().find(|b| b.tag == *tag) {
                    e.remaining_ms = *remaining_ms;
                    e.paused = *paused;
                    e.values = values.clone();
                } else {
                    // C# `BuffDialog.CreateBuff`：`_buffList.Insert(0, image)` → **最新在 i=0**
                    // （图标行最右），旧 buff 依次左移
                    buff.buffs.insert(
                        0,
                        BuffEntry {
                            tag: *tag,
                            remaining_ms: *remaining_ms,
                            paused: *paused,
                            values: values.clone(),
                        },
                    );
                }
                buff.message = format!("获得状态: {}", buff_name(*tag));
                tracing::info!(
                    "✨ AddBuff: tag={} {}ms values={:?}",
                    tag,
                    remaining_ms,
                    values
                );
                // #1552：SwiftFeet(12)/MoonLight(10) → StatusFlags 移动旗标
                if let Ok(mut f) = flags_q.single_mut() {
                    match *tag {
                        12 => f.sprint = true,
                        10 => f.sneaking = true,
                        _ => {}
                    }
                }
            }
            ServerEvent::BuffRemoved { tag } => {
                buff.buffs.retain(|b| b.tag != *tag);
                buff.message = format!("状态消失: {}", buff_name(*tag));
                tracing::info!("✨ RemoveBuff: tag={}", tag);
                if let Ok(mut f) = flags_q.single_mut() {
                    match *tag {
                        12 => f.sprint = false,
                        10 => f.sneaking = false,
                        _ => {}
                    }
                }
            }
            ServerEvent::BuffPaused {
                buff_type, paused, ..
            } => {
                // `S.PauseBuff` 走 C# 线格式（`BuffType` 判别值）：服务端只对 Transform 发暂停
                // （`@TOGGLETRANSFORM`），mock 的 `--recipe-test` 用 Haste —— 两者都反查回本端 tag
                if let Some(tag) = tag_from_csharp(*buff_type) {
                    if let Some(e) = buff.buffs.iter_mut().find(|b| b.tag == tag) {
                        e.paused = *paused;
                        tracing::info!("⏸ Buff tag={} 暂停={}", tag, paused);
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(tag: u8, ms: u32, values: Vec<i32>) -> BuffEntry {
        BuffEntry {
            tag,
            remaining_ms: ms,
            paused: false,
            values,
        }
    }

    /// #2791 单元④：面板 index/尺寸按 count（C# `UpdateWindow`）
    #[test]
    fn panel_index_matches_csharp() {
        assert_eq!(panel_index(1), 20);
        assert_eq!(panel_index(2), 21);
        assert_eq!(panel_index(10), 29);
        assert_eq!(panel_index(11), 30, ">10 用两行面板");
        assert_eq!(panel_index(20), 30);
        assert_eq!(PANEL_SIZES[panel_index(1) - 20].0, 44.0);
        assert_eq!(PANEL_SIZES[panel_index(3) - 20].0, 92.0);
        // 收起态恒用 20（44x34）——回归：此前按 count 取面板，折叠时宽度还是展开的 92
        assert_eq!(panel_index_for(3, false), 20);
        assert_eq!(panel_index_for(3, true), 22);
        assert_eq!(PANEL_SIZES[panel_index_for(3, false) - 20], (44.0, 34.0));
    }

    /// #2791 单元④：图标位置公式（C# `Size.Width - 10 - 23 - i*23 + 230*(i/10)`）
    #[test]
    fn icon_offset_matches_csharp_formula() {
        assert_eq!(icon_offset(0, 44.0), (11.0, 6.0), "1 格：面板内右对齐");
        assert_eq!(icon_offset(1, 68.0), (12.0, 6.0));
        assert_eq!(icon_offset(2, 92.0), (13.0, 6.0));
        assert_eq!(icon_offset(0, 252.0), (219.0, 6.0));
        assert_eq!(icon_offset(10, 252.0), (219.0, 30.0), "第 11 格换行回右侧");
    }

    /// #2791 单元④：`BuffString` 文案（C# `Chinese.json` Text/Enum 逐字）
    #[test]
    fn buff_hint_matches_csharp_buffstring() {
        assert_eq!(
            buff_hint(&entry(10, 25_000, vec![])),
            "月影隐身\n在远距离时对玩家及多数怪物隐形。\n过期: 25s"
        );
        assert_eq!(
            buff_hint(&entry(2, 30_000, vec![12])),
            "攻击加成\n增加 最大攻击 ：12\n过期: 30s"
        );
        assert_eq!(
            buff_hint(&entry(6, 8_000, vec![15])),
            "魔法盾\n增加 伤害减免 ：15%\n过期: 8s"
        );
        // 负值 → 降低（C# `Decreases`），值按原样带负号
        assert_eq!(
            buff_hint(&entry(3, 5_000, vec![-7])),
            "物防加成\n降低 最大物防 ：-7\n过期: 5s"
        );
        // 多值：犀牛祭司减益（三字段固定降值；服务端按 C# `RhinoPriest.cs:91-93` 下发负值）
        assert_eq!(
            buff_hint(&entry(26, 9_000, vec![-3, -4, -5])),
            "犀牛祭司减益\n降低 最大攻击 ：-3\n降低 最大魔法 ：-4\n降低 最大道术 ：-5\n过期: 9s"
        );
        // 暂停（C# `ExpirePaused`）
        let mut paused = entry(23, 4_000, vec![]);
        paused.paused = true;
        assert_eq!(buff_hint(&paused), "变身\n伪装你的外观。\n过期: 已暂停");
    }

    /// #2791 单元④：收起态合计（C# `CombinedBuffText` = `当前增益效果\n` + `Stats.Add` 合计）
    #[test]
    fn combined_text_sums_same_stat() {
        let buffs = vec![
            entry(2, 1000, vec![10]),
            entry(2, 1000, vec![5]),
            entry(3, 1000, vec![7]),
        ];
        assert_eq!(
            combined_buff_text(&buffs),
            "当前增益效果\n增加 最大攻击 ：15\n增加 最大物防 ：7\n"
        );
        assert_eq!(combined_buff_text(&[]), "当前增益效果\n");
    }

    /// #2791 单元④：≤5s 闪烁（C# `round((ExpireTime-now)/100ms) % 10 < 5`；Paused 不闪）
    #[test]
    fn blink_matches_csharp() {
        assert!(!blink_hidden(5_100, false), ">5s 不闪");
        assert!(
            blink_hidden(5_000, false),
            "5.0s → round(50)%10=0 <5 → 隐藏"
        );
        assert!(
            blink_hidden(4_400, false),
            "4.4s → round(44)%10=4 <5 → 隐藏"
        );
        assert!(
            !blink_hidden(4_600, false),
            "4.6s → round(46)%10=6 ≥5 → 显示"
        );
        assert!(
            !blink_hidden(3_000, true),
            "暂停不闪（C# `buff.Paused` 提前 continue）"
        );
    }

    /// #2791 单元④：显示表抽样（图标/名称/属性键，C# `BuffImage` + `Chinese.json`）
    #[test]
    fn buff_display_samples_match_csharp_tables() {
        let d = buff_display(10);
        assert_eq!((d.name, d.icon), ("月影隐身", 65));
        let d = buff_display(2);
        assert_eq!((d.name, d.icon, d.stats), ("攻击加成", 249, D_MAXDC));
        let d = buff_display(26);
        assert_eq!(d.stats, D_RHINO);
        assert_eq!(
            buff_display(7).icon,
            221,
            "绿毒（C# PoisonType.Green → BuffIcon 221）"
        );
        assert_eq!(
            buff_display(19).icon,
            225,
            "缓慢（C# PoisonType.Slow → BuffIcon 225）"
        );
        assert_eq!(buff_display(20).icon, 223, "冰冷");
    }
}
