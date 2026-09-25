// ============================================================================
// 对话框系统（M9）
// 交互参考：Client/MirScenes/Dialogs/*.cs（原版 C#）
// 绘制参考：Client-Macroquad/src/scenes/dialogs/game/*.rs
// 框架：DialogManager 维护打开栈（z 序），每个对话框一个插件子模块
// ============================================================================

pub mod amount_box;
pub mod assign_key;
pub mod big_map;
pub mod buff;
pub mod character;
pub mod chat_notice;
pub mod compass;
pub mod craft;
pub mod creature;
pub mod dura_status;
pub mod fishing;
pub mod friend;
pub mod game_shop;
pub mod group;
pub mod guild;
pub mod guild_territory;
pub mod help;
pub mod hero;
pub mod hero_belt;
pub mod hero_equipment;
pub mod hero_inventory;
pub mod hero_pages;
pub mod hero_skills;
/// #2892 批C：C# `MirInputBox`（服务端发起式取名提示框）
pub mod input_box;
pub mod inspect;
pub mod inventory;
pub mod item_rental;
/// #2720：C# `ItemRentalDialog`（浏览已租出物品；与出租流程 `item_rental` 分开）
pub mod item_rental_browse;
pub mod keyboard_layout;
pub mod mail;
pub mod market;
pub mod market_filter;
pub mod mentor;
pub mod menu;
pub mod minimap;
pub mod mount;
pub mod notice;
pub mod npc;
pub mod npc_awake;
pub mod npc_goods;
pub mod option;
pub mod potion_belt;
pub mod quest_log;
pub mod quest_tracking;
pub mod ranking;
pub mod refine;
pub mod relationship;
pub mod report;
pub mod roll;
pub mod sell_panel;
pub mod settings_file;
pub mod skill_desc;
pub mod socket;
pub mod storage;
pub mod text_input;
pub mod timer;
pub mod trade;
// #2892 批D 单元①：C# `MirControl.Movable` 窗口拖动（腰带/聊天/备注/钓鱼/下拉框）
/// #2892 批D 单元①：C# `MemoDialog`（好友备注窗）
pub mod memo;
pub mod window_drag;

use bevy::prelude::*;

use crate::game::dialogs::text_input::TextInputRect;
use crate::scenes::AppState;
use crate::ui::theme::UiDropDown;

/// 对话框类型。
///
/// **新增变体时**：本枚举带 `Reflect` 派生，`tests/ui_alignment.rs` 的
/// `dialog_kind_registry_covers_all_variants` 会拿 `EnumInfo::variant_names()` 与登记表
/// `ALL_DIALOG_KINDS` 对账——只加变体不登记会红。`control.rs::has_rpc_mapping` 与
/// `ui_alignment::kind_alignment_tests` 是无通配 `match`，漏分类会编译失败。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Reflect)]
pub enum DialogKind {
    Inventory,
    Character,
    QuestLog,
    Settings,
    Menu,
    GameShop,
    Minimap,
    Npc,
    Group,
    Friend,
    Trade,
    /// 对方交易窗（C# GuestTradeDialog——独立窗、独立拖动，与 Trade 成对显隐）
    GuestTrade,
    Inspect,
    NpcGoods,
    Guild,
    Mail,
    Ranking,
    Mentor,
    Relationship,
    Mount,
    Report,
    HeroInventory,
    HeroEquipment,
    Creature,
    ItemRental,
    GuildTerritory,
    Help,
    Notice,
    Buff,
    Fishing,
    Socket,
    Refine,
    Craft,
    DuraStatus,
    Roll,
    NpcAwake,
    Timer,
    KeyboardLayout,
    BigMap,
    ChatNotice,
    Market,
    Storage,
    /// #2720：C# `ItemRentalDialog`（浏览已租出物品列表；与出租流程 `ItemRental` 分开）
    ItemRentalBrowse,
    /// #2791：C# `HeroManageDialog`（`Prguse[1688]` 独立窗、独立拖动；随 `S.ManageHeroes`
    /// 弹出，关闭键/ESC 隐藏）。不复用 `Hero`——拖动与置顶按 kind 聚合，
    /// 复用会让拖英雄窗连带移动它（C# 两窗各自 `Movable`）
    HeroManage,
    /// #2801：C# `QuestDetailDialog`（任务详情窗，`Prguse[960]` 316x466 @(532,60)，
    /// `QuestDialogs.cs:463-628`）。由任务日记行左键打开（`QuestSingleQuestItem._questLabel.Click`，
    /// `QuestDialogs.cs:1928-1935`）；`Movable = true` 独立拖动，故独立 kind 不复用 `QuestLog`
    QuestDetail,
    /// #2892 批C：C# `MirInputBox`（玩家取名提示框，`Prguse[660]` 288x156 居中，
    /// `Modal = true` / `Movable = false`）。服务端发起：`S.GuildNameRequest`（行会取名）、
    /// `S.GuildRequestWar`（宣战目标行会名，`GameScene.cs:5784-5802`）。
    /// 独立 kind：`Modal` 语义需要「在世界输入锁里算一个打开窗口」（`blocks_world_click`）。
    InputBox,
    /// #2892 批D 单元①：C# `MemoDialog`（好友备注窗，`Title[209]` 196x166 居中、
    /// `Movable = true`，`FriendDialog.cs:480-568`）。独立 kind：C# 是**独立可拖小窗**，
    /// 本端此前把备注做成好友窗内嵌输入框，按 C# 拆窗后拖动归一 kind 聚合。
    Memo,
    /// #2892/#2926：C# `FishingStatusDialog`（钓鱼状态窗，`Prguse[1341]` 244x128 @(390,300)，
    /// `Movable = true`）。与 `Fishing`（带钓具格的主窗 `Prguse[1340]`）**成对显隐但各自独立拖动**
    /// —— 共用 kind 会让拖动一个时把另一个也拖走（C# 两窗各自 `Movable`）。
    FishingStatus,
    /// #3103：C# 两张**写邮件**窗（`MailComposeLetterDialog` `Title[671]` 236x300 @(100,100)、
    /// `MailComposeParcelDialog` `Title[674]` 236x384 @(背包宽+10,0)，均 `Movable = true`，
    /// `MailDialogs.cs:596-1010`）。独立 kind：C# 里它们是**独立可拖窗**，与邮件列表窗
    /// （`MailListDialog` `Title[670]`）各自拖动——共用 `Mail` 会让拖邮件列表时把写邮件窗一起拖走
    /// （owner 2026-09-24 截图里那块"飘在世界中间的写邮件面板"就是这么来的）。
    MailCompose,
    /// #3103 读侧：C# 两张**读**邮件窗（`MailReadLetterDialog` `Title[672]` 236x300 @(100,100)、
    /// `MailReadParcelDialog` `Title[675]` @(100,100)，均 `Movable = true` / `Sort = true`，
    /// `MailDialogs.cs:979-1272`）。独立 kind 的理由同 `MailCompose`：C# 里读信窗与邮件列表窗
    /// 各自 `Movable`，共用 kind 会让拖一个把另一个也拖走。
    MailRead,
}

/// 对话框管理（打开栈，栈顶在最前）
#[derive(Resource, Default)]
pub struct DialogManager {
    pub open: Vec<DialogKind>,
}

/// #2836 单元③：C# `KeybindOptions.Closeall`（ESC）**直接** Hide 的窗口
/// （`Client/MirScenes/GameScene.cs:669-708`）。
///
/// 逐条对应（C# → Bevy kind）：InventoryDialog→Inventory、CharacterDialog→Character、
/// OptionDialog→Settings、MenuDialog→Menu、NPCDialog→Npc、HelpDialog→Help、
/// KeyboardLayoutDialog→KeyboardLayout、RankingDialog→Ranking、
/// IntelligentCreatureDialog（+Options/Grade 两个子窗）→Creature、MountDialog→Mount、
/// FishingDialog→Fishing、FriendDialog→Friend、RelationshipDialog→Relationship、
/// MentorDialog→Mentor、GameShopDialog→GameShop、GroupDialog→Group、GuildDialog→Guild、
/// InspectDialog→Inspect、StorageDialog→Storage、TrustMerchantDialog→Market、
/// QuestListDialog/QuestLogDialog→QuestLog、QuestDetailDialog→QuestDetail、
/// NPCAwakeDialog→NpcAwake、RefineDialog→Refine、BigMapDialog→BigMap、
/// Mail*（5 个 mail 窗）→Mail、ItemRentalDialog（浏览窗）→ItemRentalBrowse、NoticeDialog→Notice、
/// HeroInventoryDialog→HeroInventory、HeroDialog→HeroEquipment（C# `CharacterDialog` 实例，含装备/状态/状态二/技能四页）、
/// HeroManageDialog→HeroManage（状态驱动，见 `closeall`）。
/// **#2892 批C 变更**：原 `DialogKind::Hero` 自造聚合窗已删除——`HeroDialog`（C# 是
/// `CharacterDialog` 实例）**对应本端一个** `HeroEquipment` 窗（#2892 批58 合并），不再有独立 kind。
///
/// **刻意不在表内**（原版 ESC 不关这些）：`Trade`/`GuestTrade`（交易窗）、`Timer`、
/// `Buff`、`Minimap`、`DuraStatus`（`CharacterDuraPanel.Hide()` 在 `:691` 被注释掉）、
/// `Socket`、`ChatNotice`、`ItemRental`（出租方/租客窗，只有浏览窗在表内）。
pub const CLOSEALL_DIRECT: &[DialogKind] = &[
    DialogKind::Inventory,
    DialogKind::Character,
    DialogKind::Settings,
    DialogKind::Menu,
    DialogKind::Npc,
    DialogKind::Help,
    DialogKind::KeyboardLayout,
    DialogKind::Ranking,
    DialogKind::Creature,
    DialogKind::Mount,
    DialogKind::Fishing,
    DialogKind::Friend,
    DialogKind::Relationship,
    DialogKind::Mentor,
    DialogKind::GameShop,
    DialogKind::Group,
    DialogKind::Guild,
    DialogKind::Inspect,
    DialogKind::Storage,
    DialogKind::Market,
    DialogKind::QuestLog,
    DialogKind::QuestDetail,
    DialogKind::NpcAwake,
    DialogKind::Refine,
    DialogKind::BigMap,
    DialogKind::Mail,
    DialogKind::ItemRentalBrowse,
    DialogKind::Notice,
    DialogKind::HeroInventory,
    DialogKind::HeroEquipment,
];

