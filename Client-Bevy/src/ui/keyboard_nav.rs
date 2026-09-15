// ============================================================================
// keyboard_nav - 对话框键盘交互（#92）
// 参考：C# KeyBindSettings Closeall（ESC 关闭全部窗口）+ MirControl 键盘导航
//   - ESC：关闭所有打开对话框（C# Closeall 语义）
//   - ↑/↓/PageUp/PageDown：滚动最上层打开对话框的列表（ScrollList）
//   - Tab/Shift+Tab：在最上层对话框按钮间切换焦点，Enter 触发点击
//     （click_remaining 保证对话框系统无论顺序都能读到点击）
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogManager, DialogRoot};
use crate::ui::scroll_list::ScrollList;
use crate::ui::sprite_ui::{UiButton, UiEntity};

/// 键盘导航状态
#[derive(Resource, Default)]
pub struct KeyboardNav {
    /// 当前聚焦按钮
    pub focused: Option<Entity>,
    /// Enter 触发后保持 clicked=true 的剩余帧数
    pub click_remaining: u8,
    /// 焦点高亮框实体
    pub highlight: Option<Entity>,
}

/// ESC 优先级层级（#2595/#2604，C# WinForms 焦点路由 + MirTextBox.cs:386-395：
/// 模态最顶层先消费 Esc 且 e.Handled，不向底下层传导）：
/// 1. 聊天输入开 → 本系统让路（chat_input_system 同帧关闭输入行；
///    注册处 .before(chat_input_system) 保证这里先看到 input_active=true）；
///    C# TextBox_KeyPress Escape → ActiveControl=null 且 e.Handled，不触发 Closeall
/// 2. 数量输入框（AmountBox 模态）开 → 让路（amount_box_system 自己消费
///    Esc=Cancel/Enter=OK；旧实现不检查 → Esc 把数量框后面的全部对话框
///    关掉而数量框还开着，#2598 审查遗留）
/// 3. 通用输入框聚焦 → 只取消聚焦（对话框不动）
/// 4. 玩家右键菜单开着 → 让路（player_menu_ui_system 自己关；同理不叠发）
/// 5. 无输入聚焦 → Closeall（C# KeyBindSettings Closeall）
pub fn esc_close_dialogs_system(
    keys: Res<ButtonInput<KeyCode>>,
    chat: Res<crate::game::chat::ChatState>,
    amount: Res<crate::game::dialogs::amount_box::AmountBoxState>,
    player_menu: Res<crate::game::player_menu::PlayerMenuState>,
    mut mgr: ResMut<DialogManager>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    // #2836 单元③：C# `HeroManageDialog?.Hide()`（`GameScene.cs:707`）——本端该窗状态驱动
    mut hero: ResMut<crate::game::dialogs::hero::HeroState>,
    // #2892：C# `GameScene.cs:698` —— ESC 只在「ESC 退出」勾选时取消钓鱼
    fishing: Res<crate::game::dialogs::fishing::FishingState>,
    net: Res<crate::network::NetConnection>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    if chat.input_active {
        return;
    }
    if amount.visible {
        return;
    }
    if input.active.is_some() {
        input.active = None;
        return;
    }
    if player_menu.visible {
        return;
    }
    // #2836 单元③：按 C# `KeybindOptions.Closeall` 的**集合**关闭（`GameScene.cs:668-711`），
    // 不再 blanket `open.clear()` —— 原版 ESC **不关** 交易窗/计时器/Buff/小地图/耐久面板/
    // 镶嵌窗/聊天公告/租赁双方窗（见 `dialogs::CLOSEALL_DIRECT` 注释）。
    // #2892：C# `GameScene.cs:698` —— `if (FishingStatusDialog.bEscExit) FishingStatusDialog.Cancel();`
    // （钓鱼**状态窗不在** Closeall 集合里：主窗由下面的 Closeall 关，
    //  状态窗只在勾选「ESC 退出」时取消钓鱼 → 服务端回 `Fishing=false` 才消失）
    if fishing.esc_exit && mgr.is_open(crate::game::dialogs::DialogKind::FishingStatus) {
        crate::game::dialogs::fishing::cancel_fishing(&net, &mut mgr);
        tracing::info!("⌨️ ESC 取消钓鱼（ESC 退出已勾选）");
    }
    let (mut managing, mut confirm_slot) = (hero.managing, hero.confirm_slot);
    if crate::game::dialogs::closeall(&mut mgr, &mut managing, &mut confirm_slot) {
        hero.managing = managing;
        hero.confirm_slot = confirm_slot;
        tracing::info!("⌨️ ESC 关闭对话框（C# Closeall 集合）");
    }
}

