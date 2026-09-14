// ============================================================================
// 掷骰子对话框（M57）
// 参考：C# RollDialog（Client/MirScenes/Dialogs/RollDialog.cs）
//   - 服务端 Roll 包（270）触发：Type(0=骰子/1=尤茨) Page Result AutoRoll
//   - 三段相位（C# `_animation` / `_image` 两个控件）：
//       Idle   ：骰子 Prguse[282] / 尤茨 Items[2581]，点图开掷（`:117` `_image_Click` → `Roll()`）
//       Rolling：骰子 Prguse[290..293] 4 帧 100ms、`Loop=true` 计 6 轮 = 2.4s（`:130-146` `_currentLoop < 5`）
//                尤茨 Items[2581..2586] 6 帧 100ms、`Loop=false` = 0.6s（`:147-162`）
//       Result ：骰子 Prguse[281+result] / 尤茨 Items[2587+result]，点图关闭（`_rolled` 分支）
//   - 布局：骰子 Size 65x65 @ (SW/2-38, SH/2-40)=(474,344)；尤茨 Size 180x130 @ (422,319)
//     两个控件的 `UseOffSet = true` → 实际绘制点 = Location + `.Lib` 帧自带 offset
//     （骰子帧 offset (0,0)、尤茨结果帧 (1,0)），**绘制尺寸用帧原生尺寸而非控件 Size**
//   - autoRoll=true 直接进 Rolling（C# `Setup` 末尾 `Roll()`）
//   - 动画结束即回调：CallNPC "[page]"（C# ReturnResult）
// 网络：服务端 NPC 脚本 ROLLDIE/ROLLYUT 动作发送 Roll 包
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::npc::NpcDialogState;
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot, NotDraggable};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiFont;
use crate::ui::theme::{load_lib_image, spawn_container, spawn_label, spawn_panel};

/// 屏幕尺寸（C# `Settings.ScreenWidth/ScreenHeight`，本端固定 1024x768）
pub const SCREEN_W: f32 = 1024.0;
pub const SCREEN_H: f32 = 768.0;

/// C# `RollDialog.Setup` 的控件矩形
/// 骰子：`Size = new Size(65, 65); Location = ((SW/2)-38, (SH/2)-40)`
pub const DIE_CONTROL_SIZE: (f32, f32) = (65.0, 65.0);
pub const DIE_ORIGIN: (f32, f32) = (SCREEN_W / 2.0 - 38.0, SCREEN_H / 2.0 - 40.0); // (474,344)
/// 尤茨：`Size = new Size(180, 130); Location = ((SW/2)-90, (SH/2)-65)`
pub const YUT_CONTROL_SIZE: (f32, f32) = (180.0, 130.0);
pub const YUT_ORIGIN: (f32, f32) = (SCREEN_W / 2.0 - 90.0, SCREEN_H / 2.0 - 65.0); // (422,319)
/// 掷骰帧（C# `Setup` / `Roll`）
pub const DIE_IDLE_INDEX: usize = 282;
pub const DIE_ANIM_INDEX: usize = 290;
pub const DIE_ANIM_FRAMES: usize = 4;
/// C# `_currentLoop < 5` → 第 6 次 `AfterAnimation` 才出结果
pub const DIE_ANIM_LOOPS: usize = 6;
pub const YUT_IDLE_INDEX: usize = 2581;
pub const YUT_ANIM_FRAMES: usize = 6;
/// C# `AnimationDelay = 100`
pub const FRAME_SECS: f32 = 0.1;
/// 结果帧基数（C# `_image.Index = 281 + _result` / `2587 + _result`）
pub const DIE_RESULT_BASE: usize = 281;
pub const YUT_RESULT_BASE: usize = 2587;
/// 结果态停留多久自动关闭（**仅 autoRoll**：C# 需玩家点图关闭，本端自动化链路 `--roll-test`
/// 没有点击注入，保留原有的定时关闭；手动掷骰仍按 C# 等点击）
pub const AUTO_RESULT_HOLD_SECS: f32 = 2.0;

/// 掷骰相位（对应 C# `_animation.Visible` / `_image.Visible` 的组合）
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum RollPhase {
    /// 静止：显示空闲帧，点图开掷
    #[default]
    Idle,
    /// 转动中：播放动画帧（此期间点击无效，C# `_image_Click` 的 `if (_rolling) return;`）
    Rolling,
    /// 出结果：显示结果帧，点图关闭；回调已在进入本相位时发出
    Result,
}

