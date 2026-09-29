// ============================================================================
// 统一信息提示框（C# `MirMessageBox` + `MirMessageBoxButtons.OK`）+ 各窗 `Show()` 前置守卫
// ============================================================================
// 为什么要有这个模块（2026-09-28，金标准逐窗像素 A/B 捞出来的）：
//   原版有 4 扇窗在**状态不具备**时不开窗，而是弹一个只带 OKAY 的 `MirMessageBox`
//   （守卫写在各自对话框的 `Show()` 覆写里 ⇒ 键盘路径与程序化开窗都必须走）：
//     · 宠物 `IntelligentCreatureDialogs.cs:832-841`：`!User.IntelligentCreatures.Any()` → NoCreatures
//     · 行会 `GuildDialog.cs:2156-2166`：`User.GuildName == ""`            → NotInGuild
//     · 坐骑 `MountDialog.cs:240-251`：`User.MountType < 0`                → NoMount
//     · 钓鱼 `FishingDialog.cs:135-147`：`!User.HasFishingRod`             → NoFishingRod
//   本端此前这 4 扇窗照开空窗（还带大片黑底）。这里把「提示框」与「守卫判定」收敛成**唯一入口**，
//   供键盘热键（`keyboard_layout::dialog_hotkey_system`）与 RPC/控制路径
//   （`control::apply_control_commands` 的 `Dialog` 分支）共用，避免再复制第三份。
//
// 布局与文案来源（都对原版）：
//   · 面板 `Prguse[360]` 456x190，居中 @(284,289)（`MirMessageBox.cs:23-26`）
//   · 文本 `(35,35)` 尺寸 `390x110`（`MirMessageBox.cs:29-37`）
//   · OKAY 按钮 `Title[200/201/202]` @(360,157)（`MirMessageBox.cs:42-52`，原生 69x25）
//   · 文案取原版中文包：`Client/Localization/Chinese.json` 的
//     NoCreatures/NotInGuild/NoMount/NoFishingRod
// ============================================================================

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiCjkFont;
use crate::ui::sprite_ui::{shared_cjk_font, UiFont};
use crate::ui::theme::{load_lib_image, spawn_icon_button, spawn_label, spawn_panel};

/// 面板精灵（C# `MirMessageBox.Index = 360; Library = Libraries.Prguse`）
pub const PANEL: (LibraryName, usize) = (LibraryName::Prguse, 360);
/// C# 图头尺寸 456x190
pub const PANEL_SIZE: (f32, f32) = (456.0, 190.0);
/// C# `Location = ((ScreenWidth - W)/2, (ScreenHeight - H)/2)`（1024x768 ⇒ (284,289)）
pub const PANEL_POS: (f32, f32) = ((1024.0 - PANEL_SIZE.0) / 2.0, (768.0 - PANEL_SIZE.1) / 2.0);
/// C# `Label`：`Location = (35,35)`、`Size = (390,110)`
pub const LABEL_POS: (f32, f32) = (35.0, 35.0);
pub const LABEL_SIZE: (f32, f32) = (390.0, 110.0);
pub const LABEL_FONT_SIZE: f32 = 12.0;
/// C# `OKButton`：`Title[200/201/202]` @(360,157)。
/// **原生尺寸 76x25**（`libextract.py Title.Lib 200` 实测；第一版按截图目测写成 69x25，
/// 结果按钮被横向压扁 8%、逐窗 A/B 的按钮区差异 81%——面板 456x190 同法实测无误）。
pub const OK_POS: (f32, f32) = (360.0, 157.0);
pub const OK_SIZE: (f32, f32) = (76.0, 25.0);

/// 要显示的信息提示（`Some` = 显示；同一时刻只有一个，C# `MirMessageBox` 是模态单例）。
#[derive(Resource, Default)]
pub struct NoticeBox {
    pub text: Option<String>,
    /// 面板是否**真的建出来了**（`spawn_notice_box` 建成功置 `true`，`cleanup_notice_box` 置回 `false`）。
    ///
    /// 为什么要它（独立复核 2026-09-29 提出的退化路径）：`spawn_notice_box` 在 `Prguse[360]`
    /// 取不到时会提前 `return`（本文件 `let Some(bg) = load_lib_image(..) else { return }`）——
    /// 那时**没有面板、没有 OK 钮**，而四扇窗的守卫照样能往 `text` 里写文案。
    /// 若可见性只看 `text`，就会把一个**纯显示层退化**升级成「世界输入被锁、屏上却没有任何可见 UI」。
    /// 故可见性 = **有文案 且 面板真的在**（Enter/ESC 仍能清文案，不是死锁，但不该锁）。
    pub panel_ready: bool,
}

