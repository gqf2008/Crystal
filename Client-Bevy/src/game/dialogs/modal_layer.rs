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

/// 模态面板标记 —— **模态面板的唯一识别方式**（比「z >= [`MODAL_BLOCKER_Z`]」这个代理更硬）。
///
/// 由 [`spawn_modal_panel`] 挂；静态审计门禁 `tools/acceptance/csharp_golden/modal_panel_audit.py`
/// 的「面 A」用它核对「凡模态层的根都经唯一入口生成」。
#[derive(Component)]
pub struct ModalPanel;

/// 生成一块**模态面板** —— 模态层的**唯一入口**。
///
/// 与直接调 `ui::theme::spawn_panel(..., z)` 的区别：**这里不接受 z 参数**，固定用
/// [`MODAL_PANEL_Z`] 并挂 [`ModalPanel`] ⇒ 调用方**没有机会**把 z 传错。
///
/// 起因（2026-09-30，walgit 线程 `crystal-modal-layer-guards`）：`assign_key.rs` 曾给通用
/// `spawn_panel` 传字面量 `60` —— 数值恰好等于 `MODAL_PANEL_Z` 而长期「看起来对」。一旦有人
/// 为插新层调高 `MODAL_PANEL_Z`，该面板会落到遮挡层 `59` **之下**、被自家遮挡层盖住，而它是
/// 模态的、**没有关闭钮**、也不在实机巡回名单里 ⇒ 可能直接卡住交互。
/// 把 z 从「调用方传参」改成「入口内部决定」，这类漏改在**构造上**不再可能。
pub fn spawn_modal_panel(
    commands: &mut Commands,
    image: Handle<Image>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) -> Entity {
    let panel = crate::ui::theme::spawn_panel(commands, image, x, y, w, h, MODAL_PANEL_Z);
    commands.entity(panel).insert(ModalPanel);
    panel
}