/// 控件绘制原点（不含帧自带 offset）
pub fn roll_origin(kind: i32) -> (f32, f32) {
    if kind == 1 {
        YUT_ORIGIN
    } else {
        DIE_ORIGIN
    }
}

/// 控件 `Size`（仅命中框；绘制尺寸见 `roll_frame` 的帧原生尺寸）
pub fn roll_control_size(kind: i32) -> (f32, f32) {
    if kind == 1 {
        YUT_CONTROL_SIZE
    } else {
        DIE_CONTROL_SIZE
    }
}

/// 转动时长（C# 帧数 × AnimationDelay）
pub fn roll_duration(kind: i32) -> f32 {
    if kind == 1 {
        YUT_ANIM_FRAMES as f32 * FRAME_SECS
    } else {
        // 骰子 = 4 帧 × 6 轮
        (DIE_ANIM_FRAMES * DIE_ANIM_LOOPS) as f32 * FRAME_SECS
    }
}

/// 当前应显示的帧（lib, index）
pub fn roll_frame(kind: i32, phase: RollPhase, elapsed: f32, result: i32) -> (LibraryName, usize) {
    let yut = kind == 1;
    match phase {
        RollPhase::Idle => {
            if yut {
                (LibraryName::Items, YUT_IDLE_INDEX)
            } else {
                (LibraryName::Prguse, DIE_IDLE_INDEX)
            }
        }
        RollPhase::Rolling => {
            let step = (elapsed / FRAME_SECS).max(0.0) as usize;
            if yut {
                (
                    LibraryName::Items,
                    YUT_IDLE_INDEX + step.min(YUT_ANIM_FRAMES - 1),
                )
            } else {
                (LibraryName::Prguse, DIE_ANIM_INDEX + step % DIE_ANIM_FRAMES)
            }
        }
        RollPhase::Result => {
            let r = result.clamp(1, 6) as usize;
            if yut {
                (LibraryName::Items, YUT_RESULT_BASE + r)
            } else {
                (LibraryName::Prguse, DIE_RESULT_BASE + r)
            }
        }
    }
}

/// 掷骰状态（网络 Roll 包填充）
#[derive(Resource, Default)]
pub struct RollState {
    pub visible: bool,
    /// 0=骰子 1=尤茨
    pub r#type: i32,
    /// 掷完回调的 NPC 页
    pub page: String,
    /// 结果 1-6
    pub result: i32,
    pub auto_roll: bool,
    /// 回调 NPC object_id（来自当前 NPC 对话框）
    pub npc_id: u32,
    /// 当前相位
    pub phase: RollPhase,
    /// 相位起点是否已初始化（不能用 `started_at == 0.0` 当哨兵：`Time::elapsed_secs()` 在
    /// 极早期也可能为 0，会被误判成「未初始化」而每帧重置起点）
    pub started: bool,
    /// 当前相位起始时间（秒，来自 `Time::elapsed_secs`）
    pub started_at: f32,
    /// 回调是否已发出（进入 Result 相位即置真）
    pub finished: bool,
}

#[derive(Component)]
pub struct RollWidget;

#[derive(Component)]
pub struct RollResultImage;

#[derive(Component)]
pub struct RollPrompt;

pub struct RollPlugin;

impl Plugin for RollPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RollState>();
        app.add_systems(OnEnter(AppState::Game), spawn_roll);
        app.add_systems(OnExit(AppState::Game), cleanup_roll);
        app.add_systems(Update, roll_server_events.run_if(in_state(AppState::Game)));
        app.add_systems(Update, roll_ui_system.run_if(in_state(AppState::Game)));
    }
}