impl NoticeBox {
    pub fn show(&mut self, text: impl Into<String>) {
        self.text = Some(text.into());
    }

    /// 提示框是否可见 —— 也就是 C# `MirMessageBox.Modal == true` 生效的那段期间。
    ///
    /// 原版依据（`Client/MirControls/MirControl.cs`）：
    /// ```csharp
    /// public virtual bool IsMouseOver(Point p)          // :825-828
    /// { return Visible && (DisplayRectangle.Contains(p) || Moving || Modal) && !NotControl; }
    /// ```
    /// `Modal` 为真时**任意点**都返回 `true`；子控件派发是自顶向下取第一个命中并 `return`
    /// （`:921-927` `for (int i = Controls.Count - 1; i >= 0; i--)`）——
    /// 所以可见的 Modal 控件会**吞掉整个客户区的鼠标输入**，不只是自己矩形内的。
    /// `MirMessageBox.cs:19` 构造即 `Modal = true`。
    ///
    /// 本端已接的（`player_control::UiLockState`）：世界点击闸。
    /// **未接**：其它 UI 对话框的点击（C# 里同样被吞）——那属于「按 picking 遮挡」，
    /// headless 门禁证不了（见 `dialogs/interact_gate.rs` 模块头「不守遮挡」），
    /// 需实机 `ui_interact_sweep.ps1`，见 walgit 线程 `crystal-modal-input-lock`。
    ///
    /// **注意它跟踪的是「模型 + 面板就绪」而不是面板的 `Visibility`**：当前两者同源一致
    /// （`notice_box_system` 的显隐就是由 `text.is_some()` 推导的），但**将来若把这个资源
    /// 复用作非模态 toast，就会静默锁住世界输入**（独立复核 2026-09-29 提示）。真要复用请
    /// 另开一个字段/资源区分「模态」与「仅显示」，别直接拿这个判据。
    pub fn is_visible(&self) -> bool {
        self.panel_ready && self.text.is_some()
    }
}

#[derive(Component)]
pub struct NoticeBoxPanel;

#[derive(Component)]
pub struct NoticeBoxText;

#[derive(Component)]
pub struct NoticeOkButton;

pub struct NoticeBoxPlugin;

impl Plugin for NoticeBoxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NoticeBox>();
        app.add_systems(OnEnter(AppState::Game), spawn_notice_box);
        app.add_systems(OnExit(AppState::Game), cleanup_notice_box);
        app.add_systems(Update, notice_box_system.run_if(in_state(AppState::Game)));
    }
}

fn cleanup_notice_box(
    mut commands: Commands,
    roots: Query<Entity, With<NoticeBoxPanel>>,
    mut notice: ResMut<NoticeBox>,
) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
    // 面板没了 ⇒ 可见性判据必须跟着假，否则退出/重进 Game 会留下「锁着但画不出」的状态
    notice.panel_ready = false;
    // 文案也必须一起清（独立复核 2026-09-29 的「旧提示复活」）：只清 `panel_ready` 时，
    // 退出 Game 那一刻开着的提示会留在 `text` 里，下次 `spawn_notice_box` 把面板建出来后
    // **上一局的旧提示会自己弹回来**（可见性判据自洽、锁也没错，纯粹是脏状态）。
    notice.text = None;
}

