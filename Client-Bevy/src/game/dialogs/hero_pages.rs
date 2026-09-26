// ============================================================================
// #2892 批57：英雄对话框的「状态页 / 状态二页」与四页签（C# `CharacterDialog`）
//
// C# `HeroDialog = new CharacterDialog(MirGridType.HeroEquipment, Hero)`，与角色窗同源
// （`Client/MirScenes/Dialogs/CharacterDialog.cs`）：
//   - 四页签 `CharacterButton/StatusButton/StateButton/SkillButton`
//     = `Title[500..503]`，64x20 @(8/70/132/194, 70)（`:146-200`）；当前页用按下帧
//   - `StatusPage` = `Title[506]` @(8,90)：AC/MAC/DC/MC/SC、HP/MP、暴击率/伤害、
//     攻速、命中、敏捷、幸运（标签 x=126，y = 20 + 18i，`:94-109`）
//   - `StatePage`  = `Title[507]` @(8,90)：经验 %、背包/穿戴/手持重量、魔法躲避、毒躲避、
//     体力/魔法/毒恢复、神圣、冰冻、毒攻击（标签同上，`:121-133`）
//
// 本端与 C# 同构：**一个**英雄对话框窗（`DialogKind::HeroEquipment`）承载四页，
// 页签点击改 [`HeroPageState`] → `hero_pages_system` 就地切页（页容器显隐）。
// `HeroInventoryDialog` 在 C# 里本就是独立窗，故 `HeroInventory` 保持独立 kind。
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager};
use crate::map_renderer::GameLibraries;
use crate::network::server_event::HeroStatsInfo;
use crate::resources::libraries::LibraryName;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont};
use crate::ui::theme::{load_lib_image, spawn_image, spawn_label};

/// 英雄对话框的四页（C# `CharacterDialog` 的 Character/Status/State/Skill）
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum HeroPage {
    /// 装备页（C# `CharacterPage` = `Prguse[340]`）
    #[default]
    Equipment,
    /// 状态页（C# `StatusPage` = `Title[506]`）
    Status,
    /// 状态二页（C# `StatePage` = `Title[507]`）
    State,
    /// 技能页（C# `SkillPage` = `Title[508]`）
    Skill,
}

/// 当前页（两窗共享；C# 是同一个 dialog 的页状态）
#[derive(Resource, Default)]
pub struct HeroPageState {
    pub page: HeroPage,
}

/// 页签按钮（`Title[500..503]` + 按下帧）
#[derive(Component)]
pub struct HeroTabBtn(pub HeroPage);

/// 页根（切页时整页显隐；装备页/技能页由各自窗口标记）
#[derive(Component)]
pub struct HeroPageRoot(pub HeroPage);

/// 状态页/状态二页的属性标签
#[derive(Component)]
pub struct HeroStatLabel(pub HeroStatKind);

/// 状态页/状态二页的标签种类（与 C# 标签一一对应）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HeroStatKind {
    // StatusPage
    Health,
    Mana,
    Ac,
    Mac,
    Dc,
    Mc,
    Sc,
    CriticalRate,
    CriticalDamage,
    AttackSpeed,
    Accuracy,
    Agility,
    Luck,
    // StatePage
    ExpPercent,
    BagWeight,
    WearWeight,
    HandWeight,
    MagicResist,
    PoisonResist,
    HealthRecovery,
    SpellRecovery,
    PoisonRecovery,
    Holy,
    Freezing,
    PoisonAttack,
}

/// 页签定义：`(页, 正常帧, 按下帧, x)`（C# `:146-200`，均 `Libraries.Title`，64x20 @ y=70）
pub const HERO_TABS: [(HeroPage, usize, usize, f32); 4] = [
    (HeroPage::Equipment, 500, 500, 8.0),
    (HeroPage::Status, 501, 501, 70.0),
    (HeroPage::State, 502, 502, 132.0),
    (HeroPage::Skill, 503, 503, 194.0),
];
pub const HERO_TAB_Y: f32 = 70.0;
pub const HERO_TAB_SIZE: (f32, f32) = (64.0, 20.0);

/// `HeroPage` → 页签下标（顺序与 [`HERO_TABS`] 一致：装备 0 / 状态 1 / State 2 / 技能 3）
/// —— 与角色窗的 `CharPage.0` 同口径，便于共用 [`crate::game::dialogs::character::char_tab_visible`]。
pub fn hero_page_index(page: HeroPage) -> usize {
    match page {
        HeroPage::Equipment => 0,
        HeroPage::Status => 1,
        HeroPage::State => 2,
        HeroPage::Skill => 3,
    }
}
/// 页区原点（C# `Location = new Point(8, 90)`）
pub const HERO_PAGE_X: f32 = 8.0;
pub const HERO_PAGE_Y: f32 = 90.0;
/// 状态页/状态二页标签：x=126、首行 y=20、行距 18（C# `:96-133`）
pub const HERO_LABEL_X: f32 = 126.0;
pub const HERO_LABEL_Y0: f32 = 20.0;
pub const HERO_LABEL_DY: f32 = 18.0;

