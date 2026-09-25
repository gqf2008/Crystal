// ============================================================================
// 计时器对话框（M50）
// 布局基准：C# `Client/MirScenes/Dialogs/TimerDialog.cs`（#2892 批C）
//   - `MirControl`（**无背景图**）`Size = 120x100` @ `(ScreenWidth-120, ScreenHeight-230)`
//     = (904,538)，`NotControl = true`、`Movable = false`（`:27-31`）
//   - `_eggTimer`：`Prguse2[960..965]` 6 帧、333ms、`Loop=true`、@(23,0)（`:33-46`）
//   - 数字位（`UseOffSet`）：`_1000`@(0,70)、`_100`@(22,70)、`_colon`=`Prguse2[910]`@(44,70)、
//     `_10`@(58,70)、`_1`@(80,70)，索引 = `900 + 数字`（`:48-97`）
//   - `UpdateTimeGraphic`：小时>0 → HHMM，否则 MMSS（`:180-196`）
//   - `Type`（= wire `kind`）：0 → 沙漏隐藏；1 → `Prguse2[960]`；2 → `Prguse2[440]`（`:201-216`）
// 此前是 Bevy 自造的 `Prguse[170]` 拉伸面板 + 文本行（面板/位置/内容都与原版不同）。
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot, NotDraggable};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{load_lib_image, UiRootDisplay};

/// C# `TimerDialog.cs:27-31`：窗口是 `MirControl`（无背景图），120x100 @ (904,538)
pub const PANEL_W: f32 = 120.0;
pub const PANEL_H: f32 = 100.0;
pub const PANEL_ORIGIN: (f32, f32) = (1024.0 - PANEL_W, 768.0 - 230.0);

/// `_eggTimer` @(23,0)，6 帧、333ms（C# `AnimationCount/AnimationDelay`）
pub const EGG_POS: (f32, f32) = (23.0, 0.0);
pub const EGG_SIZE: (f32, f32) = (52.0, 52.0);
pub const EGG_FRAMES: usize = 6;
pub const EGG_DELAY_SECS: f32 = 0.333;
/// 沙漏帧基址：`Type=1` → 960，`Type=2` → 440（C# `:201-216`）
pub const EGG_BASE_KIND1: usize = 960;
pub const EGG_BASE_KIND2: usize = 440;

/// 四个数字位左边界（`_1000`/`_100`/`_10`/`_1`，C# `:48-97`）
pub const DIGIT_X: [f32; 4] = [0.0, 22.0, 58.0, 80.0];
pub const DIGIT_Y: f32 = 70.0;
/// 数字精灵基址：`Index = 900 + 数字`
pub const DIGIT_BASE: usize = 900;
/// 冒号 `Prguse2[910]` @(44,70)
pub const COLON_POS: (f32, f32) = (44.0, 70.0);
pub const COLON_INDEX: usize = 910;

/// 状态
#[derive(Resource, Default)]
pub struct TimerState {
    /// 兼容字段（C# 窗口内无文本；供自动化/日志读取）
    pub message: String,
    /// #230：网络计时器是否激活（S.SetTimer 启动 / S.ExpireTimer 或倒计时归零关闭）
    pub active: bool,
    /// 剩余秒（供自动化/日志；由 `counter` 派生）
    pub remaining: f32,
    /// C# `_timerCounter`：整秒倒计时（每秒递减，<0 时窗口隐藏，`TimerDialog.cs:112-127`）
    pub counter: i32,
    /// 秒累加器（C# `_timerTime = CMain.Time + 1000` 的等价物）
    pub accum: f32,
    /// C# `ClientTimer.Type`（0/1/2，决定沙漏帧基址）
    pub kind: u8,
    /// C# `ClientTimer.Key`
    pub timer_key: i32,
}

#[derive(Component)]
pub struct TimerWidget;

/// 沙漏动画（`_eggTimer`）
#[derive(Component)]
pub struct TimerSandglass;

/// 数字位（0..3，从左到右 = C# `_1000`/`_100`/`_10`/`_1`）
#[derive(Component)]
pub struct TimerDigit(pub usize);

/// 冒号（`_colon`）
#[derive(Component)]
pub struct TimerColon;