/// #2836 单元③：`NPCDialog.Hide()` 的**级联**（`NPCDialogs.cs:1020-1040`）——仅当 NPC 对话窗
/// 当时可见时才发生（C# `if (NPCDialog.Visible) NPCDialog.Hide();`）：
/// NPCGoods/NPCSubGoods/NPCCraftGoods/NPCDrop→`NpcGoods`、NPCAwakeDialog→`NpcAwake`、
/// RefineDialog→`Refine`、StorageDialog→`Storage`、TrustMerchantDialog→`Market`、
/// QuestListDialog→`QuestLog`、RollControl→`Roll`、GuildTerritoryDialog→`GuildTerritory`、
/// BigButtonDialog（本端无独立窗）。
pub const CLOSEALL_NPC_CASCADE: &[DialogKind] = &[
    DialogKind::NpcGoods,
    DialogKind::NpcAwake,
    DialogKind::Refine,
    DialogKind::Storage,
    DialogKind::Market,
    DialogKind::QuestLog,
    DialogKind::Roll,
    DialogKind::GuildTerritory,
];

/// #2836 单元③：执行一次 C# 语义的 `Closeall`，返回是否关掉了任何窗口。
///
/// 顺序照 C#：先 `NPCDialog.Hide()`（含级联，仅当 NPC 窗当时可见），再逐条 Hide 直接表；
/// 状态驱动窗（`HeroManage`：`HeroManageDialog?.Hide()`）单独清状态。
pub fn closeall(
    mgr: &mut DialogManager,
    hero_managing: &mut bool,
    hero_confirm: &mut Option<usize>,
) -> bool {
    let mut changed = false;
    if mgr.is_open(DialogKind::Npc) {
        for kind in CLOSEALL_NPC_CASCADE {
            if mgr.is_open(*kind) {
                mgr.close(*kind);
                changed = true;
            }
        }
    }
    for kind in CLOSEALL_DIRECT {
        if mgr.is_open(*kind) {
            mgr.close(*kind);
            changed = true;
        }
    }
    // C# `HeroManageDialog?.Hide()`（`GameScene.cs:707`）：本端 `HeroManage` 是状态驱动窗
    if *hero_managing {
        *hero_managing = false;
        *hero_confirm = None;
        changed = true;
    }
    changed
}

/// 恒可见标记：挂该组件的实体不随 `DialogManager.open` 门控显隐（如 C# DuraStatusDialog
/// 切换钮——对话框关闭也恒可见）。`enforce_dialog_visibility` 会跳过它。
#[derive(Component)]
pub struct AlwaysVisible;

/// 不可拖动标记（#2797 单元②）：C# `Movable = false` 的窗口（如 `BuffDialog`，`BuffDialog.cs:32`）
/// 不参与 `dialog_drag_system` 的拖动与包围盒——否则点它会把同 `DialogKind` 的窗口一起拖走。
#[derive(Component)]
pub struct NotDraggable;

/// 「按下这里不起拖」的子区域（**相对所属 `DialogRoot` 面板左上角**的 x/y/w/h）。
///
/// 用途：**自算命中的列表/滚动区**。邮件列表行不是 `Button`（`mail_ui_system` 自己按行矩形判命中），
/// 于是「按在某一行上、手抖轻微移动」会被 `dialog_drag_system` 当成拖整窗（#3106 残留③）。
/// 给面板挂上本组件即声明「这块矩形内的按下不发起拖动」，其余区域（标题栏等）照旧可拖。
#[derive(Component, Clone, Copy, Debug)]
pub struct DragBlockArea(pub (f32, f32, f32, f32));

/// 状态驱动窗口的统一桥接：服务端/脚本状态变化时同步管理栈，
/// 让通用显隐兜底、世界输入锁与 z 序使用同一真值。
pub fn sync_dialog_state(mgr: &mut DialogManager, kind: DialogKind, visible: bool) {
    if visible {
        mgr.open(kind);
    } else {
        mgr.close(kind);
    }
}

/// 客户端逻辑分辨率（C# Settings.ScreenWidth/ScreenHeight）。
pub const UI_SCREEN_W: f32 = 1024.0;
pub const UI_SCREEN_H: f32 = 768.0;

/// C# `MirControl.Center`：逐轴整数除法，等价于 f32 结果的 floor。
pub fn center_origin(width: f32, height: f32) -> (f32, f32) {
    (
        ((UI_SCREEN_W - width) / 2.0).floor(),
        ((UI_SCREEN_H - height) / 2.0).floor(),
    )
}

impl DialogManager {
    pub fn is_open(&self, kind: DialogKind) -> bool {
        self.open.contains(&kind)
    }
    pub fn toggle(&mut self, kind: DialogKind) {
        if let Some(pos) = self.open.iter().position(|k| *k == kind) {
            self.open.remove(pos);
        } else {
            self.open.push(kind);
        }
    }
    /// 打开对话框（幂等）
    pub fn open(&mut self, kind: DialogKind) {
        if !self.open.contains(&kind) {
            self.open.push(kind);
        }
    }
    pub fn close(&mut self, kind: DialogKind) {
        self.open.retain(|k| *k != kind);
    }

    /// #1830/#2588：是否有窗口类对话框打开。
    /// Minimap 与 Timer 是 C# NotControl 覆盖层，不屏蔽世界点击。
    pub fn blocks_world_click(&self) -> bool {
        self.open
            .iter()
            .any(|k| !matches!(k, DialogKind::Minimap | DialogKind::Timer))
    }
}

/// 41 窗「点 X 关」交互门禁（headless 版实机巡回脚本，随 `cargo test --lib` 进 CI）
#[cfg(test)]
mod interact_gate;

