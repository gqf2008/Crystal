// ============================================================================
// 对话框系统（M9）
// 交互参考：Client/MirScenes/Dialogs/*.cs（原版 C#）
// 绘制参考：Client-Macroquad/src/scenes/dialogs/game/*.rs
// 框架：DialogManager 维护打开栈（z 序），每个对话框一个插件子模块
// ============================================================================

pub mod amount_box;
pub mod assign_key;
pub mod belt;
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
pub mod hero_inventory;
pub mod hero_equipment;
pub mod hero_skills;
pub mod inspect;
pub mod inventory;
pub mod item_rental;
pub mod keyboard_layout;
pub mod mail;
pub mod market;
pub mod menu;
pub mod mentor;
pub mod minimap;
pub mod mount;
pub mod notice;
pub mod npc;
pub mod npc_awake;
pub mod npc_drop;
pub mod npc_goods;
pub mod option;
pub mod potion_belt;
pub mod quest_log;
pub mod quest_tracking;
pub mod settings_file;
pub mod ranking;
pub mod refine;
pub mod relationship;
pub mod report;
pub mod roll;
 pub mod sell_panel;
pub mod socket;
 pub mod storage;
pub mod text_input;
pub mod timer;
pub mod trade;
pub mod trust_merchant;

use bevy::prelude::*;

use crate::scenes::AppState;
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
    TrustMerchant,
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
    NpcDrop,
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
}

/// 通用弹窗拖动系统：
/// - 按 DialogKind 聚合实体，用实体位置估算窗口包围盒，标题栏（顶部 28px）可拖
/// - 拖动时对所有该对话框实体整体平移（保持相对布局）
pub fn dialog_drag_system(
    mut drag: ResMut<DialogDrag>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    ui_cameras: Query<(&Camera, &GlobalTransform), With<UiEntity>>,
    buttons: Query<&UiButton>,
    mut dialogs: Query<(Entity, &DialogRoot, &Visibility, &mut Transform)>,
) {
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    // UI 相机是 Fixed 1024x768：窗口被缩放/最大化时 cursor_position 与 UI 世界坐标不一致，
    // 必须经相机换算（viewport_to_world_2d）得到 UI 世界坐标再参与命中。
    let Ok((ui_camera, cam_tf)) = ui_cameras.single() else { return };
    let Ok(world) = ui_camera.viewport_to_world_2d(cam_tf, cursor) else { return };
    // UI 世界坐标 y 向下（实体 Transform 用 -y），转回屏幕正坐标便于包围盒计算
    let cursor = Vec2::new(world.x, -world.y);

    // 聚合每个可见对话框的包围盒（隐藏窗口不参与命中，避免拖错/拖到不可见窗口）
    let mut boxes: std::collections::HashMap<DialogKind, (f32, f32, f32, f32)> =
        std::collections::HashMap::new();
    for (_, root, vis, tf) in dialogs.iter() {
        if *vis != Visibility::Visible {
            continue;
        }
        let (x, y) = (tf.translation.x, -tf.translation.y);
        let b = boxes.entry(root.0).or_insert((x, y, x, y));
        b.0 = b.0.min(x);
        b.1 = b.1.min(y);
        b.2 = b.2.max(x);
        b.3 = b.3.max(y);
    }

    // 点击标题栏开始拖动（按钮命中区不触发拖动，避免点关闭/翻页等误拖）
    if mouse.just_pressed(MouseButton::Left) && drag.dragging.is_none() {
        let on_button = buttons.iter().any(|b| {
            let (x, y, w, h) = b.rect;
            cursor.x >= x && cursor.x <= x + w && cursor.y >= y && cursor.y <= y + h
        });
        if !on_button {
            for (kind, (minx, miny, maxx, maxy)) in &boxes {
                // 标题栏：窗口顶部 28px
                if cursor.x >= *minx && cursor.x <= *maxx && cursor.y >= *miny && cursor.y <= *miny + 28.0 {
                    let origins = dialogs
                        .iter()
                        .filter(|(_, r, v, _)| *v == Visibility::Visible && r.0 == *kind)
                        .map(|(e, _, _, tf)| (e, tf.translation))
                        .collect::<std::collections::HashMap<_, _>>();
                    drag.dragging = Some(*kind);
                    drag.start_cursor = cursor;
                    drag.origins = origins;
                    tracing::info!("🖱️ 拖动对话框 {:?}", kind);
                    break;
                }
            }
        }
    }

    // 拖动中：整体平移
    if let Some(kind) = drag.dragging {
        if mouse.pressed(MouseButton::Left) {
            let delta = cursor - drag.start_cursor;
            for (e, _, vis, mut tf) in dialogs.iter_mut() {
                if *vis == Visibility::Visible {
                    if let Some(orig) = drag.origins.get(&e) {
                        tf.translation = *orig + Vec3::new(delta.x, -delta.y, 0.0);
                    }
                }
            }
        } else {
            // 松开结束
            drag.dragging = None;
            drag.origins.clear();
        }
    }
}


pub struct DialogsPlugin;

impl Plugin for DialogsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DialogManager>();
        app.init_resource::<DialogDrag>();
        app.add_plugins(hero_equipment::HeroEquipmentPlugin);
        app.add_plugins(hero_skills::HeroSkillPlugin);
                app.add_systems(Update, dialog_drag_system.run_if(in_state(AppState::Game)));
        // #182 登出：清理对话框与会话状态
        app.add_systems(Update, logout_server_events.run_if(in_state(AppState::Game)));
        app.add_systems(Update, crate::ui::scroll_list::scroll_list_system.run_if(in_state(AppState::Game)));
        app.add_systems(Update, (crate::ui::controls::checkbox_system, crate::ui::controls::dropdown_system, crate::ui::controls::scrolling_label_system, crate::ui::controls::item_cell_system, crate::ui::controls::animated_button_system).run_if(in_state(AppState::Game)));
        app.init_resource::<crate::ui::keyboard_nav::KeyboardNav>();
        app.init_resource::<crate::ui::scroll_list::ScrollDrag>();
        app.add_systems(Update, (crate::ui::keyboard_nav::esc_close_dialogs_system, crate::ui::keyboard_nav::keyboard_scroll_lists_system, crate::ui::keyboard_nav::tab_focus_system).run_if(in_state(AppState::Game)));
        // #93 通用 Tooltip
        app.init_resource::<crate::ui::tooltip::TooltipState>();
        app.add_systems(OnEnter(AppState::Game), crate::ui::tooltip::spawn_tooltip_panel_system);
        app.add_systems(OnExit(AppState::Game), crate::ui::tooltip::despawn_tooltip_panel);
        app.add_systems(Update, (crate::ui::tooltip::tooltip_hint_system, crate::ui::tooltip::tooltip_panel_system).run_if(in_state(AppState::Game)));
        app.init_resource::<inventory::InventoryState>();
        app.add_plugins(text_input::TextInputPlugin);
        app.init_resource::<character::CharacterState>();
        app.add_plugins((
            (
                inventory::InventoryDialogPlugin,
                assign_key::AssignKeyPlugin,
                character::CharacterDialogPlugin,
                menu::MenuDialogPlugin,
                minimap::MiniMapPlugin,
                belt::BeltPlugin,
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
                hero_inventory::HeroInventoryPlugin,
                creature::CreaturePlugin,
                trust_merchant::TrustMerchantPlugin,
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
                npc_drop::NpcDropPlugin,
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