fn cleanup_roll(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_roll(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    // bevy_ui 图案控件（骰子默认图，白 1x1 占位；roll_ui_system 每帧按相位换图 + 尺寸/位置）
    // 用 `Button` + `Interaction` 承接 C# `_image.Click`（点在控件矩形内才生效）
    let white = images.add(crate::map_renderer::make_image(
        vec![255, 255, 255, 255],
        1,
        1,
    ));
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(DIE_ORIGIN.0),
                top: Val::Px(DIE_ORIGIN.1),
                width: Val::Px(DIE_CONTROL_SIZE.0),
                height: Val::Px(DIE_CONTROL_SIZE.1),
                ..default()
            },
            Button,
            ImageNode::new(white),
            GlobalZIndex(50),
            RollResultImage,
            // #2825 单元①：C# `RollDialog` 显式 `Movable = false`（`RollDialog.cs:24`）→ 不可拖
            DialogRoot(DialogKind::Roll),
            NotDraggable,
            RollWidget,
            Visibility::Hidden,
        ))
        .id();

    // 提示文字（**Bevy 扩展**：C# `RollDialog` 没有文字控件，见 `docs/UI_COMPONENTS.md` §7）
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(SCREEN_W / 2.0 - 40.0),
                top: Val::Px(SCREEN_H / 2.0 + 45.0),
                ..default()
            },
            Text::new("点击继续"),
            TextFont {
                font: FontSource::Handle(font.clone()),
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::WHITE),
            GlobalZIndex(51),
            RollPrompt,
            DialogRoot(DialogKind::Roll),
            NotDraggable,
            RollWidget,
            Visibility::Hidden,
        ))
        .id();
}