/// #2825 单元①：测试用辅助 —— 给 world 装上「光标按下」状态后跑一次真实
/// [`dialog_drag_system`]，断言没有窗口起拖。用于「C# `Movable = false` 的窗口」的
/// 行为级验证（结构断言只保证挂了 `NotDraggable`，这里验证拖动系统真的不动它）。
#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::window::{PrimaryWindow, Window};

    /// 在 `cursor`（逻辑视口坐标）处按下左键跑一次拖动系统，断言未起拖。
    pub(crate) fn assert_no_drag_start(world: &mut World, cursor: Vec2) {
        // 对话框 spawn 时都是 Hidden（由 DialogManager 显示）——拖动系统只处理 Visible 根，
        // 故先把所有 `DialogRoot` 置为 Visible，否则断言会「假绿」
        let mut roots = world.query_filtered::<&mut Visibility, With<DialogRoot>>();
        for mut v in roots.iter_mut(world) {
            *v = Visibility::Visible;
        }
        let mut window = Window::default();
        window.set_cursor_position(Some(cursor));
        world.spawn((window, PrimaryWindow));
        let mut mouse = ButtonInput::<MouseButton>::default();
        mouse.press(MouseButton::Left);
        world.insert_resource(mouse);
        world.insert_resource(DialogDrag::default());
        world.insert_resource(crate::game::dialogs::inventory::InventoryOrigin(0.0, 0.0));
        world
            .run_system_once(dialog_drag_system)
            .expect("drag 系统应运行");
        assert_eq!(
            world.resource::<DialogDrag>().dragging,
            None,
            "Movable = false 的窗口在光标 {cursor:?} 处不应起拖"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::window::{PrimaryWindow, Window};

    #[test]
    fn state_dialog_sync_truth_table() {
        let mut m = DialogManager::default();
        for kind in [
            DialogKind::Npc,
            DialogKind::Trade,
            DialogKind::GuestTrade,
            DialogKind::NpcGoods,
            DialogKind::Buff,
            DialogKind::Roll,
        ] {
            sync_dialog_state(&mut m, kind, true);
            assert!(m.is_open(kind), "{kind:?} visible 应进入管理栈");
            sync_dialog_state(&mut m, kind, false);
            assert!(!m.is_open(kind), "{kind:?} hidden 应退出管理栈");
        }
    }

    #[test]
    fn center_origin_matches_mir_control_integer_division() {
        assert_eq!(center_origin(264.0, 272.0), (380.0, 248.0));
        assert_eq!(center_origin(200.0, 287.0), (412.0, 240.0));
        assert_eq!(center_origin(244.0, 207.0), (390.0, 280.0));
        assert_eq!(center_origin(284.0, 194.0), (370.0, 287.0));
        assert_eq!(center_origin(452.0, 376.0), (286.0, 196.0));
        assert_eq!(center_origin(590.0, 740.0), (217.0, 14.0));
    }

    #[test]
    fn test_blocks_world_click() {
        let mut m = DialogManager::default();
        assert!(!m.blocks_world_click(), "空状态不屏蔽");
        m.open.push(DialogKind::Minimap);
        assert!(!m.blocks_world_click(), "小地图不屏蔽");
        m.open.clear();
        m.open.push(DialogKind::Timer);
        assert!(!m.blocks_world_click(), "计时器 NotControl 不屏蔽");
        m.open.push(DialogKind::Inventory);
        assert!(m.blocks_world_click(), "背包打开屏蔽");
        m.open.clear();
        m.open.push(DialogKind::Npc);
        assert!(m.blocks_world_click(), "NPC 对话打开屏蔽");
        m.open.clear();
        m.open.push(DialogKind::BigMap);
        assert!(m.blocks_world_click(), "大地图打开屏蔽");
    }

    /// 回归：关闭根必须隐藏整棵 UI 子树。Bevy 的显式 Visible 子节点不会随父
    /// Visibility::Hidden 隐藏，因此根必须切到 Display::None；重开时恢复布局模式。
    #[test]
    fn hidden_dialog_root_suppresses_explicit_visible_children() {
        let mut world = World::new();
        world.insert_resource(DialogManager::default());
        let root = world
            .spawn((
                DialogRoot(DialogKind::Inventory),
                Visibility::Visible,
                Node {
                    display: Display::Grid,
                    ..default()
                },
            ))
            .id();
        let child = world.spawn((Visibility::Visible, Node::default())).id();
        world.entity_mut(root).add_child(child);

        world
            .run_system_once(enforce_dialog_visibility)
            .expect("对话框可见性系统应运行");
        world
            .run_system_once(crate::ui::theme::enforce_ui_root_display)
            .expect("UI 根显隐系统应运行");
        assert_eq!(
            world.entity(root).get::<Node>().unwrap().display,
            Display::None,
            "关闭根必须隐藏整棵子树"
        );
        assert_eq!(
            world.entity(root).get::<Visibility>(),
            Some(&Visibility::Hidden),
            "关闭根自身也应隐藏"
        );
        assert_eq!(
            world.entity(child).get::<Visibility>(),
            Some(&Visibility::Visible),
            "复现前提：子节点显式 Visible，不会被父 Hidden 级联"
        );

        world
            .resource_mut::<DialogManager>()
            .open(DialogKind::Inventory);
        world.entity_mut(root).insert(Visibility::Visible);
        world
            .run_system_once(enforce_dialog_visibility)
            .expect("对话框可见性系统应可重复运行");
        world
            .run_system_once(crate::ui::theme::enforce_ui_root_display)
            .expect("UI 根显隐系统应可重复运行");
        assert_eq!(
            world.entity(root).get::<Node>().unwrap().display,
            Display::Grid,
            "重新打开时恢复根原有 Display 模式"
        );
    }

    /// node_rect：根面板 Node Px 字段 → 屏幕矩形
    ///
    /// #2836 单元③：`closeall` 纯函数——C# 集合语义（直接表 + NPC 级联 + HeroManage 状态重置），
    /// 且**不动**交易窗等原版不关的窗口。
    #[test]
    fn closeall_matches_csharp_set() {
        let mut mgr = DialogManager::default();
        for k in [
            DialogKind::Npc,
            DialogKind::NpcGoods, // 只在 Npc 开着时才级联
            DialogKind::Inventory,
            DialogKind::Mail,
            DialogKind::Trade,      // 原版不关
            DialogKind::Timer,      // 原版不关
            DialogKind::DuraStatus, // 原版不关（`CharacterDuraPanel.Hide()` 被注释）
        ] {
            mgr.open(k);
        }
        let (mut managing, mut confirm) = (true, Some(1usize));
        assert!(
            closeall(&mut mgr, &mut managing, &mut confirm),
            "有关闭动作应返回 true"
        );
        for k in [
            DialogKind::Npc,
            DialogKind::NpcGoods,
            DialogKind::Inventory,
            DialogKind::Mail,
        ] {
            assert!(!mgr.is_open(k), "{k:?} 应被 Closeall 关闭");
        }
        for k in [DialogKind::Trade, DialogKind::Timer, DialogKind::DuraStatus] {
            assert!(mgr.is_open(k), "{k:?} 不在 Closeall 集合内，不应被关");
        }
        assert!(
            !managing && confirm.is_none(),
            "HeroManage 是状态驱动窗，需一并清状态"
        );

        // 无窗口可关时返回 false（不刷日志）
        let mut empty = DialogManager::default();
        let (mut m2, mut c2) = (false, None);
        assert!(!closeall(&mut empty, &mut m2, &mut c2));
    }

    /// node_rect：根面板 Node Px 字段 → 屏幕矩形
    #[test]
    fn node_rect_reads_px() {
        let node = Node {
            position_type: PositionType::Absolute,
            left: Val::Px(280.0),
            top: Val::Px(80.0),
            width: Val::Px(316.0),
            height: Val::Px(236.0),
            ..default()
        };
        assert_eq!(node_rect(&node), (280.0, 80.0, 316.0, 236.0));
        // 非 Px 字段防御回退 0
        let auto = Node::default();
        assert_eq!(node_rect(&auto), (0.0, 0.0, 0.0, 0.0));
    }

    /// is_descendant_of：沿 ChildOf 逐级向上找根面板
    #[test]
    fn descendant_walks_up_childof() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let mid = world.spawn_empty().id();
        let leaf = world.spawn_empty().id();
        world.entity_mut(mid).insert(ChildOf(root));
        world.entity_mut(leaf).insert(ChildOf(mid));
        let parents = world.query::<&ChildOf>();

        // 无关实体提前生成（避免闭包捕获 &world 期间可变借用冲突）
        let other = world.spawn_empty().id();
        let mut parents = world.query::<&ChildOf>();
        let mut parent_of = |x: Entity| parents.get(&world, x).ok().map(|c| c.parent());
        assert!(is_descendant_of(leaf, root, &mut parent_of));
        assert!(is_descendant_of(mid, root, &mut parent_of));
        assert!(!is_descendant_of(root, root, &mut parent_of));
        assert!(!is_descendant_of(other, root, &mut parent_of));
    }

    /// #2797 单元②：`NotDraggable` 的根（C# `Movable = false`，如 Buff 窗）不参与拖动
    #[test]
    fn not_draggable_root_is_ignored_by_drag_system() {
        // （见本测试下方的 `drag_block_area_suppresses_accidental_window_drag`：那条覆盖 #3106 残留③）
        let mut world = World::new();
        let mut window = Window::default();
        window.set_cursor_position(Some(Vec2::new(10.0, 10.0)));
        world.spawn((window, PrimaryWindow));
        let mut mouse = ButtonInput::<MouseButton>::default();
        mouse.press(MouseButton::Left);
        world.insert_resource(mouse);
        world.insert_resource(DialogDrag::default());
        world.insert_resource(crate::game::dialogs::inventory::InventoryOrigin(0.0, 0.0));

        // Buff 窗根面板 @(854,0) 44x34 + NotDraggable（C# `Movable = false`）
        let panel = world
            .spawn((
                DialogRoot(DialogKind::Buff),
                NotDraggable,
                Visibility::Visible,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Px(44.0),
                    height: Val::Px(34.0),
                    ..default()
                },
                GlobalZIndex(30),
            ))
            .id();

        world
            .run_system_once(dialog_drag_system)
            .expect("drag 系统应运行");
        assert_eq!(
            world.resource::<DialogDrag>().dragging,
            None,
            "NotDraggable 根不应起拖"
        );
        // 移动光标到别处：面板位置不变（未被拖动）
        let mut windows = world.query_filtered::<&mut Window, With<PrimaryWindow>>();
        if let Some(mut w) = windows.iter_mut(&mut world).next() {
            w.set_cursor_position(Some(Vec2::new(200.0, 200.0)));
        }
        world
            .run_system_once(dialog_drag_system)
            .expect("drag 系统应运行");
        let node = world.entity(panel).get::<Node>().unwrap().clone();
        assert_eq!(node.left, Val::Px(0.0));
        assert_eq!(node.top, Val::Px(0.0));
    }

    /// #3106 残留③：挂了 [`DragBlockArea`] 的区域按下**不得**发起拖动（自算命中列表行的误拖），
    /// 同一面板的其它区域（标题栏）仍必须能拖。
    ///
    /// 阳性对照写在同一条测试里：**去掉 `DragBlockArea` 后**，同一坐标按下会起拖（第三条断言）
    /// ——证明这道门禁真的能红。
    #[test]
    fn drag_block_area_suppresses_accidental_window_drag() {
        fn build(with_block: bool) -> World {
            let mut world = World::new();
            let mut window = Window::default();
            window.set_cursor_position(Some(Vec2::new(150.0, 200.0))); // 落在列表行区域
            world.spawn((window, PrimaryWindow));
            let mut mouse = ButtonInput::<MouseButton>::default();
            mouse.press(MouseButton::Left);
            world.insert_resource(mouse);
            world.insert_resource(DialogDrag::default());
            world.insert_resource(crate::game::dialogs::inventory::InventoryOrigin(0.0, 0.0));
            let mut panel = world.spawn((
                DialogRoot(DialogKind::Mail),
                Visibility::Visible,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(100.0),
                    top: Val::Px(100.0),
                    width: Val::Px(312.0),
                    height: Val::Px(444.0),
                    ..default()
                },
                GlobalZIndex(30),
            ));
            if with_block {
                // 邮件列表行区域：相对面板 (10,58) 290×330（与 mail.rs 的 UiScrollList 同矩形）
                panel.insert(DragBlockArea((10.0, 58.0, 290.0, 330.0)));
            }
            world
        }
        fn move_cursor(world: &mut World, x: f32, y: f32) {
            let mut windows = world.query_filtered::<&mut Window, With<PrimaryWindow>>();
            for mut w in windows.iter_mut(world) {
                w.set_cursor_position(Some(Vec2::new(x, y)));
            }
        }

        // ① 块内按下 → 不起拖
        let mut world = build(true);
        world
            .run_system_once(dialog_drag_system)
            .expect("drag 系统应运行");
        assert_eq!(
            world.resource::<DialogDrag>().dragging,
            None,
            "列表行区域按下不得起拖（选信时的按下+轻微移动不该拖整窗）"
        );

        // ② 同一面板标题栏（块外）按下 → 仍要能拖
        let mut world = build(true);
        move_cursor(&mut world, 150.0, 110.0);
        world
            .run_system_once(dialog_drag_system)
            .expect("drag 系统应运行");
        assert_eq!(
            world.resource::<DialogDrag>().dragging,
            Some(DialogKind::Mail),
            "标题栏仍必须可拖（守卫不能把整窗拖死）"
        );

        // ③ 阳性对照：没有 DragBlockArea 时，同一坐标会起拖
        let mut world = build(false);
        world
            .run_system_once(dialog_drag_system)
            .expect("drag 系统应运行");
        assert_eq!(
            world.resource::<DialogDrag>().dragging,
            Some(DialogKind::Mail),
            "阳性对照：无守卫时列表区按下会误拖（说明这条门禁能红）"
        );
    }

    /// bevy_ui 拖拽：点中根面板（非按钮）→ 拖动 Node.left/top；第二帧移动鼠标 →
    /// 面板平移 + InventoryOrigin 同步；子输入框命中区（绝对坐标）同步
    #[test]
    fn drag_moves_panel_node_and_syncs_abs_rects() {
        let mut world = World::new();
        let mut window = Window::default();
        window.set_cursor_position(Some(Vec2::new(50.0, 50.0)));
        world.spawn((window, PrimaryWindow));
        let mut mouse = ButtonInput::<MouseButton>::default();
        mouse.press(MouseButton::Left); // just_pressed + pressed
        world.insert_resource(mouse);
        world.insert_resource(DialogDrag::default());
        world.insert_resource(crate::game::dialogs::inventory::InventoryOrigin(0.0, 0.0));

        // 背包根面板 @ (0,0) 316x236（inventory 拖拽同步 InventoryOrigin）
        let panel = world
            .spawn((
                DialogRoot(DialogKind::Inventory),
                Visibility::Visible,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Px(316.0),
                    height: Val::Px(236.0),
                    ..default()
                },
                GlobalZIndex(30),
            ))
            .id();
        // 输入框（面板子节点，TextInputRect 绝对屏幕坐标）
        world.entity_mut(panel).with_children(|p| {
            p.spawn((
                TextInputRect(50.0, 60.0, 130.0, 10.0),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(50.0),
                    top: Val::Px(60.0),
                    ..default()
                },
            ));
        });
        // 无关对话框（不应被移动）
        world.spawn((
            DialogRoot(DialogKind::Trade),
            Visibility::Visible,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(298.0),
                top: Val::Px(418.0),
                width: Val::Px(300.0),
                height: Val::Px(200.0),
                ..default()
            },
            GlobalZIndex(30),
        ));

        // 第一帧：点中背包 → 开始拖动
        world
            .run_system_once(dialog_drag_system)
            .expect("drag 系统应运行");
        {
            let drag = world.resource::<DialogDrag>();
            assert_eq!(drag.dragging, Some(DialogKind::Inventory), "应开始拖动背包");
        }

        // 第二帧：鼠标移到 (100,80)，保持按住 → 面板平移 (50,30)，TextInputRect 与 Origin 同步
        let mut win_q = world.query::<&mut Window>();
        for mut win in win_q.iter_mut(&mut world) {
            win.set_cursor_position(Some(Vec2::new(100.0, 80.0)));
        }
        drop(win_q);
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear_just_pressed(MouseButton::Left); // 保持 pressed，消费 just_pressed
        world
            .run_system_once(dialog_drag_system)
            .expect("drag 系统应运行");

        let mut q = world.query::<(&DialogRoot, &Node)>();
        let inv_node = q
            .iter(&world)
            .find(|(r, _)| r.0 == DialogKind::Inventory)
            .map(|(_, n)| {
                (
                    match n.left {
                        Val::Px(v) => v,
                        _ => -1.0,
                    },
                    match n.top {
                        Val::Px(v) => v,
                        _ => -1.0,
                    },
                )
            })
            .expect("背包面板存在");
        assert_eq!(inv_node, (50.0, 30.0), "背包根面板应平移 delta=(50,30)");
        let trade_node = q
            .iter(&world)
            .find(|(r, _)| r.0 == DialogKind::Trade)
            .map(|(_, n)| match n.left {
                Val::Px(v) => v,
                _ => -1.0,
            })
            .expect("交易面板存在");
        assert_eq!(trade_node, 298.0, "无关对话框不应移动");
        // 输入框命中矩形（绝对坐标）跟随平移
        let tr = world
            .query_filtered::<&TextInputRect, ()>()
            .iter(&world)
            .next()
            .cloned()
            .expect("输入框存在");
        assert_eq!((tr.0, tr.1), (100.0, 90.0), "TextInputRect 应同步平移");
        // InventoryOrigin 同步
        let origin = world.resource::<crate::game::dialogs::inventory::InventoryOrigin>();
        assert_eq!((origin.0, origin.1), (50.0, 30.0));

        // 松开鼠标 → 结束拖动
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        world
            .run_system_once(dialog_drag_system)
            .expect("drag 系统应运行");
        assert_eq!(world.resource::<DialogDrag>().dragging, None);
    }

    /// bevy_ui 置顶：新打开对话框 → GlobalZIndex 抬到现有最大值之上（保留内部层级）
    #[test]
    fn front_bumps_new_open_to_top() {
        let mut world = World::new();
        world.insert_resource(DialogZ::default());
        world.insert_resource(ButtonInput::<MouseButton>::default());
        let mut mgr = DialogManager::default();
        mgr.open.push(DialogKind::Character);
        mgr.open.push(DialogKind::Mail);
        world.insert_resource(mgr);
        // 角色面板 gz=30；邮件主面板 gz=30、写邮件覆盖层 gz=40（内部层级）
        world.spawn((
            DialogRoot(DialogKind::Character),
            Visibility::Visible,
            Node::default(),
            GlobalZIndex(30),
        ));
        world.spawn((
            DialogRoot(DialogKind::Mail),
            Visibility::Visible,
            Node::default(),
            GlobalZIndex(30),
        ));
        world.spawn((
            DialogRoot(DialogKind::Mail),
            Visibility::Visible,
            Node::default(),
            GlobalZIndex(40),
        ));

        world
            .run_system_once(dialog_front_system)
            .expect("front 系统应运行");

        let mut q = world.query::<(&DialogRoot, &GlobalZIndex)>();
        let mail_gz: Vec<i32> = q
            .iter(&world)
            .filter(|(r, _)| r.0 == DialogKind::Mail)
            .map(|(_, g)| g.0)
            .collect();
        let char_gz = q
            .iter(&world)
            .find(|(r, _)| r.0 == DialogKind::Character)
            .map(|(_, g)| g.0)
            .expect("角色面板存在");
        // Mail 新打开 → 整体抬到 50（最高 40 + 10），内部层级差 10 保留
        let mut sorted = mail_gz.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, vec![40, 50], "Mail 两面板抬到 40/50，内部层级保留");
        assert_eq!(char_gz, 30, "Character 未置顶保持原值");
    }

    /// S1 回归：孤儿弹窗（邀请/确认框）挂 `DialogRoot` + `AlwaysVisible` 后——
    /// 1) `enforce_dialog_visibility` 不得因 kind 未 open 而误隐藏它们（显隐由各自
    ///    ui_system 驱动）；
    /// 2) 同 kind 的普通根仍被兜底隐藏。
    #[test]
    fn enforce_visibility_skips_orphan_popup_but_hides_unopened_root() {
        let mut world = World::new();
        world.insert_resource(DialogManager::default());
        let orphan = world
            .spawn((
                DialogRoot(DialogKind::Mentor),
                AlwaysVisible,
                Visibility::Visible,
            ))
            .id();
        let plain = world
            .spawn((DialogRoot(DialogKind::Trade), Visibility::Visible))
            .id();

        world
            .run_system_once(enforce_dialog_visibility)
            .expect("enforce 系统应运行");

        assert_eq!(
            world.entity(orphan).get::<Visibility>(),
            Some(&Visibility::Visible),
            "孤儿弹窗（AlwaysVisible）显隐由自身系统驱动，不被兜底误伤"
        );
        assert_eq!(
            world.entity(plain).get::<Visibility>(),
            Some(&Visibility::Hidden),
            "未 open 的普通 DialogRoot 仍被兜底隐藏"
        );
    }

    /// S1 回归：孤儿弹窗挂 `DialogRoot` 后，OnExit(Game) 的清理系统（各插件同款
    /// 「despawn 全部 DialogRoot」）能把它们一并清掉，登出重进不再堆积。
    #[test]
    fn exit_cleanup_despawns_orphan_popups() {
        fn cleanup_all(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
            for e in roots.iter() {
                commands.entity(e).despawn();
            }
        }
        let mut world = World::new();
        // 主面板 + 三个孤儿弹窗（邀请/确认框均带 GlobalZIndex + AlwaysVisible）
        world.spawn((DialogRoot(DialogKind::Mentor), Visibility::Hidden));
        world.spawn((
            DialogRoot(DialogKind::Mentor),
            AlwaysVisible,
            mentor::MentorInviteWidget,
            Visibility::Hidden,
        ));
        world.spawn((
            DialogRoot(DialogKind::Inventory),
            AlwaysVisible,
            inventory::InvConfirmWidget,
            Visibility::Hidden,
        ));
        world.spawn((
            DialogRoot(DialogKind::Market),
            AlwaysVisible,
            market::MarketConfirmWidget,
            Visibility::Hidden,
        ));

        world
            .run_system_once(cleanup_all)
            .expect("cleanup 系统应运行");

        let mut q = world.query::<&DialogRoot>();
        assert_eq!(
            q.iter(&world).count(),
            0,
            "OnExit 清理后不应残留任何 DialogRoot（含孤儿弹窗）"
        );
        let mut q = world.query::<&mentor::MentorInviteWidget>();
        assert_eq!(q.iter(&world).count(), 0);
    }

    /// 离开 Game 清理测试脚手架：真实状态机 + OnExit 挂法与生产一致
    /// （StatesPlugin + init_state + OnExit(AppState::Game)，与插件注册同配置）
    fn logout_test_app() -> App {
        let mut app = App::new();
        // App::new 只有 MainSchedulePlugin，状态迁移需要显式 StatesPlugin
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<AppState>();
        app.init_resource::<DialogManager>();
        app.init_resource::<crate::network::SessionState>();
        app.init_resource::<mentor::MentorState>();
        app.init_resource::<relationship::RelationshipState>();
        app.init_resource::<trade::TradeState>();
        app.init_resource::<roll::RollState>();
        app.init_resource::<fishing::FishingState>();
        app.init_resource::<memo::MemoState>();
        app.init_resource::<npc::NpcDialogState>();
        app.init_resource::<npc_goods::NpcGoodsState>();
        app.init_resource::<inventory::InvDropConfirm>();
        app.init_resource::<market::MarketConfirm>();
        app.init_resource::<npc::NpcInputState>();
        app.init_resource::<input_box::InputBoxState>();
        app.add_systems(OnExit(AppState::Game), clear_dialog_session_on_exit);
        app
    }

    /// 进入 Game 场景（Intro → Game）
    fn enter_game(app: &mut App) {
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Game);
        app.update();
        assert_eq!(
            *app.world().resource::<State<AppState>>().get(),
            AppState::Game
        );
    }

    /// 把世界弄脏成「离开 Game 前」：对话框栈 + 会话状态 + 全部孤儿弹窗/状态驱动资源
    fn dirty_logout_state(app: &mut App) {
        app.world_mut()
            .resource_mut::<DialogManager>()
            .open(DialogKind::Trade);
        {
            let mut session = app
                .world_mut()
                .resource_mut::<crate::network::SessionState>();
            session.self_position = Some((10, 20, 0));
            session.local_player_id = Some(42);
            session.selected_index = Some(1);
        }
        app.world_mut().resource_mut::<mentor::MentorState>().invite =
            Some(("师父".to_string(), 40));
        app.world_mut()
            .resource_mut::<relationship::RelationshipState>()
            .invite = Some("恋人".to_string());
        {
            // TradeState 全字段弄脏：visible/物品/发起者任一残留都会让交易窗
            // 带旧物品幽灵重开（对抗复核 severe-3）
            let mut trade = app.world_mut().resource_mut::<trade::TradeState>();
            trade.visible = true;
            trade.invite = Some("商人".to_string());
            trade.is_initiator = true;
            trade.my_gold = 100;
            trade.my_locked = true;
            trade.my_items[0] = Some(trade::TradeItem {
                uid: 1,
                item_index: 2,
                name: "木剑".to_string(),
                image: 3,
                count: 1,
            });
            trade.their_items[0] = Some(trade::TradeItem {
                uid: 4,
                item_index: 5,
                name: "布衣".to_string(),
                image: 6,
                count: 1,
            });
        }
        {
            let mut roll = app.world_mut().resource_mut::<roll::RollState>();
            roll.visible = true;
            roll.result = 5;
        }
        app.world_mut()
            .resource_mut::<fishing::FishingState>()
            .fishing = true;
        app.world_mut().resource_mut::<memo::MemoState>().open = true;
        app.world_mut()
            .resource_mut::<npc::NpcDialogState>()
            .visible = true;
        app.world_mut()
            .resource_mut::<npc_goods::NpcGoodsState>()
            .visible = true;
        app.world_mut()
            .resource_mut::<inventory::InvDropConfirm>()
            .visible = true;
        app.world_mut()
            .resource_mut::<market::MarketConfirm>()
            .visible = true;
        app.world_mut().resource_mut::<npc::NpcInputState>().active = true;
        app.world_mut()
            .resource_mut::<input_box::InputBoxState>()
            .open = true;
    }

    fn assert_dialog_state_clean(app: &App, ctx: &str) {
        let w = app.world();
        assert!(
            w.resource::<DialogManager>().open.is_empty(),
            "{ctx}: 对话框栈应清空"
        );
        {
            let session = w.resource::<crate::network::SessionState>();
            assert!(
                session.self_position.is_none()
                    && session.local_player_id.is_none()
                    && session.selected_index.is_none(),
                "{ctx}: SessionState 应复位"
            );
        }
        assert!(
            w.resource::<mentor::MentorState>().invite.is_none(),
            "{ctx}: MentorState.invite 应复位"
        );
        assert!(
            w.resource::<relationship::RelationshipState>()
                .invite
                .is_none(),
            "{ctx}: RelationshipState.invite 应复位"
        );
        {
            let trade = w.resource::<trade::TradeState>();
            assert!(
                !trade.visible
                    && trade.invite.is_none()
                    && !trade.is_initiator
                    && trade.my_gold == 0
                    && !trade.my_locked
                    && trade.my_items.iter().all(|i| i.is_none())
                    && trade.their_items.iter().all(|i| i.is_none()),
                "{ctx}: TradeState 应整体复位（防交易窗带旧物品幽灵重开）"
            );
        }
        {
            let roll = w.resource::<roll::RollState>();
            assert!(
                !roll.visible && roll.result == 0,
                "{ctx}: RollState 应整体复位"
            );
        }
        assert!(
            !w.resource::<fishing::FishingState>().fishing,
            "{ctx}: FishingState 应复位"
        );
        assert!(
            !w.resource::<memo::MemoState>().open,
            "{ctx}: MemoState 应复位"
        );
        assert!(
            !w.resource::<npc::NpcDialogState>().visible,
            "{ctx}: NpcDialogState 应复位"
        );
        assert!(
            !w.resource::<npc_goods::NpcGoodsState>().visible,
            "{ctx}: NpcGoodsState 应复位"
        );
        assert!(
            !w.resource::<inventory::InvDropConfirm>().visible,
            "{ctx}: InvDropConfirm 应复位"
        );
        assert!(
            !w.resource::<market::MarketConfirm>().visible,
            "{ctx}: MarketConfirm 应复位"
        );
        assert!(
            !w.resource::<npc::NpcInputState>().active,
            "{ctx}: NpcInputState 应复位"
        );
        assert!(
            !w.resource::<input_box::InputBoxState>().open,
            "{ctx}: InputBoxState 应复位"
        );
    }

    /// #182 回归：登出离开 Game（Game → Select）→ 对话框栈清空 + 会话/孤儿弹窗/
    /// 状态驱动资源全部复位。走真实状态迁移，不再靠轮询 ServerEvent。
    #[test]
    fn exit_game_to_select_clears_dialogs_and_orphan_popup_states() {
        let mut app = logout_test_app();
        enter_game(&mut app);
        dirty_logout_state(&mut app);
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Select);
        app.update();
        assert_dialog_state_clean(&app, "Game→Select 登出");
    }

    /// M6 回归：断线回登录（Game → Login）同样全清——OnExit 钩子在状态迁移帧
    /// 执行，无旧实现「logout_server_events 与 network_system 无排序、run_if
    /// 挡住重连窗口」的时序孔。且再次进出 Game 能再次触发（非一次性）。
    #[test]
    fn exit_game_to_login_disconnect_clears_dialogs() {
        let mut app = logout_test_app();
        enter_game(&mut app);
        dirty_logout_state(&mut app);
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Login);
        app.update();
        assert_dialog_state_clean(&app, "Game→Login 断线");

        // 重进 Game 对话框可再开（不被误清），再次离开再次全清
        enter_game(&mut app);
        app.world_mut()
            .resource_mut::<DialogManager>()
            .open(DialogKind::Trade);
        app.update();
        assert!(
            app.world()
                .resource::<DialogManager>()
                .is_open(DialogKind::Trade),
            "重进 Game 后对话框可正常打开"
        );
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Login);
        app.update();
        assert!(
            app.world().resource::<DialogManager>().open.is_empty(),
            "再次断线应再次清空"
        );
    }
}

