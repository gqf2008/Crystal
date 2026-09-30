// ============================================================================
// 统一模态层：C# `MirControl.Modal = true` 的**UI 层遮挡**那一半
// ============================================================================
// 原版依据（`Client/MirControls/MirControl.cs`）：
//   ```csharp
//   public virtual bool IsMouseOver(Point p)          // :825-828
//   { return Visible && (DisplayRectangle.Contains(p) || Moving || Modal) && !NotControl; }
//   ```
//   `Modal` 为真 ⇒ **任意点**都算命中；子控件派发自顶向下取第一个命中并 `return`
//   （`:921-927`）⇒ 可见的 Modal 控件吞掉**整个客户区**的鼠标输入，不只是自己矩形内的。
//   `MirMessageBox.cs:19` 构造即 `Modal = true`。
//
// 本端此前只有一半：`player_control::UiLockState` 让**世界点击**让路（#2588/#3314），
// 但弹框期间点**其它对话框**仍然穿透（walgit 线程 `crystal-modal-layer-batch` ①）。
// 这里用 bevy_ui 的一枚**全客户区遮挡节点**把它补齐：
//
//   * z = [`MODAL_BLOCKER_Z`]（59）：高于全部非模态窗（观察到的最大 51），
//     低于全部模态面板（[`MODAL_PANEL_Z`] = 60）⇒ 框**自己**的按钮照常可点；
//   * 「有没有模态」只有**一处真值**：[`ModalSources`] / [`modal_any_visible`]
//     （此前 7 处各自挑 z：60/60/60/45/45/46/47，正是本仓反复吃过的「两处维护同一份真值」）。
//     注意世界点击闸 = 该真值 **∪**「物品已选中」（`player_control::world_click_locked`）——
//     两者刻意不是一个集合，见 [`modal_any_visible`] 的说明。
//
// ⚠️ 这条遮挡**离线证不了**（bevy_ui picking 的命中栈要真光标）：`dialogs/interact_gate.rs`
// 模块头自己写明不守遮挡。判据在 `tools/acceptance/ui_interact_sweep.ps1` 的
// 「模态遮挡」段（弹提示框 → 点下面那扇窗的关闭钮 → 断言没关），见 §3.2bw。
// ============================================================================

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::game::dialogs::amount_box::AmountBoxState;
use crate::game::dialogs::assign_key::AssignKeyState;
use crate::game::dialogs::game_shop::GameShopState;
use crate::game::dialogs::group::GroupState;
use crate::game::dialogs::guild::GuildState;
use crate::game::dialogs::hero::HeroState;
use crate::game::dialogs::inventory::InvDropConfirm;
use crate::game::dialogs::notice_box::NoticeBox;
use crate::scenes::AppState;

/// 客户区尺寸（C# `Settings.ScreenWidth/ScreenHeight`；本端固定 1024x768）
pub const CLIENT_W: f32 = 1024.0;
pub const CLIENT_H: f32 = 768.0;
/// 遮挡层 z：高于全部**非模态**窗（本仓观察到的最大 51），低于全部模态面板。
pub const MODAL_BLOCKER_Z: i32 = 59;
/// 模态面板统一 z（C# 里这些面板都是 `MirMessageBox` 系 = `Modal = true`，谁也不能被遮挡层盖住）
pub const MODAL_PANEL_Z: i32 = 60;

/// 遮挡节点标记（全客户区、`Button` + `Interaction` ⇒ bevy_ui picking 会命中它而不是下层按钮）
#[derive(Component)]
pub struct ModalBlocker;

/// 所有模态来源的状态（C# `Modal = true` 的那些控件对应的本端状态）。
#[derive(SystemParam)]
pub struct ModalSources<'w> {
    /// `MirAmountBox`（数量框）
    pub amount: Res<'w, AmountBoxState>,
    /// 丢弃确认（`MirMessageBox`）
    pub confirm: Res<'w, InvDropConfirm>,
    /// 快捷键分配框（`MirInputBox`）
    pub assign_key: Res<'w, AssignKeyState>,
    /// 通用 `MirMessageBox`（`notice_box.rs`）
    pub notice: Res<'w, NoticeBox>,
    /// 组队邀请提示（C# `GroupDialog` 的 `MirMessageBox`）
    pub group: Res<'w, GroupState>,
    /// 行会邀请提示
    pub guild: Res<'w, GuildState>,
    /// 商城购买确认（`MirMessageBox`，`game_shop.rs:1633`）
    pub shop: Res<'w, GameShopState>,
    /// 英雄 `MakeActiveHero` 询问（`MirMessageBox`，`hero.rs:308`）
    pub hero: Res<'w, HeroState>,
}

