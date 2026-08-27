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
//   有意偏差：快捷键页沿用本端口 KeyboardState 动态生成（C# 是固定清单 +
//     本地化描述，动作集略有出入）；标题"技能 ({0})"类带占位符的本地化值
//     取其干净前缀
// ============================================================================

use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::game::dialogs::keyboard_layout::{KeyboardState, key_name};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_icon_button, spawn_image, spawn_label, spawn_label_center, spawn_panel,
    ImageButton,
};

/// 背景 Prguse[920] 实测 536x509；Location = Center = ((1024-536)/2, (768-509)/2)
pub const ORIGIN: (f32, f32) = (244.0, 129.0);
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

/// 页码（C# PageLabel，"x / N" 居中 80x20 @(270,490)）
#[derive(Component)]
pub struct HelpPageLabelText;

/// 当前图文页精灵（Help 库图像按页切换）
#[derive(Component)]
pub struct HelpPageImage;

/// 快捷键页两列表头（C# shortcutTitleLabel/infoTitleLabel，对话框坐标
/// 中心 (75,125)/(328,125)——C# 页内 (13,75)/(114,75) + 页偏移 (12,35)；
/// 携带自身文案，图文页清空/快捷键页还原）
#[derive(Component)]
pub struct HelpShortcutHeader(pub &'static str);

/// 快捷键页左列（键名，黄，C# 对话框坐标 (30,142+20i)）
#[derive(Component)]
pub struct HelpShortcutKey(usize);

/// 快捷键页右列（说明，白，C# 对话框坐标 (131,142+20i)）
#[derive(Component)]
pub struct HelpShortcutInfo(usize);

/// 生成快捷键页行（本端口 KeyboardState 动态分组——有意偏差见文件头）
fn shortcut_rows(state: &KeyboardState, groups: &[&str]) -> Vec<(String, String)> {
    state
        .bindings
        .iter()
        .filter(|b| groups.contains(&b.group))
        .map(|b| (key_name(b.key).to_string(), b.action.to_string()))
        .collect()
}

/// 45 页清单：3 快捷键页 + 42 图文页（顺序与 C# LoadImagePages 一致）
fn build_pages(state: &KeyboardState) -> Vec<(String, PageDef)> {
    vec![
        (
            "快捷方式信息".to_string(),
            PageDef::Shortcut(shortcut_rows(state, &["移动", "交互", "界面"])),
        ),
        (
            "快捷方式信息".to_string(),
            PageDef::Shortcut(shortcut_rows(state, &["系统", "技能"])),
        ),
        (
            "聊天快捷键".to_string(),
            PageDef::Shortcut(vec![
                ("/w 名字".to_string(), "私聊对方".to_string()),
                ("!文字".to_string(), "附近喊话".to_string()),
                ("!~文字".to_string(), "行会频道".to_string()),
            ]),
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
        app.add_systems(OnEnter(AppState::Game), spawn_help);
        app.add_systems(OnExit(AppState::Game), cleanup_help);
        app.add_systems(
            Update,
            (help_ui_system,)
                .chain()
                .run_if(in_state(AppState::Game)),
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
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();
    let cjk = shared_cjk_font(&mut fonts, &mut cjk_font);
    let (ox, oy) = ORIGIN;

    // bevy_ui 面板 Prguse[920]（536x509 @ ox,oy）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 920) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, ox, oy, 536.0, 509.0, 30);
    commands.entity(panel).insert((DialogRoot(DialogKind::Help), HelpWidget));

    commands.entity(panel).with_children(|p| {
        // 标题图 Title[57] @(18,9)
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 57) {
            spawn_image(p, h, 18.0, 9.0, 103.0, 17.0, 9);
        }
        // 关闭 [360-362] @(509,3)；Previous @(210,485)；Next @(310,485)
        let buttons: [(HelpBtnKind, usize, usize, usize, f32, f32); 3] = [
            (HelpBtnKind::Close, 360, 361, 362, 509.0, 3.0),
            (HelpBtnKind::Prev, 240, 241, 242, 210.0, 485.0),
            (HelpBtnKind::Next, 243, 244, 245, 310.0, 485.0),
        ];
        for (kind, n, h, pr, rx, ry) in buttons {
            if let (Some(nh), Some(hh), Some(ph)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, n),
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, h),
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, pr),
            ) {
                spawn_icon_button(p, nh, hh, ph, rx, ry, 16.0, 16.0, 10)
                    .insert(HelpBtn(kind));
            }
        }
        // 页标题（居中 @(268,54) 242x30）
        spawn_label_center(p, &font, "", 268.0, 54.0, 242.0, 10.0, Color::WHITE, 9)
            .insert(HelpTitleText);
        // 页码（居中 @(270,490) 80x20）
        spawn_label_center(p, &font, "", 270.0, 490.0, 80.0, 9.0, Color::WHITE, 9)
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
        // 快捷键页两列表头（居中）
        for (text, cx, ry) in [("快捷键", 75.0, 125.0), ("信息", 328.0, 125.0)] {
            spawn_label_center(p, &font, text, cx, ry, 100.0, 10.0, Color::WHITE, 9)
                .insert(HelpShortcutHeader(text));
        }
        // 快捷键页行（黄键名/白说明）
        for i in 0..SHORTCUT_ROWS {
            let y = 142.0 + i as f32 * 20.0;
            spawn_label(p, &cjk, "", 30.0, y, 9.0, Color::srgb(1.0, 1.0, 0.0), 9)
                .insert(HelpShortcutKey(i));
            spawn_label(p, &cjk, "", 131.0, y, 9.0, Color::WHITE, 9)
                .insert(HelpShortcutInfo(i));
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
    use super::*;

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

    /// 快捷键页动态行有内容（键盘默认绑定）
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
}
