// ============================================================================
// mir2 bevy - 主入口
// ============================================================================
// #1418：GUI 子系统——启动不弹日志控制台窗口（否则控制台会盖住游戏窗口，
// 用户看到的是日志而不是游戏画面）。stdout/stderr 重定向仍可用（E2E 脚本不受影响）。
#![windows_subsystem = "windows"]
// 传奇2 (Legend of Mir 2) 客户端 Bevy 移植版
//
// 用法:
//   cargo run --bin client_bevy                     # 默认地图 n0 + 演示角色
//   cargo run --bin client_bevy -- --map n0
//   cargo run --bin client_bevy -- --map 11yearvilliage
//   cargo run --bin client_bevy -- --no-actors      # 只渲染地图（截图验证用）
//   cargo run --bin client_bevy -- --window-title 修复版   # 自定义窗口标题（多实例并行时区分）

use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::render::settings::{Backends, RenderCreation, WgpuSettings};
use bevy::render::RenderPlugin;
use client_bevy::actor::ActorPlugin;
use client_bevy::control::ControlPlugin;
use client_bevy::map_renderer::MapRenderPlugin;
use client_bevy::network::NetworkPlugin;
use client_bevy::scenes::AppState;
use client_bevy::ui::intro::IntroPlugin;
use client_bevy::ui::login::LoginPlugin;
use client_bevy::ui::modal_box::ModalBoxPlugin;
use client_bevy::ui::new_character::NewCharacterPlugin;
use client_bevy::ui::pinyin_ime::PinyinImePlugin;
use client_bevy::ui::select::SelectPlugin;

mod auto;

use std::path::{Path, PathBuf};

/// 解析 assets 根目录（#S5 发布打包修复）。
/// 运行时 exe 相对优先（发布布局 `exe_dir/assets`）；开发期回退编译期
/// CARGO_MANIFEST_DIR——env! 固化的是构建机绝对路径，玩家机器上不存在，
/// shaders/*.wgsl 会静默加载失败（昼夜/灯光失效）。
fn resolve_assets_path() -> String {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("assets"));
        }
    }
    candidates.push(PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets"
    )));
    pick_first_dir(&candidates, |p| p.is_dir())
        .expect("assets 候选链至少含编译期回退")
        .to_string_lossy()
        .into_owned()
}

/// 候选链取首个存在目录；全部缺失时回退最后一个候选（编译期路径）。
fn pick_first_dir(candidates: &[PathBuf], exists: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    candidates
        .iter()
        .find(|p| exists(p))
        .or_else(|| candidates.last())
        .cloned()
}

/// 日志过滤器（#A3）：debug 保持 info 全量；release 默认 warn——拖窗/置顶/chunk
/// 流式全刷 info 会刷屏，仅排障需要的模块（网络收发/断线重连、战斗结算）白名单保留 info。
fn log_filter() -> &'static str {
    if cfg!(debug_assertions) {
        "info,bevy_render=warn,bevy_asset=warn,bevy_log=warn,bevy_diagnostic=warn,wgpu_hal=warn,naga=warn,icu4x=error,icu_segmenter=error"
    } else {
        "warn,client_bevy::network=info,client_bevy::game::combat=info,icu4x=error,icu_segmenter=error"
    }
}

// #71：全局给 UI 实体打 RenderLayers layer 1（由独立 UI 相机渲染，地图相机不重画 UI）
use client_bevy::ui::sprite_ui::mark_ui_render_layers;
// #2521：layer 1 向下传播到 UiEntity 的后代（RenderLayers 不随层级传播）
use client_bevy::ui::sprite_ui::propagate_ui_render_layers;

/// 打印一次窗口度量（物理尺寸 / 逻辑尺寸 / DPI scale）。
///
/// 为什么需要它：本端 UI 全按 1024×768 **逻辑**画布排布，而 `bevy_window::WindowResolution::new`
/// 收的是**物理**像素——高 DPI 机器上两者会分叉（150% 时逻辑只剩 682×512），底部 UI（聊天面板、
/// 底栏）就可能被排到窗口外。这条日志把"到底是真裁切、还是截图/DPI 上下文的问题"一次问清，
/// 避免继续靠猜（2026-09-26：拍中文界面像素证据时卡在这里，见线程
/// `crystal-chat-panel-screenshot-mismatch`）。
///
/// 注意 `scale_factor()` 要等 winit 后端就绪才可信，Startup 里可能还是 1.0 ⇒ 这里等
/// "scale != 1 或者已经跑了 120 帧"再打，只打一次。
fn log_window_metrics(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut frames: Local<u32>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    *frames += 1;
    let Ok(w) = windows.single() else { return };
    let scale: f32 = w.scale_factor();
    if scale == 1.0 && *frames < 120 {
        return;
    }
    let res = &w.resolution;
    tracing::info!(
        "🪟 窗口度量：physical={}x{} logical={:.0}x{:.0} scale={} （设计画布 1024x768 逻辑；logical < 设计画布 ⇒ 底部 UI 会被裁）",
        res.physical_width(),
        res.physical_height(),
        res.width(),
        res.height(),
        scale
    );
    *done = true;
}