impl ModalSources<'_> {
    /// 任一模态控件可见 ⇒ 必须遮挡。
    pub fn any_visible(&self) -> bool {
        modal_any_visible(
            self.amount.visible,
            self.confirm.visible,
            self.assign_key.visible,
            self.notice.is_visible(),
            self.group.invite.is_some(),
            self.guild.invite.is_some(),
            self.shop.pending.is_some(),
            self.hero.managing && self.hero.confirm_slot.is_some(),
        )
    }
}

/// 「有没有模态」的**纯函数真值**（UI 遮挡层用，单测直接钉它）。
///
/// 世界点击闸**不等于**它：世界点击闸 = 本函数 ∪ 「物品已选中」
/// （`player_control::world_click_locked`）。两者刻意不是一个集合，理由见下。
///
/// **为什么这里不含「选中物品」**（2026-09-30 修 #3396 引入的回归）：
/// C# 全仓 `Modal = true` 只有 7 处 —— `MirMessageBox.cs:19`、`MirAmountBox.cs:21,100`、
/// `MirInputBox.cs:14`、`NewCharacterDialog.cs:51`、`MainDialogs.cs:2013,3802`
/// （其中 `2013` 因 `NotControl` 失效）—— **没有一处对应「选中物品」**。
/// 把它算进遮挡真值，就会让「背包里单击选中一件物品」凭空升起一层全屏遮挡节点
/// （z = [`MODAL_BLOCKER_Z`] 盖住 z ∈ [`DIALOG_Z_MIN`, `DIALOG_Z_MAX`] 的全部对话框）
/// ⇒ 连背包自己的 X / Add / Del 都点不动。
///
/// 而「选中物品时**世界点击**让路」是本仓既有且刻意的行为（原
/// `modal_ui_locked(selected, amount, confirm, assign_key)` 就带它，并有真值表单测），
/// 所以它留在 `world_click_locked` 里，不进这里。
#[allow(clippy::too_many_arguments)]
pub fn modal_any_visible(
    amount: bool,
    confirm: bool,
    assign_key: bool,
    notice: bool,
    group_invite: bool,
    guild_invite: bool,
    shop_confirm: bool,
    hero_confirm: bool,
) -> bool {
    amount
        || confirm
        || assign_key
        || notice
        || group_invite
        || guild_invite
        || shop_confirm
        || hero_confirm
}

pub struct ModalLayerPlugin;

impl Plugin for ModalLayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Game), spawn_modal_blocker);
        app.add_systems(OnExit(AppState::Game), cleanup_modal_blocker);
        app.add_systems(Update, modal_layer_system.run_if(in_state(AppState::Game)));
    }
}

fn spawn_modal_blocker(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Px(CLIENT_W),
            height: Val::Px(CLIENT_H),
            ..default()
        },
        // `Button` + `Interaction` ⇒ 参与 bevy_ui picking：光标落在它上面时命中它，
        // 下层的对话框按钮拿不到 hover/press。
        Button,
        // 显式写死两件（`Button` 的 `#[require]` 理论上会补 `FocusPolicy::Block`，
        // 但实测「弹框期间点下层窗照样关」⇒ 不押在 require 上）：
        //   * `FocusPolicy::Block`：`ui_focus_system` 自顶向下遇到它就 break（下层不进 Pressed）；
        //   * `Pickable`：bevy_picking 的 UI 后端在 `require_markers` 下只认挂了 `Pickable`
        //     的节点（没有它会被直接跳过 ⇒ HoverMap 里根本看不见遮挡层）。
        bevy::ui::FocusPolicy::Block,
        bevy::picking::Pickable::default(),
        Interaction::default(),
        BackgroundColor(Color::NONE),
        GlobalZIndex(MODAL_BLOCKER_Z),
        ModalBlocker,
        // 无模态时隐藏 ⇒ picking 直接跳过（bevy_ui 只拾取可见节点）
        Visibility::Hidden,
    ));
}