fn roll_ui_system(
    mut mgr: ResMut<DialogManager>,
    time: Res<Time>,
    mut state: ResMut<RollState>,
    net: ResMut<NetConnection>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut widgets: Query<&mut Visibility, With<RollWidget>>,
    mut img: Query<
        (
            Entity,
            &mut ImageNode,
            &mut Node,
            &Interaction,
            &RollResultImage,
        ),
        Without<RollPrompt>,
    >,
    mut prompt: Query<&mut Text, (With<RollPrompt>, Without<RollResultImage>)>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
    mut last_frame: Local<Option<(LibraryName, usize)>>,
    mut logged: Local<bool>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    crate::game::dialogs::sync_dialog_state(&mut mgr, DialogKind::Roll, state.visible);
    if !state.visible {
        for mut vis in &mut widgets {
            *vis = Visibility::Hidden;
        }
        *last_frame = None;
        *logged = false;
        return;
    }
    for mut vis in &mut widgets {
        *vis = Visibility::Visible;
    }

    if !state.started {
        state.started = true;
        state.started_at = time.elapsed_secs();
    }
    let now = time.elapsed_secs();
    let elapsed = now - state.started_at;

    // 点击边沿（C# `_image.Click += _image_Click`）
    let mut clicked = false;
    for (e, _, _, inter, _) in &mut img {
        if edge(e, inter, &mut prev_inter) {
            clicked = true;
        }
    }

    // 相位推进（C# `Setup` / `_image_Click` / `AfterAnimation`）
    match state.phase {
        RollPhase::Idle => {
            if clicked {
                state.phase = RollPhase::Rolling;
                state.started_at = now;
                state.finished = false;
                tracing::info!(
                    "🎲 掷骰开始: type={} auto={}",
                    state.r#type,
                    state.auto_roll
                );
            }
        }
        RollPhase::Rolling => {
            // 转动中点击无效（C# `if (_rolling) return;`）
            if elapsed >= roll_duration(state.r#type) {
                state.phase = RollPhase::Result;
                state.started_at = now;
                // C# `ReturnResult()`：动画结束即回调 `C.CallNPC{Key="[page]"}`
                if !state.finished {
                    state.finished = true;
                    net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                        object_id: state.npc_id,
                        key: format!("[{}]", state.page),
                    });
                    tracing::info!(
                        "🎲 掷骰完成，回调 NPC {} 页 [{}]（result={}）",
                        state.npc_id,
                        state.page,
                        state.result
                    );
                }
            }
        }
        RollPhase::Result => {
            // C#：`_rolled` 后点图 `Hide()`；autoRoll 链路（`--roll-test`，无点击注入）保留定时关闭
            let auto_close = state.auto_roll && elapsed >= AUTO_RESULT_HOLD_SECS;
            if clicked || auto_close {
                state.visible = false;
                state.started = false;
                state.started_at = 0.0;
            }
        }
    }

    // 渲染当前帧（帧原生尺寸 + `.Lib` 自带 offset，对齐 C# `UseOffSet = true`）
    let elapsed = (time.elapsed_secs() - state.started_at).max(0.0);
    let (lib, idx) = roll_frame(state.r#type, state.phase, elapsed, state.result);
    let origin = roll_origin(state.r#type);
    if *last_frame != Some((lib, idx)) {
        if let Some(h) = load_lib_image(&mut libs, &mut images, lib, idx) {
            for (_, mut node, _, _, _) in &mut img {
                node.image = h.clone();
            }
            *last_frame = Some((lib, idx));
        }
    }
    if let Some((w, h, ox, oy)) = libs.0.get_image(lib, idx).map(|i| {
        (
            i.width as f32,
            i.height as f32,
            i.offset_x as f32,
            i.offset_y as f32,
        )
    }) {
        for (_, _, mut n, _, _) in &mut img {
            n.left = Val::Px(origin.0 + ox);
            n.top = Val::Px(origin.1 + oy);
            n.width = Val::Px(w);
            n.height = Val::Px(h);
        }
    }
    if !*logged {
        tracing::info!(
            "🎲 掷骰子: type={} result={} page={} auto={}",
            state.r#type,
            state.result,
            state.page,
            state.auto_roll
        );
        *logged = true;
    }
    for mut text in &mut prompt {
        let want = match state.phase {
            RollPhase::Idle => "点击掷骰",
            RollPhase::Rolling => "掷骰中...",
            RollPhase::Result => "点击关闭",
        };
        if text.0 != want {
            text.0 = want.to_string();
        }
    }
}

/// 消费服务端 Roll 事件（网络层只广播 ServerEvent）
fn roll_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut roll: ResMut<RollState>,
    npc_dialog: Res<NpcDialogState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::Roll {
            r#type,
            page,
            result,
            auto_roll,
            visible,
            started_at,
            finished,
        } = ev
        {
            // npc_id 来自当前 NPC 对话框（原网络层直读，现由数据所有者提供）
            roll.npc_id = npc_dialog.npc_object_id;
            roll.r#type = *r#type;
            roll.page = page.clone();
            roll.result = *result;
            roll.auto_roll = *auto_roll;
            roll.visible = *visible;
            // 相位起点：C# `Setup` 末尾 `if (autoRoll) Roll();` → 自动掷直接进 Rolling，
            // 否则停在 Idle 等玩家点图。`started_at`/`finished` 由 `roll_ui_system` 维护
            // （网络层的同名字段是过渡期占位，恒 0.0/false）
            roll.phase = if *auto_roll {
                RollPhase::Rolling
            } else {
                RollPhase::Idle
            };
            roll.started = false;
            roll.started_at = *started_at;
            roll.finished = *finished;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2825 单元①：C# `RollDialog.Movable = false`（`RollDialog.cs:24`）→ 本窗两个
    /// `DialogRoot(DialogKind::Roll)`（结果图 + 提示文字）都必须挂 `NotDraggable`
    #[test]
    fn roll_window_roots_are_not_draggable() {
        use crate::game::dialogs::{DialogKind, DialogRoot, NotDraggable};
        use crate::resources::libraries::{resolve_data_path, Libraries};
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        world.insert_resource(crate::map_renderer::GameLibraries(Libraries::new(
            resolve_data_path(),
        )));
        world.insert_resource(Assets::<Image>::default());
        world.insert_resource(Assets::<Font>::default());
        world.insert_resource(crate::ui::sprite_ui::UiFont::default());
        world
            .run_system_once(spawn_roll)
            .expect("spawn_roll 应成功");

        let mut q = world.query::<(Entity, &DialogRoot)>();
        let roots: Vec<Entity> = q
            .iter(&world)
            .filter(|(_, r)| r.0 == DialogKind::Roll)
            .map(|(e, _)| e)
            .collect();
        assert_eq!(roots.len(), 2, "RollDialog 有结果图 + 提示文字两个根");
        for e in roots {
            assert!(
                world.entity(e).contains::<NotDraggable>(),
                "Roll 根 {e:?} 缺 NotDraggable（C# Movable = false）"
            );
        }
        crate::game::dialogs::test_support::assert_no_drag_start(
            &mut world,
            bevy::math::Vec2::new(506.0, 376.0),
        );
    }

    /// C# `RollDialog.Setup`（`RollDialog.cs:78-114`）的两种布局与帧表。
    ///
    /// 阳性对照：把 `YUT_ORIGIN` 改成骰子的 (474,344)（即修正前本端「尤茨也用骰子原点、
    /// 只是把尺寸改成 180x130」的写法）→ 本测试 FAILED。
    #[test]
    fn roll_layout_matches_csharp_setup() {
        // 骰子：Size 65x65 @ ((1024/2)-38, (768/2)-40)
        assert_eq!(DIE_CONTROL_SIZE, (65.0, 65.0));
        assert_eq!(DIE_ORIGIN, (474.0, 344.0));
        // 尤茨：Size 180x130 @ ((1024/2)-90, (768/2)-65)
        assert_eq!(YUT_CONTROL_SIZE, (180.0, 130.0));
        assert_eq!(YUT_ORIGIN, (422.0, 319.0));
        assert_eq!(roll_origin(0), DIE_ORIGIN);
        assert_eq!(roll_origin(1), YUT_ORIGIN);
        // C# 帧表
        assert_eq!(DIE_IDLE_INDEX, 282);
        assert_eq!(
            (DIE_ANIM_INDEX, DIE_ANIM_FRAMES, DIE_ANIM_LOOPS),
            (290, 4, 6)
        );
        assert_eq!((YUT_IDLE_INDEX, YUT_ANIM_FRAMES), (2581, 6));
        assert_eq!((DIE_RESULT_BASE, YUT_RESULT_BASE), (281, 2587));
        assert_eq!(FRAME_SECS, 0.1);
    }

    /// 相位 → 帧映射：Idle / Rolling（轮播与截断）/ Result（`281+result` / `2587+result`）。
    #[test]
    fn roll_frames_follow_csharp_phase_machine() {
        use LibraryName::{Items, Prguse};
        // Idle
        assert_eq!(roll_frame(0, RollPhase::Idle, 0.0, 4), (Prguse, 282));
        assert_eq!(roll_frame(1, RollPhase::Idle, 0.0, 4), (Items, 2581));
        // Rolling：骰子 4 帧循环
        for (t, want) in [(0.0, 290), (0.09, 290), (0.1, 291), (0.35, 293), (0.4, 290)] {
            assert_eq!(
                roll_frame(0, RollPhase::Rolling, t, 4),
                (Prguse, want),
                "骰子 t={t}"
            );
        }
        // Rolling：尤茨 6 帧不循环（超出后停在末帧）
        assert_eq!(roll_frame(1, RollPhase::Rolling, 0.0, 4), (Items, 2581));
        assert_eq!(roll_frame(1, RollPhase::Rolling, 0.5, 4), (Items, 2586));
        assert_eq!(roll_frame(1, RollPhase::Rolling, 5.0, 4), (Items, 2586));
        // Result：C# `281 + result` / `2587 + result`
        for r in 1..=6 {
            assert_eq!(
                roll_frame(0, RollPhase::Result, 0.0, r),
                (Prguse, 281 + r as usize)
            );
            assert_eq!(
                roll_frame(1, RollPhase::Result, 0.0, r),
                (Items, 2587 + r as usize)
            );
        }
    }

    /// 转动时长：骰子 4 帧 × 6 轮 × 100ms = 2.4s（C# `_currentLoop < 5` 的 6 次 `AfterAnimation`）；
    /// 尤茨 6 帧 × 100ms = 0.6s。
    #[test]
    fn roll_duration_matches_csharp_loops() {
        assert!((roll_duration(0) - 2.4).abs() < 1e-6, "骰子 2.4s");
        assert!((roll_duration(1) - 0.6).abs() < 1e-6, "尤茨 0.6s");
    }

    /// 真实 spawn 出的滚动图 / 提示文字：登记 `Interaction` 承接 C# `_image.Click`；
    /// 提示文字是 Bevy 扩展（C# 无文字控件）。
    #[test]
    fn roll_image_is_clickable_button() {
        use crate::resources::libraries::{resolve_data_path, Libraries};
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        world.insert_resource(crate::map_renderer::GameLibraries(Libraries::new(
            resolve_data_path(),
        )));
        world.insert_resource(Assets::<Image>::default());
        world.insert_resource(Assets::<Font>::default());
        world.insert_resource(crate::ui::sprite_ui::UiFont::default());
        world
            .run_system_once(spawn_roll)
            .expect("spawn_roll 应成功");

        let mut q = world.query_filtered::<&Node, With<RollResultImage>>();
        let nodes: Vec<(f32, f32)> = q
            .iter(&world)
            .map(|n| match (n.left, n.top, n.width, n.height) {
                (Val::Px(l), Val::Px(t), Val::Px(w), Val::Px(h)) => (l + t, w + h),
                _ => (f32::NAN, f32::NAN),
            })
            .collect();
        assert_eq!(nodes.len(), 1, "应恰好 1 个滚动图");
        assert_eq!(nodes[0], (DIE_ORIGIN.0 + DIE_ORIGIN.1, 65.0 + 65.0));
        let mut q2 = world
            .query_filtered::<Entity, (With<RollResultImage>, With<Button>, With<Interaction>)>();
        assert_eq!(
            q2.iter(&world).count(),
            1,
            "滚动图必须是可点击的 Button（C# `_image.Click`）"
        );
    }

    /// 真实跑 `roll_ui_system` 走完整相位机：autoRoll 直接转动 → 到时出结果并置 `finished`
    /// → 结果停留 2s 后自动关闭（自动化链路 `--roll-test` 依赖这条路径）。
    ///
    /// 阳性对照：把 `roll_duration(0)` 改成 0.0 → 一进游戏相位就跳到 Result（本测试断言
    /// 「2.5s 前仍在 Rolling」会 FAILED）。
    #[test]
    fn roll_phase_machine_runs_through_rolling_to_result() {
        use crate::resources::libraries::{resolve_data_path, Libraries};
        use bevy::ecs::system::RunSystemOnce;

        if !crate::resources::libraries::data_assets_present() {
            eprintln!("skip roll_phase_machine_runs_through_rolling_to_result: 无 Data 资产");
            return;
        }
        let mut world = World::new();
        world.insert_resource(crate::map_renderer::GameLibraries(Libraries::new(
            resolve_data_path(),
        )));
        world.insert_resource(Assets::<Image>::default());
        world.insert_resource(Assets::<Font>::default());
        world.insert_resource(crate::ui::sprite_ui::UiFont::default());
        world.insert_resource(DialogManager::default());
        world.insert_resource(NetConnection::default());
        world.insert_resource(Time::<()>::default());
        world
            .run_system_once(spawn_roll)
            .expect("spawn_roll 应成功");

        let mut roll = RollState {
            visible: true,
            r#type: 0,
            page: "TestRoll".to_string(),
            result: 4,
            auto_roll: true,
            npc_id: 7,
            phase: RollPhase::Rolling,
            started: false,
            started_at: 0.0,
            finished: false,
        };
        // 第 1 帧：记录相位起点，仍是 Rolling
        world.insert_resource(std::mem::take(&mut roll));
        world
            .run_system_once(roll_ui_system)
            .expect("roll_ui_system 应成功");
        let s = world.resource::<RollState>();
        assert_eq!(s.phase, RollPhase::Rolling, "autoRoll 应直接进 Rolling");
        assert!(!s.finished, "转动中不应发回调");

        // t=1.0s：骰子总时长 2.4s，仍在 Rolling
        world
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(1.0));
        world
            .run_system_once(roll_ui_system)
            .expect("roll_ui_system 应成功");
        assert_eq!(
            world.resource::<RollState>().phase,
            RollPhase::Rolling,
            "1.0s < 2.4s 仍在转动"
        );

        // t=2.5s：过 2.4s → Result + finished（C# `AfterAnimation` → `ReturnResult`）
        world
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(1.5));
        world
            .run_system_once(roll_ui_system)
            .expect("roll_ui_system 应成功");
        let s = world.resource::<RollState>();
        assert_eq!(s.phase, RollPhase::Result, "2.5s ≥ 2.4s 应出结果");
        assert!(s.finished, "出结果应发回调（`finished` = true）");
        assert!(s.visible, "结果态仍可见（等点击/定时关闭）");

        // 再 +2.5s：autoRoll 结果停留期满 → 关闭
        world
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(2.5));
        world
            .run_system_once(roll_ui_system)
            .expect("roll_ui_system 应成功");
        assert!(
            !world.resource::<RollState>().visible,
            "autoRoll 结果停留 {}s 后应自动关闭",
            AUTO_RESULT_HOLD_SECS
        );
    }
}