/// 计时器窗口用到的精灵表（spawn 时加载，UI 系统换帧/换数字）
#[derive(Resource, Default)]
pub struct TimerImages {
    /// 两档沙漏帧表（`[0]` = `Prguse2[960..]`, `[1]` = `Prguse2[440..]`）
    pub egg: [Vec<Handle<Image>>; 2],
    /// 数字 `Prguse2[900..909]`
    pub digits: Vec<Handle<Image>>,
    /// 每个数字精灵的 `UseOffSet` 偏移（C# 绘制位置 = Location + 精灵 offset）
    pub digit_offsets: Vec<(f32, f32)>,
    /// 数字精灵原生尺寸
    pub digit_sizes: Vec<(f32, f32)>,
    /// 冒号 `Prguse2[910]`
    pub colon: Option<Handle<Image>>,
    pub colon_size: (f32, f32),
}

/// C# `UpdateTimeGraphic`：返回四个数字位（左→右）显示的 0..9
/// —— 小时>0 → HHMM，否则 MMSS（`TimerDialog.cs:180-196`）
pub fn timer_digits(counter: i32) -> [usize; 4] {
    let secs = counter.max(0);
    let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    if h > 0 {
        [
            (h / 10) as usize % 10,
            (h % 10) as usize,
            (m / 10) as usize % 10,
            (m % 10) as usize,
        ]
    } else {
        [
            (m / 10) as usize % 10,
            (m % 10) as usize,
            (s / 10) as usize % 10,
            (s % 10) as usize,
        ]
    }
}

/// C# `Type` → 沙漏帧基址（`None` = Type 0，沙漏隐藏）
pub fn egg_base_for_kind(kind: u8) -> Option<usize> {
    match kind {
        1 => Some(EGG_BASE_KIND1),
        2 => Some(EGG_BASE_KIND2),
        _ => None,
    }
}

pub struct TimerPlugin;

impl Plugin for TimerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TimerState>();
        app.add_systems(OnEnter(AppState::Game), spawn_timer);
        app.add_systems(OnExit(AppState::Game), cleanup_timer);
        app.add_systems(
            Update,
            (timer_network_events, timer_countdown, timer_ui_system)
                .chain()
                .after(crate::network::network_system)
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_timer(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_timer(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut cjk_font: ResMut<UiCjkFont>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        crate::ui::sprite_ui::ensure_ui_font(&mut fonts, &mut ui_font);
    }
    let _ = shared_cjk_font(&mut fonts, &mut cjk_font); // C# 窗口内无文本，仅沙漏 + 数字位

    // 精灵表：两档沙漏 + 数字 900..909 + 冒号 910
    let mut table = TimerImages::default();
    for (slot, base) in [(0usize, EGG_BASE_KIND1), (1usize, EGG_BASE_KIND2)] {
        for i in 0..EGG_FRAMES {
            if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, base + i)
            {
                table.egg[slot].push(h);
            }
        }
    }
    for d in 0..10usize {
        let idx = DIGIT_BASE + d;
        let info = libs.0.get_image(LibraryName::Prguse2, idx);
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, idx) {
            let (w, hh, ox, oy) = info
                .map(|i| {
                    (
                        i.width.max(0) as f32,
                        i.height.max(0) as f32,
                        i.offset_x as f32,
                        i.offset_y as f32,
                    )
                })
                .unwrap_or((20.0, 22.0, 0.0, 0.0));
            table.digits.push(h);
            table.digit_sizes.push((w, hh));
            table.digit_offsets.push((ox, oy));
        }
    }
    if let Some(c) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, COLON_INDEX) {
        let size = libs
            .0
            .get_image(LibraryName::Prguse2, COLON_INDEX)
            .map(|i| (i.width.max(0) as f32, i.height.max(0) as f32))
            .unwrap_or((12.0, 19.0));
        table.colon = Some(c);
        table.colon_size = size;
    }

    // C# `TimerDialog` 是 `MirControl`（无背景图）：透明容器 120x100 @(904,538)
    let panel = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(PANEL_ORIGIN.0),
                top: Val::Px(PANEL_ORIGIN.1),
                width: Val::Px(PANEL_W),
                height: Val::Px(PANEL_H),
                ..default()
            },
            Visibility::Hidden,
            UiRootDisplay::default(),
            GlobalZIndex(30),
            DialogRoot(DialogKind::Timer),
            TimerWidget,
            // #2825 单元①：C# `TimerDialog` 显式 `Movable = false`（`TimerDialog.cs:29`），
            // 且 `MirControl._movable` 默认 false（`MirControl.cs:372`）→ 本端不可拖动
            NotDraggable,
        ))
        .id();

    commands.entity(panel).with_children(|p| {
        // `_eggTimer`：`Prguse2[960]` 6 帧 @(23,0)（Type 0 时整帧隐藏）
        if let Some(first) = table.egg[0].first().or(table.egg[1].first()) {
            p.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(EGG_POS.0),
                    top: Val::Px(EGG_POS.1),
                    width: Val::Px(EGG_SIZE.0),
                    height: Val::Px(EGG_SIZE.1),
                    ..default()
                },
                ImageNode::new(first.clone()),
                Visibility::Hidden,
                ZIndex(9),
                TimerSandglass,
            ));
        }
        // 四个数字位（`UseOffSet` → 位置 = Location + 精灵 offset）
        let (dw, dh) = table.digit_sizes.first().copied().unwrap_or((20.0, 22.0));
        let (dox, doy) = table.digit_offsets.first().copied().unwrap_or((0.0, 0.0));
        let digit_img = table.digits.first().cloned();
        for (i, x) in DIGIT_X.iter().enumerate() {
            let Some(img) = digit_img.clone() else { break };
            p.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(x + dox),
                    top: Val::Px(DIGIT_Y + doy),
                    width: Val::Px(dw),
                    height: Val::Px(dh),
                    ..default()
                },
                ImageNode::new(img),
                Visibility::Hidden,
                ZIndex(9),
                TimerDigit(i),
            ));
        }
        // 冒号 `Prguse2[910]` @(44,70)
        if let Some(c) = table.colon.clone() {
            p.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(COLON_POS.0),
                    top: Val::Px(COLON_POS.1),
                    width: Val::Px(table.colon_size.0),
                    height: Val::Px(table.colon_size.1),
                    ..default()
                },
                ImageNode::new(c),
                Visibility::Hidden,
                ZIndex(9),
                TimerColon,
            ));
        }
    });
    commands.insert_resource(table);
}