fn cleanup_modal_blocker(mut commands: Commands, blockers: Query<Entity, With<ModalBlocker>>) {
    for e in blockers.iter() {
        commands.entity(e).despawn();
    }
}

fn modal_layer_system(
    sources: ModalSources,
    mut blockers: Query<&mut Visibility, With<ModalBlocker>>,
) {
    let want = if sources.any_visible() {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut vis in blockers.iter_mut() {
        if *vis != want {
            *vis = want;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    /// z 的纪律：遮挡层必须**卡在**非模态窗与模态面板之间。改任一个常量都该红。
    #[test]
    fn blocker_z_sits_between_normal_windows_and_modal_panels() {
        assert_eq!(MODAL_BLOCKER_Z, 59);
        assert_eq!(MODAL_PANEL_Z, 60);
        assert!(
            MODAL_BLOCKER_Z > 51,
            "遮挡层要盖过全部非模态窗（本仓观察到的最大 z=51）"
        );
        assert!(
            MODAL_BLOCKER_Z < MODAL_PANEL_Z,
            "模态面板必须高于遮挡层，否则自己的按钮点不动"
        );
    }

    /// 真值函数：8 个来源任一为真即遮挡；全假时不遮挡。
    ///
    /// 8 = C# `Modal = true` 对应的本端状态（数量框/丢弃确认/快捷键/通用 MirMessageBox +
    /// 组队·行会·商城·英雄四处 MirMessageBox）。**不含「选中物品」**——那不是 C# 的 Modal，
    /// 算进来会让单击选中一件物品就升起全屏遮挡层（#3396 回归，见函数文档）。
    #[test]
    fn modal_any_visible_is_union_of_sources() {
        let none = [false; 8];
        assert!(!modal_any_visible(
            none[0], none[1], none[2], none[3], none[4], none[5], none[6], none[7]
        ));
        for i in 0..8 {
            let mut v = none;
            v[i] = true;
            assert!(
                modal_any_visible(v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]),
                "第 {i} 个模态来源为真时应当遮挡"
            );
        }
    }

    /// 遮挡节点：全客户区 + z=59 + 可拾取（`Button`/`Interaction`）+ 默认隐藏。
    #[test]
    fn blocker_node_is_full_client_area_and_pickable() {
        let mut world = World::new();
        world
            .run_system_once(spawn_modal_blocker)
            .expect("spawn 应成功");
        let mut q = world.query::<(
            &Node,
            &GlobalZIndex,
            &Visibility,
            Has<Button>,
            Has<Interaction>,
            Has<bevy::ui::FocusPolicy>,
            Has<bevy::picking::Pickable>,
        )>();
        let rows: Vec<_> = q.iter(&world).collect();
        assert_eq!(rows.len(), 1, "应当只有一枚遮挡节点");
        let (node, z, vis, has_button, has_inter, has_focus, has_pickable) = rows[0];
        assert_eq!(node.left, Val::Px(0.0));
        assert_eq!(node.top, Val::Px(0.0));
        assert_eq!(node.width, Val::Px(CLIENT_W));
        assert_eq!(node.height, Val::Px(CLIENT_H));
        assert_eq!(*z, GlobalZIndex(MODAL_BLOCKER_Z));
        assert!(
            has_button && has_inter,
            "必须能参与 picking（Button+Interaction）"
        );
        // 红检（本轮实测踩到）：缺 `FocusPolicy::Block` ⇒ `ui_focus_system` 会穿过遮挡层把
        // Pressed 发给下层按钮（弹框期间点背包 X 照样关窗）；缺 `Pickable` ⇒ UI picking 后端
        // 在 `require_markers` 下直接跳过它（HoverMap 里看不见遮挡层）。
        assert!(has_focus, "必须显式挂 FocusPolicy（Block）");
        assert!(
            has_pickable,
            "必须显式挂 Pickable（否则 picking 后端会跳过）"
        );
        assert_eq!(*vis, Visibility::Hidden, "无模态时默认隐藏");
    }
}