/// **模态来源表** —— 「哪些本端状态算 C# 的 `Modal = true`」的唯一可审计清单。
///
/// 元素是 `(本端状态名, C# 依据)`；状态名必须与 [`ModalSources`] 的字段、以及
/// [`modal_any_visible`] 的入参**一一对应** —— 由静态审计门禁
/// `tools/acceptance/csharp_golden/modal_panel_audit.py` 的「面 B」核对：
/// 新增/删除模态来源而漏改此表会红（这正是 2026-09-30 那轮「`InvClickState.selected`
/// 被错当模态源」与「4 个真 Modal 框漏掉」两类问题各自的机械防线）。
pub const MODAL_SOURCES: &[(&str, &str)] = &[
    ("amount", "`MirAmountBox.cs:21,100`（数量框）"),
    ("confirm", "`MirMessageBox`（丢弃 / 扩容确认）"),
    ("assign_key", "`MirInputBox.cs:14`（快捷键分配框）"),
    (
        "notice",
        "`MirMessageBox.cs:19`（通用提示框 `notice_box.rs`）",
    ),
    ("group", "`MirMessageBox`（组队邀请）"),
    ("guild", "`MirMessageBox`（行会邀请）"),
    ("shop", "`MirMessageBox`（商城购买确认）"),
    ("hero", "`MirMessageBox`（`MakeActiveHero` 询问）"),
];

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

    /// 模态层**唯一入口**：产出的根必须恒在模态面板 z，且带 `ModalPanel` 标记。
    ///
    /// 红检：把 `spawn_modal_panel` 里改成 `z = MODAL_BLOCKER_Z` → 本用例 FAILED。
    #[test]
    fn spawn_modal_panel_pins_z_and_marks() {
        use bevy::ecs::world::CommandQueue;
        let world = World::new();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        let e = spawn_modal_panel(
            &mut commands,
            Handle::<Image>::default(),
            1.0,
            2.0,
            3.0,
            4.0,
        );
        let mut world = world;
        queue.apply(&mut world);
        assert_eq!(
            world.entity(e).get::<GlobalZIndex>().map(|z| z.0),
            Some(MODAL_PANEL_Z),
            "唯一入口必须把 z 钉在 MODAL_PANEL_Z（不接受调用方传参）"
        );
        assert!(
            world.entity(e).get::<ModalPanel>().is_some(),
            "唯一入口必须挂 ModalPanel 标记"
        );
        assert!(
            MODAL_PANEL_Z > MODAL_BLOCKER_Z,
            "模态面板必须高于全屏遮挡层"
        );
    }

    /// 收集 **crate 全部** `.rs`（`src/` + `tests/` + `examples/` + `benches/`）。
    ///
    /// 首版只扫 `src/`，被复核指出 `tests/`、`examples/` 不在内 —— 那是漏报面。
    ///
    /// **为什么扫源码而不是另写一个 `tools/acceptance/*_audit.py`**：本仓那批 `*_audit.py`
    /// **没有被任何脚本或 CI 调用**（`grep audit.py scripts/ .github/` 为空）⇒ 写了也不会拦人，
    /// 正是「工具/文档声称能拦、实际没人跑 = 假门禁」。这里随 `cargo test --lib` 跑。
    fn rs_sources() -> Vec<(String, String)> {
        fn walk(dir: &std::path::Path, out: &mut Vec<(String, String)>) {
            let Ok(rd) = std::fs::read_dir(dir) else {
                return;
            };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, out);
                } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                    if let Ok(s) = std::fs::read_to_string(&p) {
                        out.push((p.display().to_string(), s));
                    }
                }
            }
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut v = Vec::new();
        for sub in ["src", "tests", "examples", "benches"] {
            walk(&root.join(sub), &mut v);
        }
        assert!(
            v.len() > 20,
            "源码扫描必须真扫到文件（当前 {}），否则这道门禁会静默消失",
            v.len()
        );
        v
    }

    /// 把**注释**与**字符串/字符字面量的内容**替换成空格，**保持字节偏移不变**（便于报行号）。
    ///
    /// 为什么必须剥（复核实测，首版两条都中）：
    /// - 不剥注释 ⇒ 本文档注释里写的反例 `spawn_panel(..., 60)` 被当成真实调用（首跑就这么红的）；
    /// - 不剥字符串 ⇒ 字符串里的 `spawn_panel(..., 61)` 误报；且 `let q = '"';` 会把 `in_str` 带偏，
    ///   其后**整段注释不再被剥离**（复核实测第 13 号反例）。
    /// 字符字面量只认 `'X'` / `'\\X'` 形态，避免把生命周期 `'w` 当字面量。
    fn strip_comments_and_strs(src: &str) -> String {
        let c: Vec<char> = src.chars().collect();
        let mut out = String::with_capacity(src.len());
        let (mut i, mut in_str, mut esc, mut line_c, mut blk_c) =
            (0usize, false, false, false, false);
        while i < c.len() {
            let ch = c[i];
            let nx = c.get(i + 1).copied();
            if line_c {
                if ch == '\n' {
                    line_c = false;
                    out.push('\n');
                } else {
                    out.push(' ');
                }
            } else if blk_c {
                if ch == '*' && nx == Some('/') {
                    blk_c = false;
                    out.push_str("  ");
                    i += 2;
                    continue;
                }
                out.push(if ch == '\n' { '\n' } else { ' ' });
            } else if in_str {
                if esc {
                    esc = false;
                } else if ch == '\\' {
                    esc = true;
                } else if ch == '"' {
                    in_str = false;
                    out.push('"');
                    i += 1;
                    continue;
                }
                out.push(if ch == '\n' { '\n' } else { ' ' });
            } else if ch == '/' && nx == Some('/') {
                line_c = true;
                out.push_str("  ");
                i += 2;
                continue;
            } else if ch == '/' && nx == Some('*') {
                blk_c = true;
                out.push_str("  ");
                i += 2;
                continue;
            } else if ch == '"' {
                in_str = true;
                out.push('"');
            } else if ch == '\'' {
                let close = match nx {
                    Some('\\') => c.get(i + 3).copied() == Some('\''),
                    Some(_) => c.get(i + 2).copied() == Some('\''),
                    None => false,
                };
                if close {
                    let n = if nx == Some('\\') { 4 } else { 3 };
                    out.push('\'');
                    for _ in 1..n - 1 {
                        out.push(' ');
                    }
                    out.push('\'');
                    i += n;
                    continue;
                }
                out.push('\'');
            } else {
                out.push(ch);
            }
            i += 1;
        }
        out
    }

    /// `name(...)` 的顶层实参列表，**连同匹配起始位置**（用于豁免函数定义与唯一入口自身）。
    fn top_level_args(src: &str, name: &str) -> Vec<(usize, Vec<String>)> {
        let b = src.as_bytes();
        let mut out = Vec::new();
        let mut i = 0usize;
        while let Some(pos) = src[i..].find(name) {
            let start = i + pos + name.len();
            if b.get(start) != Some(&b'(') {
                i = start;
                continue;
            }
            let (mut depth, mut j) = (1i32, start + 1);
            while j < b.len() && depth > 0 {
                if b[j] == b'(' {
                    depth += 1;
                } else if b[j] == b')' {
                    depth -= 1;
                }
                j += 1;
            }
            let inner = &src[start + 1..j - 1];
            let mut args = Vec::new();
            let (mut d, mut cur) = (0i32, String::new());
            for ch in inner.chars() {
                match ch {
                    '(' | '[' | '{' => {
                        d += 1;
                        cur.push(ch);
                    }
                    ')' | ']' | '}' => {
                        d -= 1;
                        cur.push(ch);
                    }
                    ',' if d == 0 => {
                        let t = cur.trim();
                        if !t.is_empty() {
                            args.push(t.to_string());
                        }
                        cur.clear();
                    }
                    _ => cur.push(ch),
                }
            }
            let t = cur.trim();
            if !t.is_empty() {
                args.push(t.to_string());
            }
            out.push((start, args));
            i = j;
        }
        out
    }

    /// 取 `sig` 所指函数的**字节区间**（从签名到配对的收尾大括号）。
    ///
    /// 用途：豁免**唯一入口自己体内**那次 `spawn_panel(..., MODAL_PANEL_Z)` ——
    /// 否则面 A 会把「合法的那一次」也报红（首跑就是如此）。
    fn fn_span(src: &str, sig: &str) -> Option<(usize, usize)> {
        let start = src.find(sig)?;
        let b = src.as_bytes();
        let mut i = start + src[start..].find('{')?;
        let mut depth = 0i32;
        while i < b.len() {
            if b[i] == b'{' {
                depth += 1;
            } else if b[i] == b'}' {
                depth -= 1;
                if depth == 0 {
                    return Some((start, i));
                }
            }
            i += 1;
        }
        None
    }

    /// 允许**不**走唯一入口的非模态 z 具名常量（白名单；新增必须在此显式表态）。
    const NON_MODAL_Z_ALLOWED: &[&str] = &["STORAGE_PANEL_Z", "DIALOG_Z_MIN", "DIALOG_Z_MAX"];

    /// **门禁（面 A，fail-closed）**：`spawn_panel(` 的 z 实参**只允许**「字面量 < [`MODAL_BLOCKER_Z`]」
    /// 或 [`NON_MODAL_Z_ALLOWED`] 里的具名常量；**其余一律报红**。
    ///
    /// 为什么是 fail-closed（复核实测首版 6 类静默漏报）：首版用 `parse::<i32>()` 判「是不是 ≥59 的字面量」，
    /// 解析不了就 `continue` ⇒ `30 + 36` / 具名常量 / `71i32` / `MODAL_PANEL_Z` **全漏**。
    /// 其中 `spawn_panel(..., MODAL_PANEL_Z)` 最危险：z 对、但绕过唯一入口且不带 `ModalPanel` 标记。
    /// 改成「不认识的形态就报红」，漏报就变成「必须显式表态」。
    ///
    /// 红检：把任一 `spawn_modal_panel(` 改回 `spawn_panel(..., 60)` → 本用例 FAILED。
    #[test]
    fn no_hand_rolled_modal_layer_root() {
        let mut bad = Vec::new();
        for (path, raw) in rs_sources() {
            let src = strip_comments_and_strs(&raw);
            // 豁免一：函数**定义**（`fn spawn_panel(`）不是调用
            // 豁免二：唯一入口自己体内那次调用（合法）
            let entry = fn_span(&src, "fn spawn_modal_panel(");
            for (pos, args) in top_level_args(&src, "spawn_panel") {
                let before = src[..pos].trim_end();
                let before = before
                    .strip_suffix("spawn_panel")
                    .unwrap_or(before)
                    .trim_end();
                if before.ends_with("fn") {
                    continue;
                }
                if let Some((a, b)) = entry {
                    if pos > a && pos < b {
                        continue;
                    }
                }
                let Some(last) = args.last() else { continue };
                let z = last.trim();
                let ok = match z.parse::<i32>() {
                    Ok(n) => n < MODAL_BLOCKER_Z,
                    Err(_) => NON_MODAL_Z_ALLOWED.contains(&z),
                };
                if !ok {
                    bad.push(format!("{}: spawn_panel(..., {z})", path));
                }
            }
        }
        assert!(
            bad.is_empty(),
            "spawn_panel 的 z 只允许字面量 < {} 或白名单常量；模态层的根必须走 spawn_modal_panel。疑似手搓：\n  {}",
            MODAL_BLOCKER_Z,
            bad.join("\n  ")
        );
    }

    /// **门禁（面 A′）**：不得出现 `GlobalZIndex(<字面 z ≥ MODAL_BLOCKER_Z>)` —— 那是不经任何入口
    /// 直接造模态层根的写法（复核实测：首版完全不看它）。
    ///
    /// 红检：在任一文件里写 `GlobalZIndex(60)` → 本用例 FAILED。
    #[test]
    fn no_raw_modal_z_literal() {
        let mut bad = Vec::new();
        for (path, raw) in rs_sources() {
            let src = strip_comments_and_strs(&raw);
            for (_, args) in top_level_args(&src, "GlobalZIndex") {
                let Some(a) = args.first() else { continue };
                if let Ok(n) = a.trim().parse::<i32>() {
                    if n >= MODAL_BLOCKER_Z {
                        bad.push(format!("{}: GlobalZIndex({n})", path));
                    }
                }
            }
        }
        assert!(
            bad.is_empty(),
            "模态层的 z 不得写成裸字面量；请用 spawn_modal_panel（面 A′）：\n  {}",
            bad.join("\n  ")
        );
    }

    /// **门禁（面 B）**：`ModalSources` 的字段集合必须与 [`MODAL_SOURCES`] 表**完全一致**。
    ///
    /// 首版两处缺陷（复核实测）已修：**块注释里以 `pub ` 开头的行会误报**（改为先剥注释）、
    /// **私有（无 `pub`）字段会静默漏报**（改为 `pub` 可选）。
    ///
    /// 边界（**本门禁不管**）：它只保证「结构体 == 表」，**表的完备性与正确性没有机械背书** ——
    /// 复核实测的 `input_box` 漏登记、`assign_key` 依据写错、`confirm` 面板曾低于
    /// 遮挡层，三件事它一件都抓不到。
    ///
    /// 红检：往 `ModalSources` 加一个字段 → 本用例 FAILED。
    #[test]
    fn modal_sources_table_matches_the_param_struct_in_source() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/game/dialogs/modal_layer.rs");
        let raw = std::fs::read_to_string(&path).expect("必须读得到 modal_layer.rs 本体");
        let src = strip_comments_and_strs(&raw);
        let body = src
            .split("pub struct ModalSources<'w> {")
            .nth(1)
            .and_then(|s| s.split('}').next())
            .expect("必须在源码里找得到 ModalSources 结构体");
        let mut fields: Vec<String> = Vec::new();
        for line in body.lines() {
            let t = line.trim();
            let t = t.strip_prefix("pub ").unwrap_or(t);
            let Some((name, _)) = t.split_once(':') else {
                continue;
            };
            let name = name.trim();
            if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                fields.push(name.to_string());
            }
        }
        let table: Vec<String> = MODAL_SOURCES.iter().map(|(n, _)| n.to_string()).collect();
        assert!(
            !fields.is_empty(),
            "源码解析必须真解出字段，否则这道门禁会静默消失"
        );
        assert_eq!(
            fields, table,
            "ModalSources 字段与 MODAL_SOURCES 表必须一一对应"
        );
    }

    /// **模态来源表**必须与 `ModalSources` 的字段一一对应（名字 + 顺序 + 非空依据）。
    ///
    /// 红检：往 `ModalSources` 加一个字段而不同步 `MODAL_SOURCES` → 本用例 FAILED
    /// （顺序/集合断言会先红）；从 `modal_any_visible` 增删入参同理。
    #[test]
    fn modal_sources_table_covers_the_param_struct() {
        let names: Vec<&str> = MODAL_SOURCES.iter().map(|(n, _)| *n).collect();
        assert_eq!(
            names,
            vec![
                "amount",
                "confirm",
                "assign_key",
                "notice",
                "group",
                "guild",
                "shop",
                "hero",
            ],
            "MODAL_SOURCES 的名字/顺序必须与 ModalSources 字段、modal_any_visible 入参一致"
        );
        assert!(
            MODAL_SOURCES.iter().all(|(_, src)| !src.is_empty()),
            "每一项都必须写明 C# 依据，空依据等于没登记"
        );
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