fn main() {
    // --window-title <标题>：自定义窗口标题（多实例并行时便于区分，如“修复版”）
    let default_title = "Mir2 (Bevy) — 传奇2 客户端移植".to_string();
    let window_title = {
        let args: Vec<String> = std::env::args().collect();
        args.iter()
            .position(|a| a == "--window-title")
            .and_then(|i| args.get(i + 1))
            .cloned()
            .unwrap_or(default_title)
    };
    let mut app = App::new();
    // 构建戳（build.rs 固化）：owner 多次拿旧构建的截图/体验当缺陷报（写邮件窗「错位」、
    // 底部对话框滚动/对齐、地图灯光、魔法特效），每次都要先花一轮证明「代码早改过了」。
    // 把这行放进启动日志后，任何一次反馈都能一眼看出「他跑的是哪个提交」。
    // 同源数据也由 control RPC `build_stamp` 暴露，供夹具断言"被测 exe 就是当前提交构建的"。
    tracing::info!(
        "🧾 客户端构建：commit={}（{}）dirty={}",
        env!("CRYSTAL_BUILD_COMMIT_SHORT"),
        env!("CRYSTAL_BUILD_COMMIT"),
        env!("CRYSTAL_BUILD_DIRTY")
    );
    // 渲染错误策略必须在 DefaultPlugins 之前装：RenderPlugin::build 走的是
    // `init_resource::<RenderErrorHandler>()`，先插入者胜出。覆盖它只为一件事——
    // 「最小化窗口」触发的那次可恢复表面配置失败不再退进程（bevy 默认策略对任何
    // RenderError 都 AppExit::error()）。详见 src/render_error.rs。
    client_bevy::render_error::install(&mut app);
    app.add_plugins(
        DefaultPlugins
            // assets 目录运行时解析：exe 相对优先（发布），编译期 manifest 回退（开发）
            .set(AssetPlugin {
                file_path: resolve_assets_path(),
                ..default()
            })
            // 使用 DX12 后端（Vulkan 的 swapchain present 在此机器上会冻结）
            .set(RenderPlugin {
                render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                    // 跨平台后端：macOS 用 Metal；其余（Windows 为主）用 DX12，
                    // 避免 Vulkan swapchain present 冻结。此前硬编码 DX12 导致 macOS
                    // 启动即报 "Unable to find a GPU"（wgpu 只枚举 DX12 adapter）。
                    backends: if cfg!(target_os = "macos") {
                        Some(Backends::METAL)
                    } else {
                        Some(Backends::DX12)
                    },
                    ..default()
                })),
                ..default()
            })
            .set(LogPlugin {
                filter: log_filter().into(),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: window_title,
                    resolution: (1024u32, 768u32).into(),
                    // 禁用系统 IME：用游戏内置拼音输入法（src/ui/pinyin_ime.rs）。
                    // winit 用 IACE_CHILDREN 解关联 IME 上下文，字母键作为原始
                    // KeyboardInput 到达，不被手心等系统输入法拦截。
                    ime_enabled: false,
                    // 无 vsync：避免会话中 vblank 缺失导致 present 永久阻塞（画面冻结）
                    present_mode: bevy::window::PresentMode::Immediate,
                    ..default()
                }),
                ..default()
            }),
    );
    app.insert_resource(ClearColor(Color::srgb(0.07, 0.08, 0.12)));
    // 性能（#112）：PresentMode::Immediate 无 vsync 会无限刷帧烧 CPU（基线 ~150% 单核）。
    // 用 winit Reactive 60Hz 限帧：动画/输入照常（事件唤醒 + 16.6ms 心跳），CPU 大幅下降，
    // 且不引入 vsync 阻塞（此前 DX12+Vulkan 均出现过 present 冻结）。
    use std::time::Duration;
    app.insert_resource(bevy::winit::WinitSettings {
        focused_mode: bevy::winit::UpdateMode::reactive(Duration::from_secs_f64(1.0 / 60.0)),
        ..default()
    });
    app.init_state::<AppState>();
    // --skip-login: 直接从登录界面进入游戏（诊断呈现问题用）
    if std::env::args().any(|a| a == "--skip-login") {
        // #UI-align：skip-login 只应触发一次状态跳转。
        // Bevy 每帧 set 同一个 NextState 会反复执行 OnExit/OnEnter，导致游戏场景每帧重建、
        // UI 大量闪烁/残留白块并刷 “Entity despawned” 告警。用 Local<bool> 保证只跳一次。
        app.add_systems(
            Update,
            |mut next: ResMut<NextState<AppState>>, mut done: Local<bool>| {
                if !*done {
                    *done = true;
                    next.set(AppState::Game);
                }
            },
        );
    }
    app.add_plugins(ControlPlugin);
    app.add_plugins(PinyinImePlugin);
    // UI 字体链 Han 回退（复刻 C# GDI：Arial 缺中文→宋体）：Startup 一次注册系统宋体。
    // 必须早于任何 Text2d 布局（首个 Update 前）——Startup 阶段即满足
    app.add_systems(
        Startup,
        client_bevy::ui::sprite_ui::setup_han_fallback_system,
    );
    app.add_plugins((
        NetworkPlugin,
        IntroPlugin,
        LoginPlugin,
        SelectPlugin,
        NewCharacterPlugin,
        ModalBoxPlugin,
        client_bevy::game::GamePlugin,
        client_bevy::ui::client_settings::ClientSettingsPlugin,
    ));
    app.add_systems(Update, (mark_ui_render_layers, propagate_ui_render_layers));
    // 一次窗口度量日志（物理/逻辑/DPI）：用于定位"底部 UI 是否被窗口裁掉"这类问题。
    app.add_systems(Update, log_window_metrics);
    // bevy_ui 迁移：三帧图按钮交互（Interaction → normal/hover/pressed 帧切换）
    app.add_systems(Update, client_bevy::ui::theme::image_button_system);
    // #2742：C# `MirImageControl.GrayScale` 等价灰度（须排在帧切换系统之后：
    // 后者每帧把原帧写回 `ImageNode.image`，本系统再按需替换成灰度变体）
    app.init_resource::<client_bevy::ui::gray::UiGrayCache>();
    app.add_systems(
        Update,
        client_bevy::ui::gray::apply_ui_gray_system
            .after(client_bevy::ui::theme::image_button_system),
    );
    // #91 UI 按钮交互音效（全场景：登录/选角/游戏）
    app.add_systems(Update, client_bevy::ui::sprite_ui::ui_button_sound_system);
    // 文本黑色描边同步（C# MirLabel OutLine：4 方向 1px 黑色副本跟随正文内容变化）。
    // 必须排在所有描边文本写方之后（变更检测按 tick 严格比较，同帧晚于本系统的
    // 零散写入永不可见）：行会名写方以 .before(本系统) 显式排序（actor/mod.rs），
    // 其余写方（tooltip/任务追踪/伤害飘字）注册于本行之前
    app.add_systems(Update, client_bevy::ui::outlined_text::sync_outline_system);
    auto::register(&mut app);
    // --no-actors: 只渲染地图（用于纯地图截图验证）
    if std::env::args().any(|a| a == "--no-actors") {
        app.add_plugins(MapRenderPlugin);
    } else {
        app.add_plugins((MapRenderPlugin, ActorPlugin));
    }
    app.run();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exe_candidate() -> PathBuf {
        PathBuf::from("/exe/assets")
    }

    fn manifest_candidate() -> PathBuf {
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/assets"))
    }

    #[test]
    fn pick_prefers_exe_relative_assets_when_present() {
        // 发布布局：exe 旁 assets 存在 → 用 exe 相对，不落编译期路径
        let candidates = vec![exe_candidate(), manifest_candidate()];
        let picked = pick_first_dir(&candidates, |p| *p == exe_candidate()).unwrap();
        assert_eq!(picked, exe_candidate());
    }

    #[test]
    fn pick_falls_back_to_manifest_when_exe_dir_missing() {
        // 开发布局：exe 在 target/debug，旁边无 assets → 回退编译期 manifest
        let candidates = vec![exe_candidate(), manifest_candidate()];
        let picked = pick_first_dir(&candidates, |p| *p == manifest_candidate()).unwrap();
        assert_eq!(picked, manifest_candidate());
    }

    #[test]
    fn pick_returns_last_candidate_when_all_missing() {
        // 全部缺失 → 仍回退编译期候选（保底给出一个确定路径）
        let candidates = vec![exe_candidate(), manifest_candidate()];
        let picked = pick_first_dir(&candidates, |_| false).unwrap();
        assert_eq!(picked, manifest_candidate());
    }

    #[test]
    fn resolve_assets_path_always_yields_assets_dir() {
        // 本机开发环境：target 下无 assets → 必然落到真实存在的 manifest assets
        let p = resolve_assets_path();
        assert!(p.ends_with("assets"));
        assert!(Path::new(&p).is_dir(), "解析结果应真实存在: {}", p);
    }

    #[test]
    fn log_filter_debug_info_release_warn_whitelist() {
        let f = log_filter();
        if cfg!(debug_assertions) {
            assert!(f.starts_with("info"), "debug 保持 info 全量: {}", f);
        } else {
            // release 默认 warn，排障模块白名单保留 info
            assert!(f.starts_with("warn"), "release 默认 warn: {}", f);
            assert!(f.contains("client_bevy::network=info"));
            assert!(f.contains("client_bevy::game::combat=info"));
        }
    }
}