/// ↑/↓/PageUp/PageDown：滚动最上层打开对话框的 ScrollList
/// （#2595：文本输入聚焦时让路——箭头键进文本框，C# 焦点路由）
pub fn keyboard_scroll_lists_system(
    keys: Res<ButtonInput<KeyCode>>,
    gate: Res<crate::game::input_gate::TextInputGate>,
    mgr: Res<DialogManager>,
    mut lists: Query<(&mut ScrollList, &DialogRoot)>,
) {
    if gate.0 {
        return;
    }
    let Some(top) = mgr.open.last().copied() else {
        return;
    };
    let delta = if keys.just_pressed(KeyCode::ArrowUp) {
        -1
    } else if keys.just_pressed(KeyCode::ArrowDown) {
        1
    } else if keys.just_pressed(KeyCode::PageUp) {
        -3
    } else if keys.just_pressed(KeyCode::PageDown) {
        3
    } else {
        0
    };
    if delta == 0 {
        return;
    }
    for (mut list, root) in &mut lists {
        if root.0 != top {
            continue;
        }
        let max = list.max_offset() as i32;
        list.offset = (list.offset as i32 + delta).clamp(0, max) as usize;
        return;
    }
}

/// Tab/Shift+Tab 焦点切换 + Enter 触发点击 + 焦点高亮框
/// （#2595：文本输入聚焦时让路——C# 聚焦 TextBox 时 Tab 转发给游戏键位
/// （拾取，MainDialogs.cs:1160-1185），Enter 归输入框；对话框导航是移植附加，
/// 打字时整体让路，仅清理残留焦点/高亮）
pub fn tab_focus_system(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    gate: Res<crate::game::input_gate::TextInputGate>,
    mut nav: ResMut<KeyboardNav>,
    mgr: Res<DialogManager>,
    mut images: ResMut<Assets<Image>>,
    mut buttons: Query<(
        Entity,
        &mut UiButton,
        Option<&DialogRoot>,
        &InheritedVisibility,
    )>,
    mut highlight_q: Query<(&mut Transform, &mut Sprite), Without<UiButton>>,
) {
    if gate.0 {
        nav.focused = None;
        nav.click_remaining = 0;
        if let Some(he) = nav.highlight {
            if let Ok((mut tf, _)) = highlight_q.get_mut(he) {
                tf.translation.x = -9999.0;
                tf.translation.y = -9999.0;
            }
        }
        return;
    }
    // 收集最上层打开对话框的按钮（按位置 y 排序便于上下导航）
    let top = mgr.open.last().copied();
    let mut cands: Vec<(Entity, (f32, f32, f32, f32))> = buttons
        .iter()
        .filter(|(_, _, root, inherited)| {
            inherited.get() && root.map(|r| Some(r.0) == top).unwrap_or(false)
        })
        .map(|(e, b, _, _)| (e, b.rect))
        .collect();
    cands.sort_by(|a, b| {
        a.1 .1
            .partial_cmp(&b.1 .1)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if cands.is_empty() {
        nav.focused = None;
        return;
    }

    // Tab/Shift+Tab 移动焦点
    if keys.just_pressed(KeyCode::Tab) {
        let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
        let cur = nav
            .focused
            .and_then(|f| cands.iter().position(|(e, _)| *e == f));
        let next = match cur {
            Some(i) if shift => (i + cands.len() - 1) % cands.len(),
            Some(i) => (i + 1) % cands.len(),
            None => 0,
        };
        nav.focused = Some(cands[next].0);
        tracing::info!("⌨️ Tab 焦点切换");
    }

    // Enter 触发点击（保持几帧，避免对话框系统顺序导致漏读）
    if keys.just_pressed(KeyCode::Enter) && nav.focused.is_some() {
        nav.click_remaining = 3;
    }
    if nav.click_remaining > 0 {
        if let Some(f) = nav.focused {
            if let Ok((_, mut b, _, _)) = buttons.get_mut(f) {
                b.clicked = true;
            }
        }
        nav.click_remaining -= 1;
    }

    // 焦点高亮框
    let Some(f) = nav.focused else {
        if let Some(he) = nav.highlight {
            if let Ok((mut tf, _)) = highlight_q.get_mut(he) {
                tf.translation.x = -9999.0;
                tf.translation.y = -9999.0;
            }
        }
        return;
    };
    let Ok((_, b, _, _)) = buttons.get(f) else {
        return;
    };
    let (x, y, w, h) = b.rect;
    if nav.highlight.is_none() {
        let white = images.add(crate::map_renderer::make_image(
            vec![255, 255, 255, 255],
            1,
            1,
        ));
        let e = commands
            .spawn((
                UiEntity,
                Sprite {
                    image: white,
                    color: Color::srgba(1.0, 0.9, 0.2, 0.5),
                    custom_size: Some(Vec2::new(w + 2.0, h + 2.0)),
                    ..default()
                },
                bevy::sprite::Anchor::TOP_LEFT,
                Transform::from_xyz(x - 1.0, -(y - 1.0), 20.0),
                Visibility::Visible,
            ))
            .id();
        nav.highlight = Some(e);
    }
    if let Some(he) = nav.highlight {
        if let Ok((mut tf, mut sp)) = highlight_q.get_mut(he) {
            tf.translation.x = x - 1.0;
            tf.translation.y = -(y - 1.0);
            if let Some(cs) = sp.custom_size.as_mut() {
                *cs = Vec2::new(w + 2.0, h + 2.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::chat::ChatState;
    use crate::game::dialogs::text_input::TextInputState;
    use bevy::input::ButtonInput;

    fn esc_app(chat_open: bool, text_active: Option<usize>) -> App {
        esc_app_ext(chat_open, text_active, false, false)
    }

    /// #2892：C# `GameScene.cs:698` —— ESC 只在「ESC 退出」（`FishingStatusDialog.bEscExit`）
    /// 勾选时取消钓鱼（`Cancel()`：发 `C.FishingCast{CastOut=false}` + 关状态窗）；
    /// 未勾选时状态窗不在 Closeall 集合里，ESC 不动它。
    ///
    /// 阳性对照：把键盘处理里的 `fishing.esc_exit &&` 条件去掉（= 无条件取消）→
    /// 本测试第一条断言（未勾选时状态窗应保持打开）FAILED。
    #[test]
    fn esc_cancels_fishing_only_when_esc_exit_checked() {
        use crate::game::dialogs::fishing::FishingState;
        use crate::game::dialogs::DialogKind;
        // 未勾选：状态窗保持打开
        let mut app = esc_app_ext(false, None, false, false);
        app.world_mut()
            .resource_mut::<DialogManager>()
            .open
            .push(DialogKind::FishingStatus);
        app.update();
        assert!(
            app.world()
                .resource::<DialogManager>()
                .is_open(DialogKind::FishingStatus),
            "未勾选 ESC 退出时 ESC 不应取消钓鱼"
        );
        // 勾选后：状态窗被取消关闭
        let mut app = esc_app_ext(false, None, false, false);
        app.world_mut()
            .resource_mut::<DialogManager>()
            .open
            .push(DialogKind::FishingStatus);
        app.world_mut().resource_mut::<FishingState>().esc_exit = true;
        app.update();
        assert!(
            !app.world()
                .resource::<DialogManager>()
                .is_open(DialogKind::FishingStatus),
            "勾选 ESC 退出后 ESC 应取消钓鱼并关闭状态窗"
        );
    }

    /// #2604：扩展 Esc 层级测试——amount/player_menu 开关
    fn esc_app_ext(
        chat_open: bool,
        text_active: Option<usize>,
        amount_open: bool,
        menu_open: bool,
    ) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<ButtonInput<KeyCode>>();
        app.insert_resource(ChatState {
            input_active: chat_open,
            ..Default::default()
        });
        app.init_resource::<TextInputState>();
        app.init_resource::<DialogManager>();
        app.init_resource::<crate::game::dialogs::amount_box::AmountBoxState>();
        app.init_resource::<crate::game::player_menu::PlayerMenuState>();
        // #2836 单元③：ESC 现按 C# Closeall 集合关窗，系统新增 `ResMut<HeroState>`
        app.init_resource::<crate::game::dialogs::hero::HeroState>();
        // #2892：ESC 还读钓鱼「ESC 退出」勾选（C# `GameScene.cs:698`）→ 需钓鱼状态与网络连接
        app.init_resource::<crate::game::dialogs::fishing::FishingState>();
        app.init_resource::<crate::network::NetConnection>();
        app.add_systems(Update, esc_close_dialogs_system);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        let mut mgr = app.world_mut().resource_mut::<DialogManager>();
        mgr.open.push(crate::game::dialogs::DialogKind::Inventory);
        if let Some(id) = text_active {
            app.world_mut().resource_mut::<TextInputState>().active = Some(id);
        }
        if amount_open {
            app.world_mut()
                .resource_mut::<crate::game::dialogs::amount_box::AmountBoxState>()
                .visible = true;
        }
        if menu_open {
            app.world_mut()
                .resource_mut::<crate::game::player_menu::PlayerMenuState>()
                .visible = true;
        }
        app
    }

    /// #2595 Esc 三级优先级（C# MirTextBox.cs:386-395）：
    /// 聊天输入开 → esc_close 让路（对话框不动，输入行由 chat_input_system 关）
    #[test]
    fn esc_yields_to_chat_input() {
        let mut app = esc_app(true, None);
        app.update();
        assert_eq!(
            app.world().resource::<DialogManager>().open.len(),
            1,
            "聊天输入开时 Esc 不应关对话框"
        );
        assert!(
            app.world().resource::<ChatState>().input_active,
            "esc_close 不动聊天输入（由 chat_input_system 同帧关）"
        );
    }

    /// 通用输入框聚焦 → 只取消聚焦，对话框不动
    #[test]
    fn esc_clears_generic_input_only() {
        let mut app = esc_app(false, Some(0));
        app.update();
        assert!(
            app.world().resource::<TextInputState>().active.is_none(),
            "Esc 应取消通用输入框聚焦"
        );
        assert_eq!(
            app.world().resource::<DialogManager>().open.len(),
            1,
            "输入框聚焦时 Esc 不应关对话框"
        );
    }

    /// 无任何输入聚焦 → Closeall（C# KeyBindSettings Closeall）
    #[test]
    fn esc_closes_all_dialogs() {
        let mut app = esc_app(false, None);
        app.update();
        assert!(
            app.world().resource::<DialogManager>().open.is_empty(),
            "无输入聚焦时 Esc 应关闭全部对话框"
        );
    }

    /// #2836 单元③：ESC 只关 C# `Closeall` 集合里的窗口（`GameScene.cs:668-711` + `NPCDialog.Hide()`
    /// 级联 `NPCDialogs.cs:1026-1037`）——**交易窗/计时器/Buff/小地图/耐久/镶嵌/聊天公告/租赁双方窗
    /// 原版 ESC 不关**，本端此前 blanket `open.clear()` 会一律关掉。
    #[test]
    fn esc_closes_closeall_set_only() {
        use crate::game::dialogs::DialogKind as K;

        let mut app = esc_app(false, None);
        {
            let mut mgr = app.world_mut().resource_mut::<DialogManager>();
            mgr.open.clear();
            // C# 直接关的 + NPC 级联（需 Npc 开着才级联）
            for k in [
                K::Inventory,
                K::Character,
                K::Mail,
                K::Npc,
                K::NpcGoods, // 级联
                K::Market,   // 级联
            ] {
                mgr.open(k);
            }
            // 原版 ESC **不关** 的
            for k in [
                K::Trade,
                K::GuestTrade,
                K::Timer,
                K::Buff,
                K::Minimap,
                K::DuraStatus,
                K::Socket,
                K::ChatNotice,
                K::ItemRental,
            ] {
                mgr.open(k);
            }
        }
        app.update();
        let mgr = app.world().resource::<DialogManager>();
        for k in [
            K::Inventory,
            K::Character,
            K::Mail,
            K::Npc,
            K::NpcGoods,
            K::Market,
        ] {
            assert!(!mgr.is_open(k), "{k:?} 应在 C# Closeall 集合内被关闭");
        }
        for k in [
            K::Trade,
            K::GuestTrade,
            K::Timer,
            K::Buff,
            K::Minimap,
            K::DuraStatus,
            K::Socket,
            K::ChatNotice,
            K::ItemRental,
        ] {
            assert!(
                mgr.is_open(k),
                "{k:?} 不在 C# Closeall 集合内，ESC 不应关它"
            );
        }
    }

    /// #2836 单元③：NPC 级联只在 `Npc` 窗开着时发生（C# `if (NPCDialog.Visible) NPCDialog.Hide();`）
    #[test]
    fn esc_npc_cascade_requires_npc_open() {
        use crate::game::dialogs::DialogKind as K;

        let mut app = esc_app(false, None);
        {
            let mut mgr = app.world_mut().resource_mut::<DialogManager>();
            mgr.open.clear();
            mgr.open(K::NpcGoods);
            mgr.open(K::Inspect); // 直接表内
        }
        app.update();
        let mgr = app.world().resource::<DialogManager>();
        assert!(
            mgr.is_open(K::NpcGoods),
            "NPC 窗没开时不应发生级联（商店留着）"
        );
        assert!(!mgr.is_open(K::Inspect), "直接表内的 Inspect 仍应关闭");
    }

    /// #2604：数量输入框（模态）开着 → Esc 让路（amount_box_system 自己消费
    /// Esc=Cancel），对话框不动。回归：去掉 amount.visible 让路则对话框被清空
    #[test]
    fn esc_yields_to_amount_box() {
        let mut app = esc_app_ext(false, None, true, false);
        app.update();
        assert_eq!(
            app.world().resource::<DialogManager>().open.len(),
            1,
            "数量框开着时 Esc 不应关其后面的对话框"
        );
        assert!(
            app.world()
                .resource::<crate::game::dialogs::amount_box::AmountBoxState>()
                .visible,
            "esc_close 不动数量框（由 amount_box_system 同帧消费 Esc=Cancel）"
        );
    }

    /// #2604：玩家右键菜单开着 → 让路（player_menu_ui_system 自己关）
    #[test]
    fn esc_yields_to_player_menu() {
        let mut app = esc_app_ext(false, None, false, true);
        app.update();
        assert_eq!(
            app.world().resource::<DialogManager>().open.len(),
            1,
            "玩家菜单开着时 Esc 不应连坐关对话框"
        );
        assert!(
            app.world()
                .resource::<crate::game::player_menu::PlayerMenuState>()
                .visible,
            "esc_close 不动玩家菜单（由 player_menu_ui_system 自己关）"
        );
    }
}