/// 显隐 + 渲染（C# `TimerDialog.UpdateTimeGraphic` `:180-216`）
fn timer_ui_system(
    mgr: Res<DialogManager>,
    timer: Res<TimerState>,
    table: Res<TimerImages>,
    time: Res<Time>,
    // B0001：四个查询都要可变访问 `Visibility`，必须用 `Without` 显式互斥
    mut widgets: Query<
        &mut Visibility,
        (
            With<TimerWidget>,
            Without<TimerSandglass>,
            Without<TimerDigit>,
            Without<TimerColon>,
        ),
    >,
    mut egg: Query<
        (&mut ImageNode, &mut Visibility),
        (
            With<TimerSandglass>,
            Without<TimerDigit>,
            Without<TimerColon>,
        ),
    >,
    mut digits: Query<
        (&TimerDigit, &mut ImageNode, &mut Node, &mut Visibility),
        (Without<TimerSandglass>, Without<TimerColon>),
    >,
    mut colon: Query<
        &mut Visibility,
        (
            With<TimerColon>,
            Without<TimerSandglass>,
            Without<TimerDigit>,
        ),
    >,
    mut egg_anim: Local<(usize, f32)>,
) {
    let open = timer.active && mgr.is_open(DialogKind::Timer);
    for mut vis in widgets.iter_mut() {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        return;
    }

    // 沙漏：C# `Type` 0 → 隐藏（`_eggTimer.Visible = false`）；1/2 → 960/440 起的 6 帧、333ms、Loop
    let frames = match timer.kind {
        1 => Some(&table.egg[0]),
        2 => Some(&table.egg[1]),
        _ => None,
    };
    let frame_img = match frames {
        Some(f) if !f.is_empty() => {
            egg_anim.1 += time.delta_secs();
            while egg_anim.1 >= EGG_DELAY_SECS {
                egg_anim.1 -= EGG_DELAY_SECS;
                egg_anim.0 += 1;
            }
            Some(f[egg_anim.0 % f.len()].clone())
        }
        _ => {
            egg_anim.0 = 0;
            egg_anim.1 = 0.0;
            None
        }
    };
    for (mut img, mut vis) in &mut egg {
        match &frame_img {
            Some(h) => {
                *vis = Visibility::Visible;
                if img.image != *h {
                    img.image = h.clone();
                }
            }
            None => *vis = Visibility::Hidden,
        }
    }

    // 数字位：小时>0 → HHMM，否则 MMSS（`UpdateTimeGraphic`）
    let want = timer_digits(timer.counter);
    for (digit, mut img, mut node, mut vis) in &mut digits {
        let value = want.get(digit.0).copied().unwrap_or(0);
        *vis = Visibility::Visible;
        let Some(h) = table.digits.get(value) else {
            continue;
        };
        if img.image != *h {
            img.image = h.clone();
        }
        // `UseOffSet`：绘制位置 = Location + 精灵 offset
        let (ox, oy) = table
            .digit_offsets
            .get(value)
            .copied()
            .unwrap_or((0.0, 0.0));
        node.left = Val::Px(DIGIT_X.get(digit.0).copied().unwrap_or(0.0) + ox);
        node.top = Val::Px(DIGIT_Y + oy);
    }
    for mut vis in &mut colon {
        *vis = Visibility::Visible;
    }
}

