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
use bevy::sprite::Anchor;

use crate::game::dialogs::text_input::TextInputRect;
use crate::scenes::AppState;
use crate::ui::controls::DropDown;
use crate::ui::sprite_ui::{UiButton, UiEntity};

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
    use bevy::asset::Assets;
    use bevy::image::Image;
    use bevy::sprite::{Anchor, Sprite};
    use bevy::transform::components::Transform;

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

    #[test]
    fn ui_sprite_rect_top_left_anchor() {
        let mut assets = Assets::<Image>::default();
        let sprite = Sprite::sized(Vec2::new(100.0, 50.0));
        let tf = Transform::from_xyz(200.0, -150.0, 5.0);
        let (x0, y0, x1, y1) = ui_sprite_rect(&tf, Some(&sprite), Some(&Anchor::TOP_LEFT), &assets);
        assert_eq!((x0, y0, x1, y1), (200.0, 150.0, 300.0, 200.0));
    }

    #[test]
    fn ui_sprite_rect_center_anchor_shifts_half_size() {
        let mut assets = Assets::<Image>::default();
        let sprite = Sprite::sized(Vec2::new(100.0, 50.0));
        let tf = Transform::from_xyz(200.0, -150.0, 5.0);
        let (x0, y0, x1, y1) = ui_sprite_rect(&tf, Some(&sprite), Some(&Anchor::CENTER), &assets);
        assert_eq!((x0, y0, x1, y1), (150.0, 125.0, 250.0, 175.0));
    }

    #[test]
    fn ui_sprite_rect_without_sprite_falls_back_to_point() {
        let mut assets = Assets::<Image>::default();
        let tf = Transform::from_xyz(200.0, -150.0, 5.0);
        let rect = ui_sprite_rect(&tf, None, None, &assets);
        assert_eq!(rect, (200.0, 150.0, 200.0, 150.0));
    }
}

/// 对话框根标记（OnExit(Game) 统一清理）
#[derive(Component)]
pub struct DialogRoot(pub DialogKind);

/// 弹窗拖动状态（#34：原版弹窗可拖动）
#[derive(Resource, Default)]
pub struct DialogDrag {
    /// 正在拖动的对话框类型
    pub dragging: Option<DialogKind>,
    /// 拖动开始时的鼠标位置（逻辑坐标）
    pub start_cursor: Vec2,
    /// 拖动开始时各实体的原始 Transform（用于增量位移）
    pub origins: std::collections::HashMap<Entity, Vec3>,
    /// 拖动开始时按钮命中矩形的原始位置（拖动后同步，否则按钮/关闭点击失效）
    pub btn_origins: std::collections::HashMap<Entity, (f32, f32)>,
    /// 拖动开始时文本输入框命中矩形的原始位置
    pub text_origins: std::collections::HashMap<Entity, (f32, f32)>,
    /// 拖动开始时下拉框命中矩形的原始位置（box_rect + popup_pos）
    pub dd_origins: std::collections::HashMap<Entity, ((f32, f32), (f32, f32))>,
    /// 拖动开始时的背包命中原点（仅拖 Inventory 时 Some）
    pub inv_origin_start: Option<(f32, f32)>,
}

/// 对话框置顶层级（front 系统维护单调递增的 z 顶值）
#[derive(Resource, Default)]
pub struct DialogZ {
    pub top: f32,
}

/// 计算 UI 实体在屏幕坐标下的包围盒（屏幕 y 向下）。
/// 没有 Sprite 的实体退化为其 Transform 点，保持原行为。
fn ui_sprite_rect(
    tf: &Transform,
    sprite: Option<&Sprite>,
    anchor: Option<&Anchor>,
    image_assets: &Assets<Image>,
) -> (f32, f32, f32, f32) {
    let Some(sprite) = sprite else {
        let (x, y) = (tf.translation.x, -tf.translation.y);
        return (x, y, x, y);
    };
    let size = sprite
        .custom_size
        .or_else(|| {
            image_assets.get(&sprite.image).map(|img| {
                let s = img.size();
                Vec2::new(s.x as f32, s.y as f32)
            })
        })
        .unwrap_or(Vec2::ZERO);
    let anchor_vec = anchor
        .map(|a| a.as_vec())
        .unwrap_or(Anchor::TOP_LEFT.as_vec());
    // Bevy Sprite 中心 = -anchor * size；左上角 = 中心 + (-0.5, 0.5) * size
    let local_top_left = (-anchor_vec + Vec2::new(-0.5, 0.5)) * size;
    let tl = tf.translation.truncate() + local_top_left;
    (tl.x, -tl.y, tl.x + size.x, -tl.y + size.y)
}

