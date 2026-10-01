// ============================================================================
// 帮助对话框（M50 → #2602 批R 对齐 C# HelpDialog 45 页翻页册）
// C# HelpDialog（MirScenes/Dialogs/HelpDialog.cs）：
//   - 背景 Prguse[920]（实测 536x509）@ Center=(244,129)；标题图 Title[57] @(18,9)
//   - 45 页循环翻页：3 快捷键页（ShortcutPage1/2/3）+ 42 图文页（Help 库 0..41
//     @ LoadImagePages :108-154 顺序：移动/攻击/拾取/生命/技能×2/法力/聊天/
//     队伍/耐久/购买/出售/修理/交易/查看/统计×6/任务×4/坐骑×2/钓鱼/宝石/
//     英雄×5/公会增益×3/觉醒×5）
//   - 图文页：Help[id] 绘制于页 (12, 35+40)；页标题 Bold10 居中 242x30 @(147,39)
//   - 页码 "n / 45" 9F 居中 80x20 @(230,480)；Previous [240-242] @(210,485) /
//     Next [243-245] @(310,485) 循环；Close [360-362] @(509,3)
//   §3.2de（2026-10-01）：快捷键三页**改成 C# 的固定清单**（`ShortcutPage1/2/3` 的顺序 +
//     `ClientTextKeys` 的中文描述原串），键位列仍按本端**当前绑定**渲染。
//     此前是"按 KeyBinds 分组（移动/交互/界面/系统/技能）动态生成"，与 C# 首行就不同
//     （本端首行「W 向上移动」 vs C# 首行「Alt + Q 退出游戏」）——中文原版帧对表时被抓出来
//     （§3.2dd：Help 行 en-vs-cn 8.68%，是全表唯一两行真·语言差之一）。
//     只列**本端已实现**的动作（C# Page1 的 19 条里本端有 18 条，缺 `TargetSpellLockOn`）；
//     标题"技能 ({0})"类带占位符的本地化值取其干净前缀
// ============================================================================

use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::game::dialogs::keyboard_layout::{KeyboardState, binding_text_for};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont};
use crate::ui::theme::{
    load_lib_image, spawn_icon_button, spawn_image, spawn_image_native, spawn_label,
    spawn_label_center, spawn_panel, CloseButton, ImageButton,
};

/// #2892 批B：面板精灵与 C# 原生尺寸（C# `HelpDialog.Index = 920; Library = Libraries.Prguse`）
pub const PANEL: (LibraryName, usize) = (LibraryName::Prguse, 920);
pub const PANEL_SIZE: (f32, f32) = (536.0, 509.0);
/// C# `HelpDialog.Size` = `GetTrueSize(Prguse[920])` = **533x509**（图头 536x509，右侧 3 列 alpha=0）
/// ⇒ `Location = Center` = `((1024-533)/2, (768-509)/2)` = **(245,129)**。
///
/// 2026-09-30 原版帧实测 `Prguse[920]` @(245,129)（§3.2ce 表里记的 (245,129) 当时被我按
/// "居中窗口径 +1px" 解释掉了——**那条口径是错的**，见 §3.2cl）；本端按图头算成 244。
pub const LAYOUT_SIZE: (f32, f32) = (533.0, 509.0);

/// 背景 Prguse[920] 实测 536x509；Location = Center = ((1024-536)/2, (768-509)/2)
/// C# 是**整数除法**：`(1024-533)/2 = 245`、`(768-509)/2 = 129`（Rust 浮点除会得 245.5/129.5，
/// 少一次 floor 就整窗右下偏 0.5px）。
pub const ORIGIN: (f32, f32) = (245.0, 129.0);
/// 快捷键页行容量（C# ShortcutPage1/2 各 18 行，留 20）
pub const SHORTCUT_ROWS: usize = 20;