/// 页签精灵开销很小：`Title[500..503]` 不存在时跳过（数据包缺图不 panic）
pub fn spawn_hero_tabs(
    parent: &mut ChildSpawnerCommands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
) {
    for (page, normal, pressed, x) in HERO_TABS {
        let (Some(n), Some(p)) = (
            load_lib_image(libs, images, LibraryName::Title, normal),
            load_lib_image(libs, images, LibraryName::Title, pressed),
        ) else {
            continue;
        };
        crate::ui::theme::spawn_icon_button(
            parent,
            n.clone(),
            n,
            p,
            x,
            HERO_TAB_Y,
            HERO_TAB_SIZE.0,
            HERO_TAB_SIZE.1,
            11,
        )
        .insert(HeroTabBtn(page));
    }
}

/// 状态页（`Title[506]` @(8,90)）+ 13 个标签（C# `:94-109`）
pub fn spawn_hero_status_page(
    parent: &mut ChildSpawnerCommands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    font: &Handle<Font>,
) {
    if let Some(art) = load_lib_image(libs, images, LibraryName::Title, 506) {
        let (w, h) = match libs.0.get_image(LibraryName::Title, 506) {
            Some(i) => (i.width.max(0) as f32, i.height.max(0) as f32),
            None => (190.0, 259.0),
        };
        spawn_image(parent, art, HERO_PAGE_X, HERO_PAGE_Y, w, h, 9)
            .insert(HeroPageRoot(HeroPage::Status));
    }
    const STATUS_LABELS: [(HeroStatKind, &str); 13] = [
        (HeroStatKind::Health, "0/0"),
        (HeroStatKind::Mana, "0/0"),
        (HeroStatKind::Ac, "0-0"),
        (HeroStatKind::Mac, "0-0"),
        (HeroStatKind::Dc, "0-0"),
        (HeroStatKind::Mc, "0-0"),
        (HeroStatKind::Sc, "0-0"),
        (HeroStatKind::CriticalRate, "0%"),
        (HeroStatKind::CriticalDamage, "0"),
        (HeroStatKind::AttackSpeed, "0"),
        (HeroStatKind::Accuracy, "+0"),
        (HeroStatKind::Agility, "+0"),
        (HeroStatKind::Luck, "0"),
    ];
    for (i, (kind, text)) in STATUS_LABELS.iter().enumerate() {
        spawn_label(
            parent,
            font,
            text,
            HERO_LABEL_X,
            HERO_LABEL_Y0 + i as f32 * HERO_LABEL_DY,
            12.0,
            Color::WHITE,
            10,
        )
        .insert((HeroStatLabel(*kind), HeroPageRoot(HeroPage::Status)));
    }
}

/// 状态二页（`Title[507]` @(8,90)）+ 12 个标签（C# `:121-133`）
pub fn spawn_hero_state_page(
    parent: &mut ChildSpawnerCommands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    font: &Handle<Font>,
) {
    if let Some(art) = load_lib_image(libs, images, LibraryName::Title, 507) {
        let (w, h) = match libs.0.get_image(LibraryName::Title, 507) {
            Some(i) => (i.width.max(0) as f32, i.height.max(0) as f32),
            None => (190.0, 259.0),
        };
        spawn_image(parent, art, HERO_PAGE_X, HERO_PAGE_Y, w, h, 9)
            .insert(HeroPageRoot(HeroPage::State));
    }
    const STATE_LABELS: [(HeroStatKind, &str); 12] = [
        (HeroStatKind::ExpPercent, "0%"),
        (HeroStatKind::BagWeight, "0/0"),
        (HeroStatKind::WearWeight, "0/0"),
        (HeroStatKind::HandWeight, "0/0"),
        (HeroStatKind::MagicResist, "+0"),
        (HeroStatKind::PoisonResist, "+0"),
        (HeroStatKind::HealthRecovery, "+0"),
        (HeroStatKind::SpellRecovery, "+0"),
        (HeroStatKind::PoisonRecovery, "+0"),
        (HeroStatKind::Holy, "+0"),
        (HeroStatKind::Freezing, "+0"),
        (HeroStatKind::PoisonAttack, "+0"),
    ];
    for (i, (kind, text)) in STATE_LABELS.iter().enumerate() {
        spawn_label(
            parent,
            font,
            text,
            HERO_LABEL_X,
            HERO_LABEL_Y0 + i as f32 * HERO_LABEL_DY,
            12.0,
            Color::WHITE,
            10,
        )
        .insert((HeroStatLabel(*kind), HeroPageRoot(HeroPage::State)));
    }
}