/// 通用弹窗拖动系统：
/// - 按 DialogKind 聚合实体，用实体位置估算窗口包围盒，按住任意位置（非按钮）可拖
/// - 拖动时对所有该对话框实体整体平移（保持相对布局）
pub fn dialog_drag_system(
    mut drag: ResMut<DialogDrag>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    ui_cameras: Query<(&Camera, &GlobalTransform), With<UiEntity>>,
    image_assets: Res<Assets<Image>>,
    mut dialogs: Query<(
        Entity,
        &DialogRoot,
        &Visibility,
        &mut Transform,
        Option<&Sprite>,
        Option<&Anchor>,
    )>,
    mut ui_buttons: Query<(Entity, &mut UiButton, Option<&DialogRoot>)>,
    mut text_rects: Query<(Entity, &mut TextInputRect, Option<&DialogRoot>)>,
    mut drop_downs: Query<(Entity, &mut DropDown, Option<&DialogRoot>)>,
    // 背包拖动时同步 InventoryOrigin（inv_slot_at/仓库/交易命中依赖它）
    mut inv_origin: ResMut<crate::game::dialogs::inventory::InventoryOrigin>,
) {
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok((ui_camera, cam_tf)) = ui_cameras.single() else {
        return;
    };
    let Ok(world) = ui_camera.viewport_to_world_2d(cam_tf, cursor) else {
        return;
    };
    let cursor = Vec2::new(world.x, -world.y);

    let mut boxes: std::collections::HashMap<DialogKind, (f32, f32, f32, f32, f32)> =
        std::collections::HashMap::new();
    for (_, root, vis, tf, sprite, anchor) in dialogs.iter() {
        if *vis != Visibility::Visible {
            continue;
        }
        let (x0, y0, x1, y1) = ui_sprite_rect(tf, sprite, anchor, &image_assets);
        let b = boxes
            .entry(root.0)
            .or_insert((x0, y0, x1, y1, tf.translation.z));
        b.0 = b.0.min(x0);
        b.1 = b.1.min(y0);
        b.2 = b.2.max(x1);
        b.3 = b.3.max(y1);
        b.4 = b.4.max(tf.translation.z);
    }

    if mouse.just_pressed(MouseButton::Left) && drag.dragging.is_none() {
        let on_button = ui_buttons.iter().any(|(_, b, _)| {
            let (x, y, w, h) = b.rect;
            cursor.x >= x && cursor.x <= x + w && cursor.y >= y && cursor.y <= y + h
        });
        if !on_button {
            let mut top: Option<(DialogKind, f32)> = None;
            for (kind, (minx, miny, maxx, maxy, maxz)) in &boxes {
                if cursor.x >= *minx && cursor.x <= *maxx && cursor.y >= *miny && cursor.y <= *maxy
                {
                    if top.map(|(_, z)| *maxz > z).unwrap_or(true) {
                        top = Some((*kind, *maxz));
                    }
                }
            }
            if let Some((kind, _)) = top {
                let origins = dialogs
                    .iter()
                    .filter(|(_, r, v, _, _, _)| *v == Visibility::Visible && r.0 == kind)
                    .map(|(e, _, _, tf, _, _)| (e, tf.translation))
                    .collect::<std::collections::HashMap<_, _>>();
                drag.dragging = Some(kind);
                drag.start_cursor = cursor;
                drag.origins = origins;
                drag.inv_origin_start =
                    (kind == DialogKind::Inventory).then(|| (inv_origin.0, inv_origin.1));
                drag.btn_origins = ui_buttons
                    .iter()
                    .filter(|(_, _, r)| r.map(|r| r.0) == Some(kind))
                    .map(|(e, btn, _)| (e, (btn.rect.0, btn.rect.1)))
                    .collect();
                drag.text_origins = text_rects
                    .iter()
                    .filter(|(_, _, r)| r.map(|r| r.0) == Some(kind))
                    .map(|(e, tr, _)| (e, (tr.0, tr.1)))
                    .collect();
                drag.dd_origins = drop_downs
                    .iter()
                    .filter(|(_, _, r)| r.map(|r| r.0) == Some(kind))
                    .map(|(e, dd, _)| {
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
            for (e, _, vis, mut tf, _, _) in dialogs.iter_mut() {
                if *vis == Visibility::Visible {
                    if let Some(orig) = drag.origins.get(&e) {
                        tf.translation = *orig + Vec3::new(delta.x, -delta.y, 0.0);
                    }
                }
            }
            for (e, mut btn, _) in ui_buttons.iter_mut() {
                if let Some(o) = drag.btn_origins.get(&e) {
                    btn.rect.0 = o.0 + delta.x;
                    btn.rect.1 = o.1 + delta.y;
                }
            }
            for (e, mut tr, _) in text_rects.iter_mut() {
                if let Some(o) = drag.text_origins.get(&e) {
                    tr.0 = o.0 + delta.x;
                    tr.1 = o.1 + delta.y;
                }
            }
            for (e, mut dd, _) in drop_downs.iter_mut() {
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
            drag.origins.clear();
            drag.btn_origins.clear();
            drag.text_origins.clear();
            drag.dd_origins.clear();
            drag.inv_origin_start = None;
        }
    }
}

/// 置顶系统：
/// - 新打开的对话框 → 置顶（对齐 C# Show → BringToFront）
/// - 点击可见对话框 → 置顶（对齐 C# 点击窗口置前）
pub fn dialog_front_system(
    mut z: ResMut<DialogZ>,
    mgr: Res<DialogManager>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    ui_cameras: Query<(&Camera, &GlobalTransform), With<UiEntity>>,
    image_assets: Res<Assets<Image>>,
    mut dialogs: Query<(
        Entity,
        &DialogRoot,
        &Visibility,
        &mut Transform,
        Option<&Sprite>,
        Option<&Anchor>,
    )>,
    mut prev_open: Local<Vec<DialogKind>>,
) {
    if z.top < 30.0 {
        z.top = 30.0;
    }

    if let Some(kind) = mgr.open.last().copied().filter(|k| !prev_open.contains(k)) {
        bump_dialog_z(kind, &mut z, &mut dialogs);
    }
    *prev_open = mgr.open.clone();

    if mouse.just_pressed(MouseButton::Left) {
        let Ok(window) = windows.single() else { return };
        let Some(cursor) = window.cursor_position() else {
            return;
        };
        let Ok((cam, gtf)) = ui_cameras.single() else {
            return;
        };
        let Ok(world) = cam.viewport_to_world_2d(gtf, cursor) else {
            return;
        };
        let cursor = Vec2::new(world.x, -world.y);

        let mut boxes: std::collections::HashMap<DialogKind, (f32, f32, f32, f32, f32)> =
            std::collections::HashMap::new();
        for (_, root, vis, tf, sprite, anchor) in dialogs.iter() {
            if *vis != Visibility::Visible {
                continue;
            }
            let (x0, y0, x1, y1) = ui_sprite_rect(tf, sprite, anchor, &image_assets);
            let b = boxes
                .entry(root.0)
                .or_insert((x0, y0, x1, y1, tf.translation.z));
            b.0 = b.0.min(x0);
            b.1 = b.1.min(y0);
            b.2 = b.2.max(x1);
            b.3 = b.3.max(y1);
            b.4 = b.4.max(tf.translation.z);
        }
        let mut best: Option<(DialogKind, f32)> = None;
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

/// 把指定对话框整体平移到置顶 z（保留内部相对层级，隐藏子页也一起平移）
fn bump_dialog_z(
    kind: DialogKind,
    z: &mut DialogZ,
    dialogs: &mut Query<(
        Entity,
        &DialogRoot,
        &Visibility,
        &mut Transform,
        Option<&Sprite>,
        Option<&Anchor>,
    )>,
) {
    let mut min_z = f32::MAX;
    let mut any = false;
    for (_, r, _, tf, _, _) in dialogs.iter() {
        if r.0 == kind {
            min_z = min_z.min(tf.translation.z);
            any = true;
        }
    }
    if !any {
        return;
    }
    let top = z.top;
    z.top += 10.0;
    for (_, r, _, mut tf, _, _) in dialogs.iter_mut() {
        if r.0 == kind {
            tf.translation.z = tf.translation.z - min_z + top;
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