/// 42 个图文页（C# LoadImagePages :112-153 的 (标题, Help 库 ImageID) 顺序清单）
pub const IMAGE_PAGES: &[(&str, usize)] = &[
    ("移动", 0),
    ("攻击", 1),
    ("拾取物品", 2),
    ("生命值", 3),
    ("技能", 4),
    ("技能", 5),
    ("法力", 6),
    ("聊天", 7),
    ("队伍", 8),
    ("耐久", 9),
    ("购买", 10),
    ("出售", 11),
    ("修理", 12),
    ("交易", 13),
    ("查看", 14),
    ("统计", 15),
    ("统计", 16),
    ("统计", 17),
    ("统计", 18),
    ("统计", 19),
    ("统计", 20),
    ("任务", 21),
    ("任务", 22),
    ("任务", 23),
    ("任务", 24),
    ("坐骑", 25),
    ("坐骑", 26),
    ("钓鱼", 27),
    ("宝石与宝珠", 28),
    ("英雄", 29),
    ("英雄", 30),
    ("英雄", 31),
    ("英雄", 32),
    ("英雄", 33),
    ("公会增益", 34),
    ("公会增益", 35),
    ("公会增益", 36),
    ("觉醒", 37),
    ("觉醒", 38),
    ("觉醒", 39),
    ("觉醒", 40),
    ("觉醒", 41),
];

/// 按钮种类（单查询分发：关闭/上一页/下一页）
#[derive(Component)]
pub struct HelpBtn(pub HelpBtnKind);

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum HelpBtnKind {
    Close,
    Prev,
    Next,
}

/// 一页的定义（快捷键页动态生成 / 图文页静态清单）
enum PageDef {
    Shortcut(Vec<(String, String)>),
    Image(usize),
}

/// 状态
#[derive(Resource, Default)]
pub struct HelpState {
    pub message: String,
    /// 当前页（C# HelpDialog CurrentPageNumber）
    pub page: usize,
}

#[derive(Component)]
pub struct HelpWidget;

#[derive(Component)]
pub struct HelpClose;

/// 上一页（C# PreviousButton Prguse2[240-242] @(210,485)）
#[derive(Component)]
pub struct HelpPrev;

/// 下一页（C# NextButton Prguse2[243-245] @(310,485)）
#[derive(Component)]
pub struct HelpNext;

/// 页标题（C# PageTitleLabel：Bold10 居中 242x30 @(147,39)）
#[derive(Component)]
pub struct HelpTitleText;

/// 页标题的**节点顶**（面板内）。C# `PageTitleLabel`：`Size=(242,30)` +
/// `HorizontalCenter | VerticalCenter`（`HelpDialog.cs:385-393`），`HelpPage` 内在 `(135,4)`
/// ⇒ 文本盒面板内 `[39.5,69.5]`、中心 **54.5**。本端 13px 档墨迹顶 = 节点顶 + 1
/// ⇒ 取 48 时墨迹 49..60（中心 54.5）。
pub const HELP_TITLE_Y: f32 = 48.0;

/// 页码的**节点顶**（面板内）。C# `PageLabel`：`Size=(80,20)` + 同样 H+V 居中
/// （`HelpDialog.cs:74-83`，`Location=(230,480)`）⇒ 文本盒 `[481,501]`、中心 **491**。
/// 本端 12px 档墨迹顶 = 节点顶 + 3 ⇒ 取 484 时墨迹 487..495。
pub const HELP_PAGE_Y: f32 = 484.0;

/// 页码（C# PageLabel，"x / N" 居中 80x20 @(270,490)）
#[derive(Component)]
pub struct HelpPageLabelText;

/// 当前图文页精灵（Help 库图像按页切换）
#[derive(Component)]
pub struct HelpPageImage;