/// #230：S.SetTimer / S.ExpireTimer → 打开/关闭计时器对话框
fn timer_network_events(
    mut mgr: ResMut<DialogManager>,
    mut timer: ResMut<TimerState>,
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
) {
    for ev in events.read() {
        match ev {
            crate::network::server_event::ServerEvent::TimerSet {
                timer_id,
                seconds,
                kind,
            } => {
                timer.active = true;
                timer.timer_key = *timer_id;
                timer.counter = (*seconds).max(0);
                timer.remaining = timer.counter as f32;
                timer.accum = 0.0;
                timer.kind = *kind;
                timer.message = format!("剩余 {} 秒", timer.counter);
                if !mgr.is_open(DialogKind::Timer) {
                    mgr.open(DialogKind::Timer);
                }
                tracing::info!(
                    "⏱️ [TIMER] 启动计时器 key={} {} 秒 类型={}",
                    timer_id,
                    seconds,
                    kind
                );
            }
            crate::network::server_event::ServerEvent::TimerExpired { .. } => {
                timer.active = false;
                timer.remaining = 0.0;
                timer.counter = 0;
                timer.message.clear();
                mgr.close(DialogKind::Timer);
                tracing::info!("⏱️ [TIMER] 计时器关闭");
            }
            _ => {}
        }
    }
}