/// 对话框根标记（OnExit(Game) 统一清理）
#[derive(Component)]
#[require(crate::ui::theme::UiRootDisplay)]
pub struct DialogRoot(pub DialogKind);

/// 弹窗拖动状态（#34：原版弹窗可拖动；bevy_ui 版按根面板 Node 增量位移）
#[derive(Resource, Default)]
pub struct DialogDrag {
    /// 正在拖动的对话框类型
    pub dragging: Option<DialogKind>,
    /// 拖动开始时的鼠标位置（逻辑坐标）
    pub start_cursor: Vec2,
    /// 拖动开始时各根面板 Node.left/top 的原始位置（bevy_ui 版）
    pub panel_origins: std::collections::HashMap<Entity, (f32, f32)>,
    /// 拖动开始时文本输入框命中矩形的原始位置（绝对屏幕坐标，跟随平移）
    pub text_origins: std::collections::HashMap<Entity, (f32, f32)>,
    /// 拖动开始时下拉框命中矩形的原始位置（box_rect + popup_pos，绝对坐标）
    pub dd_origins: std::collections::HashMap<Entity, ((f32, f32), (f32, f32))>,
    /// 拖动开始时的背包命中原点（仅拖 Inventory 时 Some）
    pub inv_origin_start: Option<(f32, f32)>,
}