/// 状态页/状态二页标签文案（C# `CharacterDialog.cs:96-132` 的格式逐条复刻）
pub fn hero_stat_text(
    kind: HeroStatKind,
    stats: &HeroStatsInfo,
    hp: i32,
    mp: i32,
    max_hp: i32,
    max_mp: i32,
    exp: i64,
    max_exp: i64,
) -> String {
    let pair = |min: i32, max: i32| format!("{min}-{max}");
    let plus = |v: i32| format!("+{v}");
    match kind {
        HeroStatKind::Health => format!("{hp}/{max_hp}"),
        HeroStatKind::Mana => format!("{mp}/{max_mp}"),
        HeroStatKind::Ac => pair(stats.min_ac, stats.max_ac),
        HeroStatKind::Mac => pair(stats.min_mac, stats.max_mac),
        HeroStatKind::Dc => pair(stats.min_dc, stats.max_dc),
        HeroStatKind::Mc => pair(stats.min_mc, stats.max_mc),
        HeroStatKind::Sc => pair(stats.min_sc, stats.max_sc),
        HeroStatKind::CriticalRate => format!("{}%", stats.critical_rate),
        HeroStatKind::CriticalDamage => format!("{}", stats.critical_damage),
        HeroStatKind::AttackSpeed => format!("{}", stats.attack_speed),
        HeroStatKind::Accuracy => plus(stats.accuracy),
        HeroStatKind::Agility => plus(stats.agility),
        HeroStatKind::Luck => format!("{}", stats.luck),
        // C# `string.Format("{0:0.##%}", actor.Experience / (double)actor.MaxExperience)`
        HeroStatKind::ExpPercent => {
            let ratio = if max_exp > 0 {
                exp as f64 / max_exp as f64
            } else {
                0.0
            };
            format!("{:.2}%", ratio * 100.0)
        }
        HeroStatKind::BagWeight => {
            format!("{}/{}", stats.current_bag_weight, stats.max_bag_weight)
        }
        HeroStatKind::WearWeight => {
            format!("{}/{}", stats.current_wear_weight, stats.max_wear_weight)
        }
        HeroStatKind::HandWeight => {
            format!("{}/{}", stats.current_hand_weight, stats.max_hand_weight)
        }
        HeroStatKind::MagicResist => plus(stats.magic_resist),
        HeroStatKind::PoisonResist => plus(stats.poison_resist),
        HeroStatKind::HealthRecovery => plus(stats.health_recovery),
        HeroStatKind::SpellRecovery => plus(stats.spell_recovery),
        HeroStatKind::PoisonRecovery => plus(stats.poison_recovery),
        HeroStatKind::Holy => plus(stats.holy),
        HeroStatKind::Freezing => plus(stats.freezing),
        HeroStatKind::PoisonAttack => plus(stats.poison_attack),
    }
}