/// 快捷键页两列表头（C# `shortcutTitleLabel/infoTitleLabel`：
/// `ShortcutInfoPage` 里 `Location=(13,75)/(114,75)`、`Size=(100,30)/(400,30)` 且居中绘制
/// ⇒ **对话框坐标**中心 `(63,90)/(314,90)`）。
///
/// §3.2de：`ShortcutInfoPage` 的 `Parent` 是 `HelpDialog`（`HelpDialog.cs:109-111`）、自身 `Location` 为默认
/// `(0,0)` —— 那个 `(12,35)` 只是 `HelpPage` **包装层**（图文页用）的位置，**快捷键页内容不加它**。
/// 本端此前把表头/行都按 +12/+35 平移，实机帧里表头低 45px、行低 29px（对中文原版量得）。
#[derive(Component)]
pub struct HelpShortcutHeader(pub &'static str);

/// 快捷键页左列（键名，黄，C# 对话框坐标 (30,142+20i)）
#[derive(Component)]
pub struct HelpShortcutKey(usize);

/// 快捷键页右列（说明，白，C# 对话框坐标 (131,142+20i)）
#[derive(Component)]
pub struct HelpShortcutInfo(usize);

/// C# 三页快捷键清单的**一条** = `(本端键位动作名, C# 的 `ClientTextKeys` 名, 描述原串)`。
///
/// 键位列由 [`crate::game::dialogs::keyboard_layout::binding_text_for`] 按**当前绑定**渲染
/// （与 C# `CMain.InputKeys.GetKey(KeybindOptions.X)` 同口径，含修饰键）；描述列用
/// `Client/Localization/Chinese.json` 里 `ClientTextKeys.<名>` 的**原串**（逐字复制）；
/// 中间那项只作溯源（运行期不读）。
type ShortcutRow = (&'static str, &'static str, &'static str);

/// C# `ShortcutPage1`（`HelpDialog.cs:211-240`）的 **18 条**，**按 C# 顺序**。
///
/// §3.2de：只保留**本端已实现**的动作（缺 `TargetSpellLockOn`：本端没有"把法术锁定在目标"的键位）⇒ 17 条。
/// 描述串逐字取自 `Client/Localization/Chinese.json`。
const SHORTCUT_PAGE1: &[ShortcutRow] = &[
    ("退出", "ExitGame", "退出游戏"),
    ("下线", "LogOut", "登出"),
    ("技能栏1", "SkillButtons", "技能按钮"),
    ("背包", "InventoryWindowOpenClose", "背包（打开/关闭）"),
    ("角色", "StatusWindowOpenClose", "状态（打开/关闭）"),
    ("技能", "SkillWindowOpenClose", "技能（打开/关闭）"),
    ("队伍", "GroupWindowOpenClose", "队伍（打开/关闭）"),
    ("请求交易", "TradeWindowOpenClose", "交易（打开/关闭）"),
    ("好友", "FriendWindowOpenClose", "好友（打开/关闭）"),
    ("小地图", "MinimapWindowOpenClose", "小地图（打开/关闭）"),
    ("行会", "GuildWindowOpenClose", "公会（打开/关闭）"),
    ("商城", "GameshopWindowOpenClose", "商城（打开/关闭）"),
    ("夫妻", "EngagementWindowOpenClose", "婚姻（打开/关闭）"),
    ("腰带", "BeltWindowOpenClose", "快捷栏（打开/关闭）"),
    ("设置", "OptionWindowOpenClose", "选项（打开 / 关闭）"),
    ("帮助", "HelpWindowOpenClose", "帮助（打开 / 关闭）"),
    ("坐骑切换", "MountDismountRide", "骑乘 / 下马"),
];

/// C# `ShortcutPage2`（`HelpDialog.cs:241-271`）的 **18 条**，**按 C# 顺序**，同样只留本端已实现的动作（7 条）。
///
/// 未实现（故不列）：`ChangeAttackmode`（攻击模式切换，本端在 `combat.rs` 里硬编码，不在键位表）、
/// 四个 `Attackmode*`（和平/组队/公会/善恶/全体）、`Autorun`、`Cameramode`、`Screenshot`、
/// `Mentor`、`CtrlRightClick`。
const SHORTCUT_PAGE2: &[ShortcutRow] = &[
    ("宠物模式切换", "TogglePetAttackPet", "切换宠物攻击宠物"),
    ("大地图", "ShowFieldMap", "显示区域地图"),
    ("技能栏显隐", "ShowSkillBar", "显示技能栏"),
    ("拾取", "HighlightPickupItems", "高亮 / 捡取物品"),
    ("钓鱼", "OpenCloseFishingWindow", "钓鱼（打开 / 关闭）"),
    (
        "宠物拾取",
        "CreaturePickupMultiMouseTarget",
        "宠物拾取（多鼠标目标）",
    ),
    (
        "宠物半自动拾取",
        "CreaturePickupSingleMouseTarget",
        "宠物拾取（单鼠标目标）",
    ),
];

/// C# `ShortcutPage3`（`HelpDialog.cs:272-286`）：三行**聊天命令**（键位列 C# 是固定串，不是绑定）。
const CHAT_COMMAND_ROWS: &[(&str, &str)] = &[
    ("/(username)", "私聊命令"),
    ("!(text)", "附近喊话命令"),
    ("!~(text)", "公会聊天命令"),
];

/// 把 C# 清单渲染成本端行：键位列取**当前绑定**（`技能栏1..8` 特判成 `F1-F8` 这种跨度写法，
/// 与 C# `GetKey(Bar1Skill1) + "-" + GetKey(Bar1Skill8)` 同构）。
fn shortcut_rows(state: &KeyboardState, rows_list: &[ShortcutRow]) -> Vec<(String, String)> {
    rows_list
        .iter()
        .map(|r| {
            let action = r.0;
            let key = if action == "技能栏1" {
                let first = binding_text_for(&state.bindings, "技能栏1");
                let last = binding_text_for(&state.bindings, "技能栏8");
                if first.is_empty() || last.is_empty() {
                    first
                } else {
                    format!("{first}-{last}")
                }
            } else {
                binding_text_for(&state.bindings, action)
            };
            (key, r.2.to_string())
        })
        .collect()
}

/// 45 页清单：3 快捷键页 + 42 图文页（顺序与 C# LoadImagePages 一致）
fn build_pages(state: &KeyboardState) -> Vec<(String, PageDef)> {
    vec![
        (
            "快捷方式信息".to_string(),
            PageDef::Shortcut(shortcut_rows(state, SHORTCUT_PAGE1)),
        ),
        (
            "快捷方式信息".to_string(),
            PageDef::Shortcut(shortcut_rows(state, SHORTCUT_PAGE2)),
        ),
        (
            "聊天快捷键".to_string(),
            PageDef::Shortcut(
                CHAT_COMMAND_ROWS
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
            ),
        ),
    ]
    .into_iter()
    .chain(
        IMAGE_PAGES
            .iter()
            .map(|(title, id)| (title.to_string(), PageDef::Image(*id))),
    )
    .collect()
}

pub struct HelpPlugin;

impl Plugin for HelpPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HelpState>();
        app.init_resource::<UiCjkFont>();
        app.add_systems(OnEnter(AppState::Game), spawn_help);
        app.add_systems(OnExit(AppState::Game), cleanup_help);
        app.add_systems(
            Update,
            (help_ui_system,).chain().run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_help(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_help(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut cjk_font: ResMut<UiCjkFont>,
) {
    libs.0.ensure_initialized();
    // 上列三处（页标题/页码/表头）此前走 Arial 主字体 → 中文全是豆腐（实机截图确认：
    // 列头 `□□□`/`□□`、标题 `1. □□□□□`）。本模块其余文本早已是 `&cjk`，此处补齐；
    // `ui_font` 随之不再需要（Arial 无 CJK 字形，且本环境实测 Han 回退对静态文本同样不生效）。
    let cjk = shared_cjk_font(&mut fonts, &mut cjk_font);
    let (ox, oy) = ORIGIN;

    // bevy_ui 面板 Prguse[920]（536x509 @ ox,oy）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 920) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, ox, oy, PANEL_SIZE.0, PANEL_SIZE.1, 30);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::Help), HelpWidget));

    commands.entity(panel).with_children(|p| {
        // 标题图 Title[57] @(18,9)：不设 Size ⇒ 美术原生 **45x14**
        // （曾写死 103x17，把 HELP 标题拉伸成 QUEST DIARY 那个尺寸）
        let _ = spawn_image_native(
            p,
            &mut libs,
            &mut images,
            LibraryName::Title,
            57,
            18.0,
            9.0,
            9,
        );
        // 关闭 [360-362] @(509,3)；Previous @(210,485)；Next @(310,485)
        let buttons: [(HelpBtnKind, usize, usize, usize, f32, f32); 3] = [
            (HelpBtnKind::Close, 360, 361, 362, 509.0, 3.0),
            (HelpBtnKind::Prev, 240, 241, 242, 210.0, 485.0),
            (HelpBtnKind::Next, 243, 244, 245, 310.0, 485.0),
        ];
        for (kind, n, h, pr, rx, ry) in buttons {
            // 尺寸取**美术原生尺寸**：`Prguse2[240..245]`（翻页箭头）= **16x16**，
            // 而 `Prguse2[360..362]`（关闭钮）= **24x21**。旧代码三颗都写死 16x16
            // ⇒ 关闭钮被压扁（`control_size_audit.py` 新加的第 4 个扫描面捞到的第二处）。
            let (bw, bh) = libs
                .0
                .get_image(LibraryName::Prguse2, n)
                .map(|i| (i.width.max(0) as f32, i.height.max(0) as f32))
                .unwrap_or((16.0, 16.0));
            if let (Some(nh), Some(hh), Some(ph)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, n),
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, h),
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, pr),
            ) {
                let mut ec = spawn_icon_button(p, nh, hh, ph, rx, ry, bw, bh, 10);
                ec.insert(HelpBtn(kind));
                if matches!(kind, HelpBtnKind::Close) {
                    ec.insert(CloseButton);
                }
            }
        }
        // 页标题（居中 @(268,54) 242x30）
        // §3.2de：字号口径 —— C# 是 **pt**（`new Font(Settings.FontName, 10F, Bold)`），
        // 本端 `spawn_label*` 的 size 是 **px**，96 DPI 下 1pt = 4/3 px ⇒ 10F≈13px、9F≈12px。
        // 此前直接照抄 pt 数值（10/9），实机帧里字形带只有原版的一半高（Help 行 6px vs 原版 9px）。
        //
        // §3.2dn：**垂直居中**——C# `PageTitleLabel` 是 `Size=(242,30)` + `VerticalCenter`
        // （`HelpDialog.cs:385-393`，`HelpPage` 内 `Location=(135,4)`），文本盒面板内
        // `[39.5,69.5]`、中心 54.5；本端 13px 档墨迹顶 = 节点顶 + 1 ⇒ 节点顶取 48 时墨迹
        // 落在 49..60（中心 54.5）。帧证：原版墨迹绝对行 179..188（中心 183.5），
        // 本端修前 184..195（中心 189.5）——**低 6px**。
        spawn_label_center(
            p,
            &cjk,
            "",
            268.0,
            HELP_TITLE_Y,
            242.0,
            13.0,
            Color::WHITE,
            9,
        )
        .insert(HelpTitleText);
        // 页码（居中 @(270,490) 80x20）
        // §3.2dn：C# `PageLabel` 是 `Size=(80,20)` + `VerticalCenter`（`HelpDialog.cs:74-83`）
        // ⇒ 文本盒面板内 `[481,501]`、中心 491；本端 12px 档墨迹顶 = 节点顶 + 3 ⇒ 节点顶 484
        // 时墨迹落在 487..495（帧证：原版绝对 616..624，本端修前 622..630——**低 6px**）。
        spawn_label_center(p, &cjk, "", 270.0, HELP_PAGE_Y, 80.0, 12.0, Color::WHITE, 9)
            .insert(HelpPageLabelText);
        // 图文页图像（@(12,75)，Auto 尺寸）
        let white = images.add(crate::map_renderer::make_image(
            vec![255, 255, 255, 255],
            1,
            1,
        ));
        p.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(12.0),
                top: Val::Px(75.0),
                ..default()
            },
            ImageNode::new(white),
            HelpPageImage,
            Visibility::Hidden,
            ZIndex(8),
        ));
        // 快捷键页两列表头（居中；C# 对话框坐标中心 (63,90)/(314,90)，见 `HelpShortcutHeader` 注释）
        // `spawn_label_center` 的 y 是**顶**；C# 表头是 `Size=(100,30)` + `VerticalCenter`
        // ⇒ 文字中心落在盒中心 90 ⇒ 本端顶 = 90 - 字形高/2 ≈ 83（实机帧里表头带 212-225）。
        for (text, cx, ry) in [("快捷键", 63.0, 83.0), ("信息", 314.0, 83.0)] {
            spawn_label_center(p, &cjk, text, cx, ry, 100.0, 13.0, Color::WHITE, 9)
                .insert(HelpShortcutHeader(text));
        }
        // 快捷键页行（黄键名/白说明）：C# `ShortcutInfoPage.LoadKeyBinds` 的
        // `Location = (18, 107 + 20*i)` / `(119, 107 + 20*i)`（`HelpDialog.cs:347/359`，对话框坐标）
        // C# 行标签 `Size=(95,23)` + `VerticalCenter` ⇒ 文字中心 = 107 + 11.5 = 118.5；
        // 本端 `spawn_label` 是顶对齐 ⇒ 顶 = 118.5 - 9/2 ≈ 114（实机帧里黄字带 244-252）。
        for i in 0..SHORTCUT_ROWS {
            let y = 114.0 + i as f32 * 20.0;
            spawn_label(p, &cjk, "", 18.0, y, 12.0, Color::srgb(1.0, 1.0, 0.0), 9)
                .insert(HelpShortcutKey(i));
            spawn_label(p, &cjk, "", 119.0, y, 12.0, Color::WHITE, 9).insert(HelpShortcutInfo(i));
        }
    });
}

