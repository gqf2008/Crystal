//! 全对话框插件 B0001 冒烟（批38-40 评审 P0 回归防线）
//!
//! B0001（同系统多查询共享组件且至少一方写、无互斥过滤）在**调度器初始化期**
//! panic：与实体是否实际重叠无关、`run_if` 只拦执行不拦初始化、`cargo check`
//! 与纯函数单测都拦不住——真实游戏首帧 `app.run()` 即崩（trade/minimap 曾中招，
//! 修复见 PR）。本测试逐插件注册到独立 App 并 `update()` 一帧：初始化 panic
//! 消息含 "B0001" 即失败并列出插件名与冲突查询。
//!
//! 非 B0001 panic（缺 Res/缺 State 等环境性失败）不视为失败——本测试只守
//! 查询访问集冲突。探针消息必须 `downcast` 取真实 payload：`{e:?}` 只打
//! `Any { .. }`，`contains("B0001")` 对任何 panic 恒真 → 假绿（实证过的坑）。

use bevy::app::App;

fn panic_msg(e: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = e.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = e.downcast_ref::<String>() {
        s.clone()
    } else {
        format!("{e:?}")
    }
}

#[test]
fn all_dialog_plugins_no_b0001_on_init() {
    let plugins: Vec<(&str, Box<dyn Fn(&mut App)>)> = vec![
        (
            "amount_box",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::amount_box::AmountBoxPlugin);
            }),
        ),
        (
            "assign_key",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::assign_key::AssignKeyPlugin);
            }),
        ),
        (
            "big_map",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::big_map::BigMapPlugin);
            }),
        ),
        (
            "buff",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::buff::BuffPlugin);
            }),
        ),
        (
            "character",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::character::CharacterDialogPlugin);
            }),
        ),
        (
            "chat_notice",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::chat_notice::ChatNoticePlugin);
            }),
        ),
        (
            "compass",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::compass::CompassPlugin);
            }),
        ),
        (
            "craft",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::craft::CraftPlugin);
            }),
        ),
        (
            "creature",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::creature::CreaturePlugin);
            }),
        ),
        (
            "dura",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::dura_status::DuraPlugin);
            }),
        ),
        (
            "fishing",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::fishing::FishingPlugin);
            }),
        ),
        (
            "friend",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::friend::FriendPlugin);
            }),
        ),
        (
            "game_shop",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::game_shop::GameShopPlugin);
            }),
        ),
        (
            "group",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::group::GroupPlugin);
            }),
        ),
        (
            "guild",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::guild::GuildPlugin);
            }),
        ),
        (
            "guild_territory",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::guild_territory::GuildTerritoryPlugin);
            }),
        ),
        (
            "help",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::help::HelpPlugin);
            }),
        ),
        (
            "hero",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::hero::HeroPlugin);
            }),
        ),
        (
            "hero_belt",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::hero_belt::HeroBeltPlugin);
            }),
        ),
        (
            "hero_equipment",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::hero_equipment::HeroEquipmentPlugin);
            }),
        ),
        (
            "hero_inventory",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::hero_inventory::HeroInventoryPlugin);
            }),
        ),
        (
            "hero_skills",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::hero_skills::HeroSkillPlugin);
            }),
        ),
        (
            "inspect",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::inspect::InspectPlugin);
            }),
        ),
        (
            "inventory",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::inventory::InventoryDialogPlugin);
            }),
        ),
        (
            "item_rental",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::item_rental::ItemRentalPlugin);
            }),
        ),
        (
            "keyboard",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::keyboard_layout::KeyboardPlugin);
            }),
        ),
        (
            "mail",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::mail::MailPlugin);
            }),
        ),
        (
            "market",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::market::MarketPlugin);
            }),
        ),
        (
            "mentor",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::mentor::MentorPlugin);
            }),
        ),
        (
            "menu",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::menu::MenuDialogPlugin);
            }),
        ),
        (
            "minimap",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::minimap::MiniMapPlugin);
            }),
        ),
        (
            "mount",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::mount::MountPlugin);
            }),
        ),
        (
            "notice",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::notice::NoticePlugin);
            }),
        ),
        (
            "npc",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::npc::NpcDialogPlugin);
            }),
        ),
        (
            "npc_awake",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::npc_awake::NpcAwakePlugin);
            }),
        ),
        (
            "npc_goods",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::npc_goods::NpcGoodsPlugin);
            }),
        ),
        (
            "option",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::option::OptionPlugin);
            }),
        ),
        (
            "potion_belt",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::potion_belt::PotionBeltPlugin);
            }),
        ),
        (
            "quest_log",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::quest_log::QuestLogPlugin);
            }),
        ),
        (
            "quest_tracking",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::quest_tracking::QuestTrackingPlugin);
            }),
        ),
        (
            "ranking",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::ranking::RankingPlugin);
            }),
        ),
        (
            "refine",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::refine::RefinePlugin);
            }),
        ),
        (
            "relationship",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::relationship::RelationshipPlugin);
            }),
        ),
        (
            "report",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::report::ReportPlugin);
            }),
        ),
        (
            "roll",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::roll::RollPlugin);
            }),
        ),
        (
            "sell_panel",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::sell_panel::SellPanelPlugin);
            }),
        ),
        (
            "socket",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::socket::SocketPlugin);
            }),
        ),
        (
            "storage",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::storage::StoragePlugin);
            }),
        ),
        (
            "text_input",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::text_input::TextInputPlugin);
            }),
        ),
        (
            "timer",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::timer::TimerPlugin);
            }),
        ),
        (
            "trade",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::trade::TradePlugin);
            }),
        ),
    ];

    run_plugins(&plugins, "全对话框插件");
}

