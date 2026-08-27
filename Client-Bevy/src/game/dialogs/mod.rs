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
pub mod hero_skills;
pub mod inspect;
pub mod inventory;
pub mod item_rental;
pub mod keyboard_layout;
pub mod mail;
pub mod market;
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
pub mod socket;
pub mod storage;
pub mod text_input;
pub mod timer;
pub mod trade;

use bevy::prelude::*;

use crate::game::dialogs::text_input::TextInputRect;
use crate::scenes::AppState;
use crate::ui::theme::UiDropDown;

/// 对话框类型
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
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
    Hero,
    HeroInventory,
    HeroEquipment,
    HeroSkill,
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
    Skills,
}

/// 对话框管理（打开栈，栈顶在最前）
#[derive(Resource, Default)]
pub struct DialogManager {
    pub open: Vec<DialogKind>,
}

/// 恒可见标记：挂该组件的实体不随 `DialogManager.open` 门控显隐（如 C# DuraStatusDialog
/// 切换钮——对话框关闭也恒可见）。`enforce_dialog_visibility` 会跳过它。
#[derive(Component)]
pub struct AlwaysVisible;

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

    /// #1830：是否有窗口类对话框打开（小地图为无阻塞覆盖层，不屏蔽世界点击）
    pub fn blocks_world_click(&self) -> bool {
        self.open.iter().any(|k| !matches!(k, DialogKind::Minimap))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::window::{PrimaryWindow, Window};

    #[test]
    fn test_blocks_world_click() {
        let mut m = DialogManager::default();
        assert!(!m.blocks_world_click(), "空状态不屏蔽");
        m.open.push(DialogKind::Minimap);
        assert!(!m.blocks_world_click(), "小地图不屏蔽");
        m.open.push(DialogKind::Inventory);
        assert!(m.blocks_world_click(), "背包打开屏蔽");
        m.open.clear();
        m.open.push(DialogKind::Npc);
        assert!(m.blocks_world_click(), "NPC 对话打开屏蔽");
        m.open.clear();
        m.open.push(DialogKind::BigMap);
        assert!(m.blocks_world_click(), "大地图打开屏蔽");
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
}

/// 对话框根标记（OnExit(Game) 统一清理）
#[derive(Component)]
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
fn node_rect(node: &Node) -> (f32, f32, f32, f32) {
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
    mut dialogs: Query<(Entity, &DialogRoot, &Visibility, &mut Node, &GlobalZIndex)>,
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
                if cursor.x >= *minx
                    && cursor.x <= *maxx
                    && cursor.y >= *miny
                    && cursor.y <= *maxy
                {
                    if top.map(|(_, z)| *maxz > z).unwrap_or(true) {
                        top = Some((*kind, *maxz));
                    }
                }
            }
            if let Some((kind, _)) = top {
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
                    .filter(|(e, _)| roots.iter().any(|r| is_descendant_of(*e, *r, &mut |x| parents.get(x).ok().map(|c| c.parent()))))
                    .map(|(e, tr)| (e, (tr.0, tr.1)))
                    .collect();
                drag.dd_origins = drop_downs
                    .iter()
                    .filter(|(e, _)| roots.iter().any(|r| is_descendant_of(*e, *r, &mut |x| parents.get(x).ok().map(|c| c.parent()))))
                    .map(|(e, dd)| {
                        (
                            e,
                            ((dd.box_rect.0, dd.box_rect.1), (dd.popup_pos.0, dd.popup_pos.1)),
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
                    dd.box_rect.0 = o.0.0 + delta.x;
                    dd.box_rect.1 = o.0.1 + delta.y;
                    dd.popup_pos.0 = o.1.0 + delta.x;
                    dd.popup_pos.1 = o.1.1 + delta.y;
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

/// 通用对话框可见性兜底（#幽灵/泄漏）：PostUpdate 强制所有挂 `DialogRoot(kind)` 且
/// kind 不在 `DialogManager.open` 的实体隐藏。根治"未 open 却 Visible"的控件泄漏——
/// 各对话框 ui_system 若漏了部分子控件门控，会被这里兜底。跳过 `AlwaysVisible`。
fn enforce_dialog_visibility(
    mgr: Res<DialogManager>,
    mut q: Query<(&DialogRoot, &mut Visibility), Without<AlwaysVisible>>,
) {
    for (root, mut vis) in &mut q {
        if !mgr.is_open(root.0) && *vis == Visibility::Visible {
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
            enforce_dialog_visibility.run_if(in_state(AppState::Game)),
        );
        app.add_systems(
            Update,
            (dialog_front_system, dialog_drag_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
        // #182 登出：清理对话框与会话状态
        app.add_systems(
            Update,
            logout_server_events.run_if(in_state(AppState::Game)),
        );
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
                crate::ui::tooltip::tooltip_panel_system,
            )
                .run_if(in_state(AppState::Game)),
        );
        app.add_plugins(text_input::TextInputPlugin);
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
        ));
    }
}

/// #182 登出成功：清理对话框与会话状态（返回选角前）
fn logout_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut mgr: ResMut<DialogManager>,
    mut session: ResMut<crate::network::SessionState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::LogOutSuccess = ev {
            mgr.open.clear();
            session.self_position = None;
            session.local_player_id = None;
            session.selected_index = None;
            tracing::info!("🧹 登出：已清理对话框/会话");
        }
        if let ServerEvent::ReturnToLogin = ev {
            // #289：服务端要求返回登录界面，同样清理对话框/会话
            mgr.open.clear();
            session.self_position = None;
            session.local_player_id = None;
            session.selected_index = None;
            tracing::info!("🧹 返回登录：已清理对话框/会话");
        }
    }
}