fn spawn_notice_box(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
    mut cjk_font: ResMut<UiCjkFont>,
    mut notice: ResMut<NoticeBox>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        crate::ui::sprite_ui::ensure_ui_font(&mut fonts, &mut ui_font);
    }
    let font = shared_cjk_font(&mut fonts, &mut cjk_font);
    let Some(bg) = load_lib_image(&mut libs, &mut images, PANEL.0, PANEL.1) else {
        return;
    };
    let panel = spawn_panel(
        &mut commands,
        bg,
        PANEL_POS.0,
        PANEL_POS.1,
        PANEL_SIZE.0,
        PANEL_SIZE.1,
        // 模态面板统一 z（`modal_layer::MODAL_PANEL_Z`）：必须高于遮挡层 59
        crate::game::dialogs::modal_layer::MODAL_PANEL_Z,
    );
    commands
        .entity(panel)
        .insert((NoticeBoxPanel, Visibility::Hidden));
    // 建到这里才算「画得出来」——上面那个 let-else 早退路径不会执行到这里，
    // 于是资产缺失时 panel_ready 保持 false，守卫写的文案不会变成一道看不见的输入锁。
    notice.panel_ready = true;
    commands.entity(panel).with_children(|p| {
        spawn_label(
            p,
            &font,
            "",
            LABEL_POS.0,
            LABEL_POS.1,
            LABEL_FONT_SIZE,
            Color::WHITE,
            9,
        )
        .insert(NoticeBoxText);
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 200),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 201),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 202),
        ) {
            spawn_icon_button(p, n, h, pr, OK_POS.0, OK_POS.1, OK_SIZE.0, OK_SIZE.1, 10)
                .insert(NoticeOkButton);
        }
    });
}

/// 显隐 + 文案同步；OKAY 点击 / 回车 / ESC 关闭（C# `MirMessageBox.HandleKeyPress`：回车 = 按 OK）。
fn notice_box_system(
    mut notice: ResMut<NoticeBox>,
    keys: Res<ButtonInput<KeyCode>>,
    mut panels: Query<&mut Visibility, (With<NoticeBoxPanel>, Without<NoticeBoxText>)>,
    mut texts: Query<&mut Text, With<NoticeBoxText>>,
    mut ok: Query<&Interaction, (With<NoticeOkButton>, Changed<Interaction>)>,
) {
    let mut close = false;
    for inter in &mut ok {
        if *inter == Interaction::Pressed {
            close = true;
        }
    }
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Escape) {
        close = true;
    }
    if close {
        notice.text = None;
    }
    let visible = notice.text.is_some();
    for mut vis in &mut panels {
        *vis = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if let Some(text) = &notice.text {
        for mut t in &mut texts {
            if t.0 != *text {
                t.0 = text.clone();
            }
        }
    }
}

/// 四扇窗 `Show()` 守卫需要的状态（都由本端已有状态推导）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShowGuardState {
    /// 是否有宠物：`CreatureState.creatures` 非空（C# `User.IntelligentCreatures.Any()`）
    pub has_creatures: bool,
    /// 是否在行会：`GuildState.in_guild`（C# `User.GuildName != ""`）
    pub in_guild: bool,
    /// 是否骑乘/拥有坐骑：本地玩家有 `MountState` 组件（C# `User.MountType >= 0`）
    pub has_mount: bool,
    /// 是否装备钓鱼竿：武器槽 shape ∈ `Globals.n`（C# `HasFishingRod = Globals.FishingRodShapes.Contains(Weapon)`，
    /// `Shared/Globals.cs`：`n = {49, 50}`）
    pub has_fishing_rod: bool,
    /// 当前地图有没有大地图：`GameData.big_map_index > 0`（C# `MapControl.BigMap > 0`）
    pub has_big_map: bool,
}

/// 钓鱼竿判定用的武器 shape 白名单（C# `Shared/Globals.cs`：`n = new int[] { 49, 50 }`）
pub const FISHING_ROD_SHAPES: [i16; 2] = [49, 50];

/// C# 各对话框 `Show()` 的前置守卫结论。
///
/// 两种拦法在 C# 里**不一样**，别混：宠物/行会/坐骑/钓鱼是「弹 `MirMessageBox(文案)` 再
/// `return`」；大地图是 `if (map.BigMap <= 0) return;`（`BigMapDialog.cs:288-289`）——
/// **什么都不弹**，窗口直接不开。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShowGuard {
    /// 放行（正常开窗）
    Allow,
    /// 拦下并弹提示框（`MirMessageBox`）
    Block(&'static str),
    /// 拦下但**不弹提示框**（C# `Show()` 干返回）
    BlockSilent,
}