/// #230：倒计时（C# `TimerDialog.Process` `:112-127`：每秒 `_timerCounter--`，
/// `< 0` 时隐藏窗口并移除该计时器）
fn timer_countdown(time: Res<Time>, mut mgr: ResMut<DialogManager>, mut timer: ResMut<TimerState>) {
    if !timer.active {
        return;
    }
    timer.accum += time.delta_secs();
    while timer.accum >= 1.0 {
        timer.accum -= 1.0;
        timer.counter -= 1;
        if timer.counter < 0 {
            timer.active = false;
            timer.remaining = 0.0;
            timer.message.clear();
            mgr.close(DialogKind::Timer);
            tracing::info!("⏱️ [TIMER] 倒计时归零");
            return;
        }
    }
    timer.remaining = timer.counter as f32;
    timer.message = format!("剩余 {} 秒", timer.counter);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2825 单元①：C# `TimerDialog.Movable = false`（`TimerDialog.cs:29`）→ 本窗
    /// **所有** `DialogRoot(DialogKind::Timer)` 都必须挂 `NotDraggable`，否则拖动系统仍会命中。
    #[test]
    fn timer_window_roots_are_not_draggable() {
        use crate::game::dialogs::{DialogKind, DialogRoot, NotDraggable};
        use crate::resources::libraries::{resolve_data_path, Libraries};
        use bevy::ecs::system::RunSystemOnce;

        // CI 无游戏资产（Data/ 不入库）→ 跳过（详见 libraries::data_assets_present）
        if !crate::resources::libraries::data_assets_present() {
            eprintln!("skip timer_window_roots_are_not_draggable: 无 Data 资产");
            return;
        }
        let mut world = World::new();
        world.insert_resource(crate::map_renderer::GameLibraries(Libraries::new(
            resolve_data_path(),
        )));
        world.insert_resource(Assets::<Image>::default());
        world.insert_resource(Assets::<Font>::default());
        world.insert_resource(crate::ui::sprite_ui::UiCjkFont::default());
        world.insert_resource(crate::ui::sprite_ui::UiFont::default());
        world
            .run_system_once(spawn_timer)
            .expect("spawn_timer 应成功");

        let mut q = world.query::<(Entity, &DialogRoot)>();
        let roots: Vec<Entity> = q
            .iter(&world)
            .filter(|(_, r)| r.0 == DialogKind::Timer)
            .map(|(e, _)| e)
            .collect();
        assert!(!roots.is_empty(), "应生成 Timer 根面板");
        for e in roots {
            assert!(
                world.entity(e).contains::<NotDraggable>(),
                "Timer 根 {e:?} 缺 NotDraggable（C# Movable = false）"
            );
        }
        // 行为级：面板中心按下左键 → 拖动系统不得起拖
        crate::game::dialogs::test_support::assert_no_drag_start(
            &mut world,
            bevy::math::Vec2::new(PANEL_ORIGIN.0 + 60.0, PANEL_ORIGIN.1 + 50.0),
        );
    }

    /// #2892 批C：数字位映射逐条对齐 C# `TimerDialog.UpdateTimeGraphic`（`:180-196`）——
    /// 小时>0 → HHMM，否则 MMSS。
    #[test]
    fn timer_digits_match_csharp_hhmm_or_mmss() {
        // 0 秒 → 00:00
        assert_eq!(timer_digits(0), [0, 0, 0, 0]);
        // 59 秒 → 00:59（MMSS）
        assert_eq!(timer_digits(59), [0, 0, 5, 9]);
        // 60 秒 → 01:00
        assert_eq!(timer_digits(60), [0, 1, 0, 0]);
        // 599 秒 → 09:59
        assert_eq!(timer_digits(599), [0, 9, 5, 9]);
        // 3600 秒 → 01:00（进入 HHMM 分支）
        assert_eq!(timer_digits(3600), [0, 1, 0, 0]);
        // 7325 秒 = 2h02m → HHMM = 02:02
        assert_eq!(timer_digits(7325), [0, 2, 0, 2]);
        // 负数（归零瞬间）按 0 处理，不 panic
        assert_eq!(timer_digits(-1), [0, 0, 0, 0]);
    }

    /// #2892 批C：`Type` → 沙漏帧基址（C# `:201-216` 的 switch：0 隐藏、1=`Prguse2[960]`、2=`Prguse2[440]`）
    #[test]
    fn egg_frames_match_csharp_type_switch() {
        assert_eq!(egg_base_for_kind(0), None, "Type 0 → 沙漏隐藏");
        assert_eq!(egg_base_for_kind(1), Some(EGG_BASE_KIND1));
        assert_eq!(egg_base_for_kind(2), Some(EGG_BASE_KIND2));
        assert_eq!(
            egg_base_for_kind(3),
            None,
            "未定义 Type 落到 default → 隐藏"
        );
    }

    /// #2892 批C：窗口几何 = C# `(ScreenWidth-120, ScreenHeight-230)`、120x100；
    /// 数字位/冒号坐标取 C# `Location`（`UseOffSet` 偏移在渲染时叠加）。
    #[test]
    fn timer_geometry_matches_csharp() {
        assert_eq!(PANEL_ORIGIN, (904.0, 538.0), "C# (1024-120, 768-230)");
        assert_eq!((PANEL_W, PANEL_H), (120.0, 100.0));
        assert_eq!(EGG_POS, (23.0, 0.0));
        assert_eq!(DIGIT_X, [0.0, 22.0, 58.0, 80.0]);
        assert_eq!(DIGIT_Y, 70.0);
        assert_eq!(COLON_POS, (44.0, 70.0));
    }

    /// #2892 批C：`S.SetTimer` 新增 `kind`（= C# `ClientTimer.Type`）线格式往返无损
    #[test]
    fn set_timer_wire_roundtrip_keeps_kind() {
        use mir2_shared::packets::base::Packet;
        use mir2_shared::packets::server::ui_events::SetTimer as P;
        for kind in [0u8, 1, 2, 3, 255] {
            let p = P {
                timer_id: 7,
                seconds: 42,
                kind,
            };
            let mut buf = Vec::new();
            p.write_body(&mut buf).expect("write_body");
            let mut cur = std::io::Cursor::new(&buf);
            let back = P::read_body(&mut cur).expect("read_body");
            assert_eq!(
                (back.timer_id, back.seconds, back.kind),
                (7, 42, kind),
                "kind={kind} 往返应无损"
            );
        }
    }
}