/// 逐个插件注册到独立 App 并跑一帧，只报 B0001（其余 panic 视为环境性失败）。
fn run_plugins(plugins: &[(&str, Box<dyn Fn(&mut App)>)], label: &str) {
    let mut failures: Vec<String> = Vec::new();
    for (name, mk) in plugins {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut app = App::new();
            mk(&mut app);
            app.update();
        }));
        if let Err(e) = r {
            let msg = panic_msg(&e);
            if msg.contains("B0001") {
                failures.push(format!(
                    "[{name}] {}",
                    &msg.chars().take(260).collect::<String>()
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{label}存在 B0001 查询冲突（同系统多查询共享组件且无互斥，运行即崩）：\n{}",
        failures.join("\n---\n")
    );
}

/// 门禁（owner 队列 `b0001-ui-plugins`）：**ui:: 下的插件与 dialogs 的核心插件**此前一条都没进
/// B0001 冒烟——而 B0001 是系统初始化 panic，`cargo check` / `cargo test --lib` 都拦不住
/// （2026-09-24 的 P0「进游戏即崩」就是 `advance_spell_fx` 的查询冲突）。
///
/// 本测试补上：
/// - `DialogsPlugin`：主题/控件的核心系统（`scroll_list_ui_system`、`item_cell_ui_system`、
///   `animated_button_ui_system`、`dropdown_ui_system`、`checkbox_system`、`scrolling_label_system`…），
///   这是 `theme.rs` 那些共享 `Node`/`Interaction`/`ImageNode` 的系统唯一被纳入冒烟的地方；
/// - `ui::*` 各插件：`new_character`（同函数双 ParamSet，历史上是 B0001 高危形状）、
///   `select`/`login`/`intro`/`modal_box`/`client_settings`/`pinyin_ime`。
///
/// 阳性对照（红控，实做过）：在 `ui/theme.rs::scroll_list_ui_system` 的参数表里加一个
/// 无过滤的 `Query<&mut Node>`（与同系统的 `node_read: &Node, Without<UiScrollThumb>` /
/// `thumb_write: &mut Node, With<UiScrollThumb>` 无法证明不重叠）→ 本测试立即红：
///
/// ```text
/// [dialogs_core] error[B0001]: Query<'_, '_, &mut Node> in system
/// client_bevy::ui::theme::scroll_list_ui_system accesses component(s) Node in a way that
/// conflicts with a previous system parameter. …
/// ```
///
/// 恢复该参数即绿——说明本测试确实**初始化到了**这些核心系统，而不是空跑。
#[test]
fn ui_and_dialog_core_plugins_no_b0001_on_init() {
    let plugins: Vec<(&str, Box<dyn Fn(&mut App)>)> = vec![
        (
            "dialogs_core",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::game::dialogs::DialogsPlugin);
            }),
        ),
        (
            "ui_new_character",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::ui::new_character::NewCharacterPlugin);
            }),
        ),
        (
            "ui_select",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::ui::select::SelectPlugin);
            }),
        ),
        (
            "ui_modal_box",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::ui::modal_box::ModalBoxPlugin);
            }),
        ),
        (
            "ui_client_settings",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::ui::client_settings::ClientSettingsPlugin);
            }),
        ),
        (
            "ui_intro",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::ui::intro::IntroPlugin);
            }),
        ),
        (
            "ui_login",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::ui::login::LoginPlugin);
            }),
        ),
        (
            "ui_pinyin_ime",
            Box::new(move |app: &mut App| {
                app.add_plugins(client_bevy::ui::pinyin_ime::PinyinImePlugin);
            }),
        ),
    ];
    run_plugins(&plugins, "ui:: / dialogs 核心插件");
}