/// 显隐 + 分页渲染 + 关闭
#[allow(clippy::type_complexity)]
#[allow(clippy::type_complexity)]
fn help_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut help: ResMut<HelpState>,
    keyboard: Res<KeyboardState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    buttons: Query<(Entity, &Interaction, &HelpBtn)>,
    mut widgets: Query<&mut Visibility, (With<HelpWidget>, Without<HelpPageImage>)>,
    mut page_img: Query<(&mut ImageNode, &mut Visibility), With<HelpPageImage>>,
    mut titles: Query<
        &mut Text,
        (
            With<HelpTitleText>,
            Without<HelpPageLabelText>,
            Without<HelpShortcutKey>,
            Without<HelpShortcutInfo>,
        ),
    >,
    mut page_labels: Query<
        &mut Text,
        (
            With<HelpPageLabelText>,
            Without<HelpTitleText>,
            Without<HelpShortcutKey>,
            Without<HelpShortcutInfo>,
        ),
    >,
    mut keys: Query<
        (&mut Text, &HelpShortcutKey),
        (
            Without<HelpTitleText>,
            Without<HelpPageLabelText>,
            Without<HelpShortcutHeader>,
            Without<HelpShortcutInfo>,
        ),
    >,
    mut infos: Query<
        (&mut Text, &HelpShortcutInfo),
        (
            Without<HelpTitleText>,
            Without<HelpPageLabelText>,
            Without<HelpShortcutHeader>,
            Without<HelpShortcutKey>,
        ),
    >,
    mut headers: Query<
        (&mut Text, &HelpShortcutHeader),
        (
            Without<HelpTitleText>,
            Without<HelpPageLabelText>,
            Without<HelpShortcutKey>,
            Without<HelpShortcutInfo>,
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

    let open = mgr.is_open(DialogKind::Help);
    for mut vis in widgets.iter_mut() {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        if let Ok((_, mut vis)) = page_img.single_mut() {
            *vis = Visibility::Hidden;
        }
        return;
    }
    let pages = build_pages(&keyboard);
    let total = pages.len();
    if help.page >= total {
        help.page = 0;
    }
    // 关闭 / 循环翻页（bevy_ui Interaction 边沿触发）
    for (e, inter, k) in buttons.iter() {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        match k.0 {
            HelpBtnKind::Close => mgr.close(DialogKind::Help),
            HelpBtnKind::Prev => {
                help.page = if help.page == 0 {
                    total - 1
                } else {
                    help.page - 1
                };
            }
            HelpBtnKind::Next => help.page = (help.page + 1) % total,
        }
    }
    let (title, def) = &pages[help.page];
    for mut t in &mut titles {
        t.0 = format!("{}. {}", help.page + 1, title);
    }
    for mut t in &mut page_labels {
        t.0 = format!("{} / {}", help.page + 1, total);
    }
    match def {
        PageDef::Image(id) => {
            if let Ok((mut node, mut vis)) = page_img.single_mut() {
                if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Help, *id) {
                    node.image = h;
                }
                *vis = Visibility::Visible;
            }
            for (mut text, _) in &mut keys {
                text.0 = String::new();
            }
            for (mut text, _) in &mut infos {
                text.0 = String::new();
            }
            for (mut text, _) in &mut headers {
                text.0 = String::new();
            }
        }
        PageDef::Shortcut(rows) => {
            if let Ok((_, mut vis)) = page_img.single_mut() {
                *vis = Visibility::Hidden;
            }
            for (mut text, row) in &mut keys {
                text.0 = rows.get(row.0).map(|r| r.0.clone()).unwrap_or_default();
            }
            for (mut text, row) in &mut infos {
                text.0 = rows.get(row.0).map(|r| r.1.clone()).unwrap_or_default();
            }
            for (mut text, h) in &mut headers {
                text.0 = h.0.to_string();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    /// §3.2dn：Help 两处标签都按 C# 的 `VerticalCenter` 盒落位
    /// （`PageTitleLabel` 242x30、`PageLabel` 80x20，`HelpDialog.cs:74-83/385-393`）。
    ///
    /// 判据：本端节点顶 + 墨迹偏移 + 墨迹高/2 = C# 盒中心。
    /// 阳性对照（实做）：把任一常量改回修前的 54 / 490 ⇒ 本测试红。
    #[test]
    fn help_labels_are_vertically_centered_in_csharp_boxes() {
        // 标题：盒中心 54.5，本端 13px 墨迹高 12、墨迹顶 = 节点顶 + 1
        assert_eq!(
            HELP_TITLE_Y + 1.0 + 12.0 / 2.0,
            55.0,
            "标题墨迹中心 = 54.5±0.5"
        );
        // 页码：盒中心 491，本端 12px 墨迹高 9、墨迹顶 = 节点顶 + 3
        assert_eq!(
            HELP_PAGE_Y + 3.0 + 9.0 / 2.0,
            491.5,
            "页码墨迹中心 = 491±0.5"
        );
        assert_ne!(HELP_TITLE_Y, 54.0, "修前值（低 6px）不得复活");
        assert_ne!(HELP_PAGE_Y, 490.0, "修前值（低 6px）不得复活");
    }

    use super::*;

    /// #2985 A2：帮助面板**页标题/页码/表头**必须用自带 CJK 字形的主字体。此前这三处
    /// 走 Arial（表体早已是宋体），实机截图列头 `□□□`/`□□`、标题 `1. □□□□□` 是豆腐。
    /// 修复前本测试 FAILED。
    #[test]
    fn help_text_uses_cjk_capable_font() {
        use bevy::ecs::system::RunSystemOnce;

        // CI 无游戏资产（`Data/` 不入库）→ spawn 在取背景图处提前返回、一个文本都没有，
        // 断言会假红（本 PR 自己的 CI 就是这么红的）；按 `data_assets_present` 跳过。
        if !crate::resources::libraries::data_assets_present() {
            eprintln!("skip: 无 Data 资产（CI 只 checkout 仓库）");
            return;
        }
        let mut world = World::new();
        world.insert_resource(GameLibraries::default());
        world.insert_resource(Assets::<Image>::default());
        world.insert_resource(Assets::<Font>::default());
        world.insert_resource(UiCjkFont::default());
        world.run_system_once(spawn_help).unwrap();

        let cjk = world.resource::<UiCjkFont>().0.clone();
        assert!(cjk.is_strong(), "CJK 字体应已被惰性加载");
        let mut n = 0usize;
        // 这些面板走 `spawn_label` → bevy_ui 的 `Text`（不是 `Text2d`），
        // 两者都用 `TextFont` 携带字体句柄，故只查后者即可全覆盖
        let mut q = world.query::<&TextFont>();
        for tf in q.iter(&world) {
            match &tf.font {
                FontSource::Handle(h) => {
                    assert_eq!(
                        *h, cjk,
                        "帮助面板文本必须用自带 CJK 的主字体（Arial 会豆腐）"
                    );
                    n += 1;
                }
                // 非 Handle 源会被 `if let` 静默跳过（假绿口子）：这类实体同样
                // 渲染中文，出现即失败
                other => panic!("帮助面板文本应为显式字体句柄，实得 {other:?}"),
            }
        }
        assert!(n > 0, "应至少 spawn 出若干文本实体");
    }

    /// 45 页 = 3 快捷键 + 42 图文；图文页 ImageID 与 C# 清单逐项一致
    #[test]
    fn pages_match_csharp_list() {
        let kb = KeyboardState::default();
        let pages = build_pages(&kb);
        assert_eq!(pages.len(), 45);
        // 前 3 页标题（C# 本地化）
        assert_eq!(pages[0].0, "快捷方式信息");
        assert_eq!(pages[1].0, "快捷方式信息");
        assert_eq!(pages[2].0, "聊天快捷键");
        // 42 图文页 (标题, id) 与 C# LoadImagePages 顺序一致
        for (i, (title, id)) in IMAGE_PAGES.iter().enumerate() {
            let (t, def) = &pages[3 + i];
            assert_eq!(t, title, "图文页 {i} 标题");
            match def {
                PageDef::Image(got) => assert_eq!(*got, *id, "图文页 {i} ImageID"),
                _ => panic!("图文页 {i} 应为 Image"),
            }
        }
        // 首末页身份（C# :112/:153）
        assert_eq!(IMAGE_PAGES.first().unwrap(), &("移动", 0));
        assert_eq!(IMAGE_PAGES.last().unwrap(), &("觉醒", 41));
        // 英雄×5 / 公会增益×3 / 觉醒×5 / 统计×6 / 任务×4 页数与 C# 一致
        let count = |t: &str| IMAGE_PAGES.iter().filter(|(x, _)| *x == t).count();
        assert_eq!(count("英雄"), 5);
        assert_eq!(count("公会增益"), 3);
        assert_eq!(count("觉醒"), 5);
        assert_eq!(count("统计"), 6);
        assert_eq!(count("任务"), 4);
        // ImageID 0..41 无重复
        let mut ids: Vec<usize> = IMAGE_PAGES.iter().map(|(_, id)| *id).collect();
        ids.sort();
        assert_eq!(ids, (0..42).collect::<Vec<_>>());
    }

    /// 快捷键页行有内容（键盘默认绑定）
    #[test]
    fn shortcut_pages_have_rows() {
        let kb = KeyboardState::default();
        let pages = build_pages(&kb);
        for i in 0..3 {
            if let PageDef::Shortcut(rows) = &pages[i].1 {
                assert!(!rows.is_empty() || i == 2, "页 {i} 行非空");
            }
        }
    }

    /// §3.2de：快捷键三页改成 **C# `ShortcutPage1/2/3` 的固定清单**（顺序 + 中文描述原串）。
    ///
    /// 阳性对照（实做）：把 `SHORTCUT_PAGE1` 的第一条换回"按分组动态生成"的产物
    /// （首行会是「W / 向上移动」）⇒ 本测试红。
    #[test]
    fn shortcut_pages_match_csharp_fixed_lists() {
        let kb = KeyboardState::default();
        let pages = build_pages(&kb);
        let rows_of = |i: usize| match &pages[i].1 {
            PageDef::Shortcut(r) => r.clone(),
            _ => panic!("页 {i} 应为 Shortcut"),
        };
        let p1 = rows_of(0);
        let p2 = rows_of(1);
        let p3 = rows_of(2);
        // ① 条数：C# Page1 18 条里本端实现 17（缺 TargetSpellLockOn）；Page2 实现 7；Page3 = 3 条命令
        assert_eq!(p1.len(), 17, "Page1 行数（C# 18 - 本端未实现的 1）");
        assert_eq!(p2.len(), 7, "Page2 行数（只列本端已实现的动作）");
        assert_eq!(p3.len(), 3, "Page3 = C# 三条聊天命令");
        // ② 首/末行与 C# 同序（C# Page1 首行 Exit、末行 TargetSpellLockOn；本端去掉后者）
        assert_eq!(
            p1[0].1, "退出游戏",
            "Page1 首行文案 = ClientTextKeys.ExitGame"
        );
        assert_eq!(
            p1[0].0, "Alt + Q",
            "Page1 首行键位 = C# GetKey(Exit)（Alt+Q）"
        );
        assert_eq!(
            p1.last().unwrap().1,
            "骑乘 / 下马",
            "Page1 末行 = MountDismountRide（本端去掉其后的 TargetSpellLockOn）"
        );
        // ③ 技能栏跨度写法（C# `GetKey(Bar1Skill1) + "-" + GetKey(Bar1Skill8)`）
        let skill_span = p1.iter().find(|(_, info)| info == "技能按钮").unwrap();
        assert_eq!(skill_span.0, "F1-F8", "技能按钮键位列 = F1-F8");
        // ③b Page2 首行 = C# `TogglePetAttackPet`；键位是本端绑定（`宠物模式切换` 本端有意从
        //     Ctrl+A 改到 Ctrl+T，见 keyboard_layout.rs 的 #1562 注释）
        assert_eq!(
            p2[0].1, "切换宠物攻击宠物",
            "Page2 首行 = ClientTextKeys.TogglePetAttackPet"
        );
        assert_eq!(
            p2[0].0, "Ctrl + T",
            "Page2 首行键位取本端绑定（Ctrl+T 为有意偏离）"
        );
        // ④ Page3 三条命令（C# 的固定串，不是绑定）
        assert_eq!(p3[0], ("/(username)".to_string(), "私聊命令".to_string()));
        assert_eq!(p3[2], ("!~(text)".to_string(), "公会聊天命令".to_string()));
        // ⑤ 旧口径不得回归：Page1 首行不能再是"按分组动态生成"的移动类动作
        assert_ne!(p1[0].1, "向上移动", "旧口径（按分组动态生成）已废");
    }
}