/// C# 各对话框 `Show()` 的前置守卫。
pub fn show_guard(kind: DialogKind, st: &ShowGuardState) -> ShowGuard {
    match kind {
        DialogKind::Creature if !st.has_creatures => ShowGuard::Block("你没有任何宠物。"),
        DialogKind::Guild if !st.in_guild => ShowGuard::Block("你不在任何公会中。"),
        DialogKind::Mount if !st.has_mount => ShowGuard::Block("你没有坐骑。"),
        DialogKind::Fishing if !st.has_fishing_rod => ShowGuard::Block("你没有拿着鱼竿。"),
        // `BigMapDialog.Show()`：`if (map.BigMap <= 0) return;`——**静默**不开窗，没有提示框
        DialogKind::BigMap if !st.has_big_map => ShowGuard::BlockSilent,
        _ => ShowGuard::Allow,
    }
}

/// 守卫所需的查询/资源打包（键盘热键系统与 `ControlQueries` 共用同一份实现）。
#[derive(SystemParam)]
pub struct ShowGuardParams<'w, 's> {
    guild: Res<'w, crate::game::dialogs::guild::GuildState>,
    creatures: Res<'w, crate::game::dialogs::creature::CreatureState>,
    mounts:
        Query<'w, 's, Option<&'static crate::actor::MountState>, With<crate::actor::LocalPlayer>>,
    loadout:
        Query<'w, 's, &'static crate::game::player_state::Loadout, With<crate::actor::LocalPlayer>>,
    /// 大地图索引来源（C# `MapControl.BigMap`）
    game_data: Res<'w, crate::map_renderer::GameData>,
    pub notice: ResMut<'w, NoticeBox>,
}

impl ShowGuardParams<'_, '_> {
    pub fn state(&self) -> ShowGuardState {
        let has_mount = self.mounts.iter().any(|m| m.is_some());
        let has_fishing_rod = self
            .loadout
            .iter()
            .next()
            .and_then(|l| l.slots.first())
            .and_then(|s| s.as_ref())
            .map(|w| FISHING_ROD_SHAPES.contains(&w.shape))
            .unwrap_or(false);
        ShowGuardState {
            has_creatures: !self.creatures.creatures.is_empty(),
            in_guild: self.guild.in_guild,
            has_mount,
            has_fishing_rod,
            has_big_map: self.game_data.big_map_index > 0,
        }
    }

    /// `true` = 被守卫拦下：调用方**不要再 open 那扇窗**。
    /// 有文案的（宠物/行会/坐骑/钓鱼）顺手写进提示框；大地图这类 C# 干返回的**不弹框**。
    pub fn blocked(&mut self, kind: DialogKind) -> bool {
        match show_guard(kind, &self.state()) {
            ShowGuard::Block(text) => {
                self.notice.show(text);
                tracing::info!("🛡️ {kind:?} 前置不成立 → 只弹提示框：{text}");
                true
            }
            ShowGuard::BlockSilent => {
                tracing::info!("🛡️ {kind:?} 前置不成立 → 不开窗（C# Show() 静默返回，不弹提示）");
                true
            }
            ShowGuard::Allow => false,
        }
    }
}

/// 打开/切换对话框，并施加 C# 的 `Show()` 守卫（被拦下时返回 `true`）。
pub fn guarded_toggle(
    kind: DialogKind,
    mgr: &mut DialogManager,
    guard: &mut ShowGuardParams,
) -> bool {
    if guard.blocked(kind) {
        return true;
    }
    mgr.toggle(kind);
    false
}