/// 页签点击 → 切页 + 窗口路由；页根显隐；状态标签刷新
#[allow(clippy::type_complexity)]
pub fn hero_pages_system(
    mut mgr: ResMut<DialogManager>,
    mut pages: ResMut<HeroPageState>,
    hero: Res<crate::game::dialogs::hero::HeroState>,
    tabs: Query<(Entity, &Interaction, &HeroTabBtn)>,
    // 页签显隐（与点击查询分开：一个读 `Interaction`、一个写 `Visibility`，同一系统里两条查询各自只碰自己的分量）
    // `Without<HeroPageRoot>`：页签与页根是两类实体，加这个过滤后两条查询**可证不相交**
    // （否则都写 `Visibility` → Bevy B0001，实测 `cargo test` 会直接报冲突）。
    mut tabs_vis: Query<(&mut Visibility, &HeroTabBtn), Without<HeroPageRoot>>,
    mut roots: Query<(&HeroPageRoot, &mut Visibility)>,
    mut labels: Query<(&HeroStatLabel, &mut Text)>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
) {
    for (e, inter, tab) in &tabs {
        let was = prev_inter.insert(e, *inter);
        let clicked = *inter == Interaction::Pressed && was != Some(Interaction::Pressed);
        if !clicked {
            continue;
        }
        pages.page = tab.0;
        // #2892 批58：四页同属**一个**英雄对话框窗（C# `CharacterDialog`）→ 就地切页，
        // 只保证窗口处于打开状态
        mgr.open(DialogKind::HeroEquipment);
        tracing::info!("🦸 英雄窗切页 → {:?}", tab.0);
    }

    // 页根显隐：#2892 批58 起四页同属一个英雄窗 → 只显示 `HeroPageState.page` 对应的页
    let page = pages.page;
    // 页签显隐：只有当前页那颗画。依据同角色窗——C# `CharacterDialog.Show*Page()` 把选中那颗设成
    // 自己的帧、其余三颗设 `Index = -1`（**未选中态的 4 个页签已烘在底图 `Title[504]` 里**）。
    // 本端曾 4 颗都用高亮帧常显 ⇒ 英雄窗上出现 4 颗高亮页签。
    let hero_open = mgr.is_open(DialogKind::HeroEquipment);
    for (mut vis, tab) in &mut tabs_vis {
        let want = if hero_open && crate::game::dialogs::character::char_tab_visible(
            hero_page_index(page),
            hero_page_index(tab.0),
        ) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
    for (root, mut vis) in &mut roots {
        let want = if root.0 == page {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }

    // 标签：随 `HeroState.stats`（`S.HeroInformation` 下发）
    let stats = &hero.stats;
    for (label, mut text) in &mut labels {
        let want = hero_stat_text(
            label.0,
            stats,
            hero.hero_hp,
            hero.hero_mp,
            hero.hero_max_hp,
            hero.hero_max_mp,
            hero.hero_exp,
            hero.hero_max_exp,
        );
        if text.0 != want {
            text.0 = want;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> HeroStatsInfo {
        HeroStatsInfo {
            min_ac: 12,
            max_ac: 24,
            min_mac: 8,
            max_mac: 16,
            min_dc: 20,
            max_dc: 40,
            min_mc: 5,
            max_mc: 10,
            min_sc: 3,
            max_sc: 6,
            critical_rate: 15,
            critical_damage: 150,
            attack_speed: 700,
            accuracy: 12,
            agility: 9,
            luck: 3,
            magic_resist: 4,
            poison_resist: 2,
            health_recovery: 5,
            spell_recovery: 6,
            poison_recovery: 0,
            holy: 1,
            freezing: 2,
            poison_attack: 7,
            current_bag_weight: 120,
            current_wear_weight: 80,
            current_hand_weight: 30,
            max_bag_weight: 400,
            max_wear_weight: 200,
            max_hand_weight: 100,
        }
    }

    /// #2892 批57：状态页/状态二页文案逐条对齐 C# `CharacterDialog.cs:96-132`
    ///
    /// 阳性对照：把 `CriticalRate` 的格式改成不带 `%` → 本条断言 FAILED。
    #[test]
    fn hero_stat_texts_match_csharp() {
        let s = sample();
        let t = |k: HeroStatKind| hero_stat_text(k, &s, 600, 100, 800, 200, 8000, 30000);
        assert_eq!(t(HeroStatKind::Health), "600/800");
        assert_eq!(t(HeroStatKind::Mana), "100/200");
        assert_eq!(t(HeroStatKind::Ac), "12-24");
        assert_eq!(t(HeroStatKind::Sc), "3-6");
        assert_eq!(t(HeroStatKind::CriticalRate), "15%");
        assert_eq!(t(HeroStatKind::CriticalDamage), "150");
        assert_eq!(t(HeroStatKind::AttackSpeed), "700");
        assert_eq!(t(HeroStatKind::Accuracy), "+12");
        assert_eq!(t(HeroStatKind::Agility), "+9");
        assert_eq!(t(HeroStatKind::Luck), "3");
        assert_eq!(t(HeroStatKind::ExpPercent), "26.67%");
        assert_eq!(t(HeroStatKind::BagWeight), "120/400");
        assert_eq!(t(HeroStatKind::WearWeight), "80/200");
        assert_eq!(t(HeroStatKind::HandWeight), "30/100");
        assert_eq!(t(HeroStatKind::MagicResist), "+4");
        assert_eq!(t(HeroStatKind::PoisonAttack), "+7");
    }

    /// 页签坐标：C# `Title[500..503]` 64x20 @(8/70/132/194, 70)
    #[test]
    fn hero_tabs_match_csharp() {
        assert_eq!(HERO_TABS[0].3, 8.0);
        assert_eq!(HERO_TABS[1].3, 70.0);
        assert_eq!(HERO_TABS[2].3, 132.0);
        assert_eq!(HERO_TABS[3].3, 194.0);
        assert_eq!(HERO_TAB_Y, 70.0);
        assert_eq!(HERO_TAB_SIZE, (64.0, 20.0));
        assert_eq!((HERO_PAGE_X, HERO_PAGE_Y), (8.0, 90.0));
        // 标签列位与行距（C# x=126、y=20+18i）
        assert_eq!(
            (HERO_LABEL_X, HERO_LABEL_Y0, HERO_LABEL_DY),
            (126.0, 20.0, 18.0)
        );
    }
}