/// 对话框置顶层级（front 系统维护单调递增的 GlobalZIndex 顶值）
#[derive(Resource, Default)]
pub struct DialogZ {
    pub top: i32,
}

/// 根面板 Node 矩形（屏幕坐标：根面板是 UI 根的子节点，left/top 即绝对坐标）。
/// bevy_ui 对话框根面板均显式设置 Px 尺寸；非 Px 回退 0（防御）。
pub(crate) fn node_rect(node: &Node) -> (f32, f32, f32, f32) {
    let x = match node.left {
        Val::Px(v) => v,
        _ => 0.0,
    };
    let y = match node.top {
        Val::Px(v) => v,
        _ => 0.0,
    };
    let w = match node.width {
        Val::Px(v) => v,
        _ => 0.0,
    };
    let h = match node.height {
        Val::Px(v) => v,
        _ => 0.0,
    };
    (x, y, w, h)
}

/// 判断实体是否挂在给定根面板之下（沿 ChildOf 逐级向上；用于拖动时同步
/// 子节点上携带的绝对坐标组件：TextInputRect / UiDropDown）。
/// `parent_of` 返回实体的父实体（无父返回 None）。
fn is_descendant_of(
    e: Entity,
    root: Entity,
    parent_of: &mut impl FnMut(Entity) -> Option<Entity>,
) -> bool {
    let mut cur = e;
    while let Some(p) = parent_of(cur) {
        if p == root {
            return true;
        }
        cur = p;
    }
    false
}