/// 打开对话框（Open 语义），并施加 C# 的 `Show()` 守卫。
pub fn guarded_open(
    kind: DialogKind,
    mgr: &mut DialogManager,
    guard: &mut ShowGuardParams,
) -> bool {
    if guard.blocked(kind) {
        return true;
    }
    mgr.open(kind);
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full() -> ShowGuardState {
        ShowGuardState {
            has_creatures: true,
            in_guild: true,
            has_mount: true,
            has_fishing_rod: true,
            has_big_map: true,
        }
    }

    /// 守卫的判据：状态缺失 ⇒ 有文案；状态具备 ⇒ 放行。
    /// **阳性对照**：把 `show_guard` 里任一分支去掉，本用例立刻红。
    #[test]
    fn show_guard_blocks_only_when_state_missing() {
        let ok = full();
        for k in [
            DialogKind::Creature,
            DialogKind::Guild,
            DialogKind::Mount,
            DialogKind::Fishing,
            DialogKind::BigMap,
            DialogKind::Inventory,
        ] {
            assert_eq!(
                show_guard(k, &ok),
                ShowGuard::Allow,
                "{k:?} 状态具备时不该拦"
            );
        }

        assert_eq!(
            show_guard(
                DialogKind::Creature,
                &ShowGuardState {
                    has_creatures: false,
                    ..ok
                }
            ),
            ShowGuard::Block("你没有任何宠物。")
        );
        assert_eq!(
            show_guard(
                DialogKind::Guild,
                &ShowGuardState {
                    in_guild: false,
                    ..ok
                }
            ),
            ShowGuard::Block("你不在任何公会中。")
        );
        assert_eq!(
            show_guard(
                DialogKind::Mount,
                &ShowGuardState {
                    has_mount: false,
                    ..ok
                }
            ),
            ShowGuard::Block("你没有坐骑。")
        );
        assert_eq!(
            show_guard(
                DialogKind::Fishing,
                &ShowGuardState {
                    has_fishing_rod: false,
                    ..ok
                }
            ),
            ShowGuard::Block("你没有拿着鱼竿。")
        );
        // 大地图：C# `BigMapDialog.Show()` 的 `if (map.BigMap <= 0) return;` ——
        // **静默**拦下（不弹 MirMessageBox），与本组前四扇的 `Block(文案)` 不同型。
        assert_eq!(
            show_guard(
                DialogKind::BigMap,
                &ShowGuardState {
                    has_big_map: false,
                    ..ok
                }
            ),
            ShowGuard::BlockSilent
        );
        // 无守卫的窗不受影响
        assert_eq!(
            show_guard(DialogKind::Inventory, &ShowGuardState::default()),
            ShowGuard::Allow
        );
    }

    /// 文案与原版中文包逐字一致（`Client/Localization/Chinese.json`）。
    #[test]
    fn guard_texts_match_csharp_chinese_localization() {
        let st = ShowGuardState {
            // 这几扇只看自己的状态位，大地图位留 true 以免误触 BigMap 分支
            has_big_map: true,
            ..ShowGuardState::default()
        };
        assert_eq!(
            show_guard(DialogKind::Creature, &st),
            ShowGuard::Block("你没有任何宠物。")
        );
        assert_eq!(
            show_guard(DialogKind::Guild, &st),
            ShowGuard::Block("你不在任何公会中。")
        );
        assert_eq!(
            show_guard(DialogKind::Mount, &st),
            ShowGuard::Block("你没有坐骑。")
        );
        assert_eq!(
            show_guard(DialogKind::Fishing, &st),
            ShowGuard::Block("你没有拿着鱼竿。")
        );
    }

    /// ② 「旧提示复活」：退出 `AppState::Game` 时若提示框还开着，`cleanup_notice_box`
    /// 必须把 **文案** 也清掉（只清 `panel_ready` 会让旧提示在下次进图时自己弹回来）。
    ///
    /// 红检：删掉 `cleanup_notice_box` 里的 `notice.text = None;` → 本用例 FAILED。
    #[test]
    fn cleanup_clears_text_so_stale_notice_cannot_revive() {
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        world.insert_resource(NoticeBox {
            text: Some("上一局的旧提示".to_string()),
            panel_ready: true,
        });
        assert!(
            world.resource::<NoticeBox>().is_visible(),
            "前置：清场前提示框是可见的"
        );
        world
            .run_system_once(cleanup_notice_box)
            .expect("cleanup_notice_box 应成功");
        let n = world.resource::<NoticeBox>();
        assert!(!n.panel_ready, "panel_ready 必须置假");
        assert!(n.text.is_none(), "text 必须清空（否则旧提示会复活）");
        assert!(!n.is_visible(), "清场后不得可见");
    }
}