/// 通用弹窗拖动系统（bevy_ui 版）：
/// - 按 DialogKind 聚合**根面板**（有 GlobalZIndex 的 DialogRoot 节点 = 根；子格等无
///   GlobalZIndex 不参与），用根面板矩形估算窗口包围盒，按住任意位置（非按钮）可拖
/// - 拖动时对根面板 Node.left/top 整体平移（子节点随根），并同步子节点上携带的
///   绝对坐标组件：TextInputRect / UiDropDown / InventoryOrigin
pub fn dialog_drag_system(
    mut drag: ResMut<DialogDrag>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    buttons: Query<&Interaction, With<Button>>,
    // 「按下不起拖」区域（只读 DialogRoot/可见性/矩形；**不碰 Node**，
    // 否则与上面 `&mut Node` 的 dialogs 查询构成 B0001 冲突）
    drag_blocks: Query<(&DialogRoot, &DragBlockArea, &Visibility)>,
    // #2797 单元②：C# `Movable = false` 的窗口（`NotDraggable`）不参与拖动/包围盒
    mut dialogs: Query<
        (Entity, &DialogRoot, &Visibility, &mut Node, &GlobalZIndex),
        Without<NotDraggable>,
    >,
    mut text_rects: Query<(Entity, &mut TextInputRect)>,
    mut drop_downs: Query<(Entity, &mut UiDropDown)>,
    parents: Query<&ChildOf>,
    // 背包拖动时同步 InventoryOrigin（inv_slot_at/仓库/交易命中依赖它）
    mut inv_origin: ResMut<crate::game::dialogs::inventory::InventoryOrigin>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    // 聚合每个 kind 的根面板包围盒（bevy_ui 面板根 = 有 GlobalZIndex 的 DialogRoot）
    let mut boxes: std::collections::HashMap<DialogKind, (f32, f32, f32, f32, i32)> =
        std::collections::HashMap::new();
    for (_, root, vis, node, gz) in dialogs.iter() {
        if *vis != Visibility::Visible {
            continue;
        }
        let (x, y, w, h) = node_rect(node);
        let b = boxes.entry(root.0).or_insert((x, y, x + w, y + h, gz.0));
        b.0 = b.0.min(x);
        b.1 = b.1.min(y);
        b.2 = b.2.max(x + w);
        b.3 = b.3.max(y + h);
        b.4 = b.4.max(gz.0);
    }

    if mouse.just_pressed(MouseButton::Left) && drag.dragging.is_none() {
        // 按钮上不触发（bevy_ui Interaction 由 ui_focus_system 按命中计算）
        let on_button = buttons.iter().any(|i| *i != Interaction::None);
        if !on_button {
            let mut top: Option<(DialogKind, i32)> = None;
            for (kind, (minx, miny, maxx, maxy, maxz)) in &boxes {
                if cursor.x >= *minx && cursor.x <= *maxx && cursor.y >= *miny && cursor.y <= *maxy
                {
                    if top.map(|(_, z)| *maxz > z).unwrap_or(true) {
                        top = Some((*kind, *maxz));
                    }
                }
            }
            if let Some((kind, _)) = top {
                // #3106 残留③：自算命中的列表区（如邮件列表行）按下时不起拖——
                // 否则「选一封邮件」的按下+轻微移动会把整窗拖走。
                let on_blocked = boxes.get(&kind).is_some_and(|(minx, miny, _, _, _)| {
                    drag_blocks.iter().any(|(root, area, vis)| {
                        if root.0 != kind || *vis != Visibility::Visible {
                            return false;
                        }
                        let (bx, by, bw, bh) = area.0;
                        let (x0, y0) = (minx + bx, miny + by);
                        cursor.x >= x0
                            && cursor.x <= x0 + bw
                            && cursor.y >= y0
                            && cursor.y <= y0 + bh
                    })
                });
                if on_blocked {
                    return;
                }
                let roots: Vec<Entity> = dialogs
                    .iter()
                    .filter(|(_, r, v, _, _)| *v == Visibility::Visible && r.0 == kind)
                    .map(|(e, _, _, _, _)| e)
                    .collect();
                drag.dragging = Some(kind);
                drag.start_cursor = cursor;
                drag.panel_origins = dialogs
                    .iter()
                    .filter(|(_, r, v, _, _)| *v == Visibility::Visible && r.0 == kind)
                    .map(|(e, _, _, node, _)| {
                        let (x, y, _, _) = node_rect(node);
                        (e, (x, y))
                    })
                    .collect();
                drag.inv_origin_start =
                    (kind == DialogKind::Inventory).then(|| (inv_origin.0, inv_origin.1));
                // 输入框/下拉框命中矩形（绝对屏幕坐标）跟随：只收集挂在被拖 kind 根面板下的
                drag.text_origins = text_rects
                    .iter()
                    .filter(|(e, _)| {
                        roots.iter().any(|r| {
                            is_descendant_of(*e, *r, &mut |x| {
                                parents.get(x).ok().map(|c| c.parent())
                            })
                        })
                    })
                    .map(|(e, tr)| (e, (tr.0, tr.1)))
                    .collect();
                drag.dd_origins = drop_downs
                    .iter()
                    .filter(|(e, _)| {
                        roots.iter().any(|r| {
                            is_descendant_of(*e, *r, &mut |x| {
                                parents.get(x).ok().map(|c| c.parent())
                            })
                        })
                    })
                    .map(|(e, dd)| {
                        (
                            e,
                            (
                                (dd.box_rect.0, dd.box_rect.1),
                                (dd.popup_pos.0, dd.popup_pos.1),
                            ),
                        )
                    })
                    .collect();
                tracing::info!("🖱️ 拖动对话框 {:?}", kind);
            }
        }
    }

    if let Some(kind) = drag.dragging {
        if mouse.pressed(MouseButton::Left) {
            let delta = cursor - drag.start_cursor;
            for (e, root, vis, mut node, _) in dialogs.iter_mut() {
                if *vis == Visibility::Visible && root.0 == kind {
                    if let Some((ox, oy)) = drag.panel_origins.get(&e) {
                        node.left = Val::Px(ox + delta.x);
                        node.top = Val::Px(oy + delta.y);
                    }
                }
            }
            // 输入框命中区 / 下拉框命中区（屏幕坐标）随面板平移
            for (e, mut tr) in text_rects.iter_mut() {
                if let Some(o) = drag.text_origins.get(&e) {
                    tr.0 = o.0 + delta.x;
                    tr.1 = o.1 + delta.y;
                }
            }
            for (e, mut dd) in drop_downs.iter_mut() {
                if let Some(o) = drag.dd_origins.get(&e) {
                    dd.box_rect.0 = o.0 .0 + delta.x;
                    dd.box_rect.1 = o.0 .1 + delta.y;
                    dd.popup_pos.0 = o.1 .0 + delta.x;
                    dd.popup_pos.1 = o.1 .1 + delta.y;
                }
            }
            // 背包拖动 → 命中用原点同步平移（与实体/rect 同一 delta）
            if let Some(o) = drag.inv_origin_start {
                *inv_origin =
                    crate::game::dialogs::inventory::InventoryOrigin(o.0 + delta.x, o.1 + delta.y);
            }
        } else {
            drag.dragging = None;
            drag.panel_origins.clear();
            drag.text_origins.clear();
            drag.dd_origins.clear();
            drag.inv_origin_start = None;
        }
    }
}

/// 置顶系统（bevy_ui 版）：
/// - 新打开的对话框 → 置顶（对齐 C# Show → BringToFront）
/// - 点击可见对话框根面板 → 置顶（对齐 C# 点击窗口置前）
pub fn dialog_front_system(
    mut z: ResMut<DialogZ>,
    mgr: Res<DialogManager>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut dialogs: Query<(Entity, &DialogRoot, &Visibility, &Node, &mut GlobalZIndex)>,
    mut prev_open: Local<Vec<DialogKind>>,
) {
    if z.top < 30 {
        z.top = 30;
    }

    if let Some(kind) = mgr.open.last().copied().filter(|k| !prev_open.contains(k)) {
        bump_dialog_z(kind, &mut z, &mut dialogs);
    }
    *prev_open = mgr.open.clone();

    if mouse.just_pressed(MouseButton::Left) {
        let Ok(window) = windows.single() else {
            return;
        };
        let Some(cursor) = window.cursor_position() else {
            return;
        };
        let mut boxes: std::collections::HashMap<DialogKind, (f32, f32, f32, f32, i32)> =
            std::collections::HashMap::new();
        for (_, root, vis, node, gz) in dialogs.iter() {
            if *vis != Visibility::Visible {
                continue;
            }
            let (x, y, w, h) = node_rect(node);
            let b = boxes.entry(root.0).or_insert((x, y, x + w, y + h, gz.0));
            b.0 = b.0.min(x);
            b.1 = b.1.min(y);
            b.2 = b.2.max(x + w);
            b.3 = b.3.max(y + h);
            b.4 = b.4.max(gz.0);
        }
        let mut best: Option<(DialogKind, i32)> = None;
        for (kind, (minx, miny, maxx, maxy, maxz)) in &boxes {
            if cursor.x >= *minx && cursor.x <= *maxx && cursor.y >= *miny && cursor.y <= *maxy {
                if best.map(|(_, bz)| *maxz > bz).unwrap_or(true) {
                    best = Some((*kind, *maxz));
                }
            }
        }
        if let Some((kind, _)) = best {
            bump_dialog_z(kind, &mut z, &mut dialogs);
        }
    }
}

/// 把指定对话框整体平移到置顶 z（保留内部相对层级：整体平移使最高者 = z.top，
/// 覆盖层如 MailCompose/StorageUnlock 保持高于其父面板）
fn bump_dialog_z(
    kind: DialogKind,
    z: &mut DialogZ,
    dialogs: &mut Query<(Entity, &DialogRoot, &Visibility, &Node, &mut GlobalZIndex)>,
) {
    let mut max_gz = i32::MIN;
    let mut any = false;
    for (_, r, _, _, gz) in dialogs.iter() {
        if r.0 == kind {
            max_gz = max_gz.max(gz.0);
            any = true;
        }
    }
    if !any {
        return;
    }
    let top = z.top.max(max_gz + 10);
    z.top = top + 10;
    let delta = top - max_gz;
    for (_, r, _, _, mut gz) in dialogs.iter_mut() {
        if r.0 == kind {
            gz.0 += delta;
        }
    }
    tracing::info!("📌 置顶对话框 {:?}（z={}）", kind, top);
}

pub struct DialogsPlugin;

/// 通用对话框可见性兜底（#幽灵/泄漏）：PostUpdate 把不在 `DialogManager.open`
/// 的 `DialogRoot(kind)` 根设为 `Visibility::Hidden`。跳过 `AlwaysVisible`。
/// 紧随其后的 `enforce_ui_root_display` 会把隐藏根切到 `Display::None`，确保显式
/// `Visibility::Visible` 的子控件也不会继续渲染。
fn enforce_dialog_visibility(
    mgr: Res<DialogManager>,
    mut q: Query<(&DialogRoot, &mut Visibility), Without<AlwaysVisible>>,
) {
    for (root, mut vis) in &mut q {
        if !mgr.is_open(root.0) {
            *vis = Visibility::Hidden;
        }
    }
}

impl Plugin for DialogsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DialogManager>();
        app.init_resource::<DialogDrag>();
        app.init_resource::<DialogZ>();
        // storage/inventory cells 循环的图像缓存（评审 P1；login/select 已有，幂等兜底）
        app.init_resource::<crate::ui::sprite_ui::UiImageCache>();
        app.add_plugins(hero_equipment::HeroEquipmentPlugin);
        app.add_plugins(hero_skills::HeroSkillPlugin);
        // 先置顶再开始拖动：点击重叠窗口时，先让被点窗口到最前，再由 drag 选中它。
        // 通用对话框可见性兜底：PostUpdate（所有 Update 对话框 ui_system 之后）强制隐藏
        // 未 open 的挂 DialogRoot 实体，消除控件泄漏叠加（清理"一堆 UI 堆屏幕"）。
        app.add_systems(
            PostUpdate,
            (
                enforce_dialog_visibility,
                crate::ui::theme::enforce_ui_root_display,
            )
                .chain()
                .run_if(in_state(AppState::Game)),
        );
        app.add_systems(
            Update,
            (dialog_front_system, dialog_drag_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
        // #182 登出 / M6 断线 / #289 ReturnToLogin：离开 Game 时统一清理对话框与会话
        // 状态。挂 OnExit 与实体清理同帧同语义——三条离开路径（登出回 Select、
        // ReturnToLogin/断线回 Login）都经过状态迁移，无轮询上升沿的排序时序孔
        app.add_systems(OnExit(AppState::Game), clear_dialog_session_on_exit);
        app.add_systems(
            Update,
            crate::ui::scroll_list::scroll_list_system.run_if(in_state(AppState::Game)),
        );
        app.add_systems(
            Update,
            (
                crate::ui::controls::checkbox_system,
                crate::ui::controls::dropdown_system,
                crate::ui::theme::dropdown_ui_system,
                crate::ui::theme::animated_button_ui_system,
                crate::ui::theme::item_cell_ui_system,
                crate::ui::theme::scroll_list_ui_system,
                crate::ui::controls::scrolling_label_system,
                crate::ui::controls::item_cell_system,
                crate::ui::controls::animated_button_system,
            )
                .run_if(in_state(AppState::Game)),
        );
        app.init_resource::<crate::ui::keyboard_nav::KeyboardNav>();
        app.init_resource::<crate::ui::scroll_list::ScrollDrag>();
        app.add_systems(
            Update,
            (
                // #2595：Esc 优先级要求 esc_close 先于 chat_input 跑——
                // 聊天输入开时 esc_close 让路，chat_input_system 同帧关闭输入行
                // #2604：amount_box/player_menu 同理（审查 MAJOR——无排序边时
                // 它们可能先跑、同帧置 visible=false，esc_close 随后读到 false
                // 误入 Closeall，一次 Esc 连坐关掉模态 + 全部对话框）
                crate::ui::keyboard_nav::esc_close_dialogs_system
                    .before(crate::game::chat::chat_input_system)
                    .before(crate::game::dialogs::amount_box::amount_box_system)
                    .before(crate::game::player_menu::player_menu_ui_system),
                crate::ui::keyboard_nav::keyboard_scroll_lists_system,
                crate::ui::keyboard_nav::tab_focus_system,
            )
                .run_if(in_state(AppState::Game)),
        );
        // #93 通用 Tooltip
        app.init_resource::<crate::ui::tooltip::TooltipState>();
        app.add_systems(
            OnEnter(AppState::Game),
            crate::ui::tooltip::spawn_tooltip_panel_system,
        );
        app.add_systems(
            OnExit(AppState::Game),
            crate::ui::tooltip::despawn_tooltip_panel,
        );
        app.add_systems(
            Update,
            (
                crate::ui::tooltip::tooltip_hint_system,
                // #2771：bevy UI 按钮（`spawn_icon_button`）的通用 Hint 通道
                crate::ui::tooltip::ui_hint_system,
                crate::ui::tooltip::tooltip_panel_system,
                // #2775：面板已改 bevy_ui 节点，描边副本内容需在其写入之后同步
                crate::ui::outlined_text::sync_outline_ui_system
                    .after(crate::ui::tooltip::tooltip_panel_system),
            )
                .run_if(in_state(AppState::Game)),
        );
        app.add_plugins(text_input::TextInputPlugin);
        // #2892 批D 单元①：好友备注窗（C# `MemoDialog`）
        app.add_plugins(memo::MemoPlugin);
        // #2892 批D 单元①：可拖窗口偏移状态 + 拖动系统（C# `MirControl.Movable`）
        app.init_resource::<window_drag::WindowDragState>();
        app.add_systems(
            Update,
            window_drag::window_drag_system.run_if(in_state(crate::scenes::AppState::Game)),
        );
        // #2892 批C：游戏内 `MirInputBox`（服务端发起式取名）
        app.add_plugins(input_box::InputBoxPlugin);
        app.add_plugins((
            (
                inventory::InventoryDialogPlugin,
                assign_key::AssignKeyPlugin,
                character::CharacterDialogPlugin,
                menu::MenuDialogPlugin,
                minimap::MiniMapPlugin,
                potion_belt::PotionBeltPlugin,
                compass::CompassPlugin,
                npc::NpcDialogPlugin,
                quest_log::QuestLogPlugin,
            ),
            (
                group::GroupPlugin,
                friend::FriendPlugin,
                amount_box::AmountBoxPlugin,
                trade::TradePlugin,
                inspect::InspectPlugin,
                npc_goods::NpcGoodsPlugin,
                guild::GuildPlugin,
                mail::MailPlugin,
            ),
            (
                ranking::RankingPlugin,
                mentor::MentorPlugin,
                relationship::RelationshipPlugin,
                mount::MountPlugin,
                report::ReportPlugin,
                hero::HeroPlugin,
                hero_belt::HeroBeltPlugin,
                hero_inventory::HeroInventoryPlugin,
                creature::CreaturePlugin,
                item_rental::ItemRentalPlugin,
                guild_territory::GuildTerritoryPlugin,
                option::OptionPlugin,
                help::HelpPlugin,
                notice::NoticePlugin,
                buff::BuffPlugin,
            ),
            (
                fishing::FishingPlugin,
                socket::SocketPlugin,
                refine::RefinePlugin,
                craft::CraftPlugin,
                dura_status::DuraPlugin,
                roll::RollPlugin,
                npc_awake::NpcAwakePlugin,
                timer::TimerPlugin,
                keyboard_layout::KeyboardPlugin,
                big_map::BigMapPlugin,
                chat_notice::ChatNoticePlugin,
            ),
            (
                market::MarketPlugin,
                game_shop::GameShopPlugin,
                storage::StoragePlugin,
                sell_panel::SellPanelPlugin,
            ),
            item_rental_browse::ItemRentalBrowsePlugin,
        ));
    }
}

/// 登出/断线共用清理：清空对话框栈 + 复位孤儿弹窗（邀请/确认/输入框）驱动状态。
/// 资源跨 Game 状态存活——不复位则登出重进/断线重连后，旧邀请与旧确认框随新 spawn
/// 再次显示（8 月实机「一堆 UI 堆屏幕」堆积 bug 的状态层根因）。
///
/// 「state → sync_dialog_state」驱动资源复位清单（凡跨 Game 存活、每帧把自身
/// visible/open 同步回 DialogManager 的资源，漏复位 = 幽灵窗带旧数据重开）：
/// - TradeState（trade.rs 每帧 sync Trade/GuestTrade）：整体 default——visible/
///   my_items/their_items/is_initiator 任一残留都会让交易窗带旧物品重开、
///   点击往死会话发包；
/// - RollState（roll.rs 每帧 sync Roll）：整体 default；
/// - FishingState（fishing.rs 每帧 sync FishingStatus）：整体 default；
/// - MemoState（memo.rs 每帧 sync Memo）：整体 default；
/// - NpcDialogState（npc.rs 每帧 sync Npc）：整体 default；
/// - NpcGoodsState（npc_goods.rs 每帧 sync NpcGoods）：整体 default；
/// - InputBoxState（input_box.rs 每帧 sync InputBox）：open=false + purpose=None。
/// 新增「state → sync_dialog_state」驱动资源时必须加入本清单。
#[allow(clippy::too_many_arguments)]
fn clear_dialog_session(
    mgr: &mut DialogManager,
    mentor: &mut mentor::MentorState,
    relationship: &mut relationship::RelationshipState,
    trade: &mut trade::TradeState,
    roll: &mut roll::RollState,
    fishing: &mut fishing::FishingState,
    memo: &mut memo::MemoState,
    npc_dialog: &mut npc::NpcDialogState,
    npc_goods: &mut npc_goods::NpcGoodsState,
    inv_confirm: &mut inventory::InvDropConfirm,
    market_confirm: &mut market::MarketConfirm,
    npc_input: &mut npc::NpcInputState,
    input_box: &mut input_box::InputBoxState,
) {
    mgr.open.clear();
    mentor.invite = None;
    relationship.invite = None;
    *trade = trade::TradeState::default();
    *roll = roll::RollState::default();
    *fishing = fishing::FishingState::default();
    *memo = memo::MemoState::default();
    *npc_dialog = npc::NpcDialogState::default();
    *npc_goods = npc_goods::NpcGoodsState::default();
    *inv_confirm = inventory::InvDropConfirm::default();
    *market_confirm = market::MarketConfirm::default();
    *npc_input = npc::NpcInputState::default();
    input_box.open = false;
    input_box.purpose = input_box::InputPurpose::None;
}

/// 离开 Game（主动登出 / 服务端 ReturnToLogin / TCP 或 Mock 断线回登录）统一清理
/// 对话框与会话状态。挂 OnExit 而非轮询上升沿：旧实现 logout_server_events 靠
/// `net.reconnecting` 上升沿 + run_if(in_state(Game))，与 network_system 无排序——
/// 断线帧若它先跑则永远观测不到上升沿（下一帧已切 Login 被 run_if 挡掉），
/// 清理静默跳过。OnExit 与实体清理同帧同语义，三条离开路径必经，无时序孔。
#[allow(clippy::too_many_arguments)]
fn clear_dialog_session_on_exit(
    mut mgr: ResMut<DialogManager>,
    mut session: ResMut<crate::network::SessionState>,
    mut mentor: ResMut<mentor::MentorState>,
    mut relationship: ResMut<relationship::RelationshipState>,
    mut trade: ResMut<trade::TradeState>,
    mut roll: ResMut<roll::RollState>,
    mut fishing: ResMut<fishing::FishingState>,
    mut memo: ResMut<memo::MemoState>,
    mut npc_dialog: ResMut<npc::NpcDialogState>,
    mut npc_goods: ResMut<npc_goods::NpcGoodsState>,
    mut inv_confirm: ResMut<inventory::InvDropConfirm>,
    mut market_confirm: ResMut<market::MarketConfirm>,
    mut npc_input: ResMut<npc::NpcInputState>,
    mut input_box: ResMut<input_box::InputBoxState>,
) {
    clear_dialog_session(
        &mut mgr,
        &mut mentor,
        &mut relationship,
        &mut trade,
        &mut roll,
        &mut fishing,
        &mut memo,
        &mut npc_dialog,
        &mut npc_goods,
        &mut inv_confirm,
        &mut market_confirm,
        &mut npc_input,
        &mut input_box,
    );
    session.self_position = None;
    session.local_player_id = None;
    session.selected_index = None;
    tracing::info!("🧹 离开 Game：已清理对话框/会话");
}
