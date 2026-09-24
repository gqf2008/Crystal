// ============================================================================
// 渲染错误策略：把「表面配置类」的 Validation 错误降级为可恢复（不退进程）
// ============================================================================
// 缺陷（owner 实机复现）：「窗口最小化 → 客户端 7ms 内退进程」。链路：
//   winit `WM_SIZE(SIZE_MINIMIZED)` → `Resized(0,0)`
//   → bevy `extract_windows` 把尺寸夹到 ≥1 → `prepare_windows` 用新尺寸调
//     `configure_surface`
//   → wgpu-hal dx12 `ResizeBuffers` 撞上「上一帧 swapchain 后备缓冲仍有存活引用」
//     （DXGI 调试层原话：`Swapchain cannot be resized unless all outstanding buffer
//     references have been released. [ MISCELLANEOUS ERROR #19 ]`）
//   → `SurfaceError::Other("window is in use")` → wgpu 归类为 Validation 错误
//   → bevy **默认** `RenderErrorHandler`（`bevy_render/src/error_handler.rs`）
//     对**任何** RenderError 一律 `AppExit::error()`。
//
// 客户端此前没有覆盖该策略，于是一次「可恢复的表面配置失败」被升级成整进程退出
// （且不产生 WER/崩溃转储）。本模块覆盖它：
//   * 表面配置类错误（`Invalid surface` / `window is in use`）→ **不退进程**：
//       - 窗口处于退化尺寸（最小化 → 主世界 physical 尺寸为 0）时 `StopRendering`
//         （暂停渲染图，避免对着 1×1/0 尺寸的表面反复 present 与刷屏；
//         主世界 Update 照常跑，网络/游戏逻辑不受影响），
//       - 其余情况 `Ignore`（bevy 文档：忽略本次并继续渲染）；
//       两者都会在下一次轮询时重新判定，窗口恢复后自动回到 `Ready`。
//   * 其它 RenderError（OOM / DeviceLost / 无关 Validation）**保持** bevy 默认的
//     退进程行为——不把真问题一起吞掉。
//
// 证据与定性见 `tools/acceptance/l5t_minimize_survives.ps1` 的运行输出（该夹具
// 的红检即本修复前的 master：同一二进制最小化后 `exit 1`）。

use bevy::log::{error, info, warn};
use bevy::prelude::*;
use bevy::render::error_handler::{ErrorType, RenderError, RenderErrorHandler, RenderErrorPolicy};

/// 阳性对照开关：设了它就不安装本策略（保留 bevy 默认「任何 RenderError 退进程」）。
/// 夹具 `tools/acceptance/l5t_minimize_survives.ps1 -NoHandler` 用它证明
/// 「红 → 绿」确实是本策略带来的，而不是别的改动顺带修好了。
pub const DISABLE_ENV: &str = "CRYSTAL_NO_RENDER_ERROR_HANDLER";

/// 表面配置类错误在 `RenderError::description` 里的签名。
///
/// wgpu-core 的 `ConfigureSurfaceError` 只有 `Invalid surface` 一种文案（`Outdated` /
/// `Lost` / `Occluded` / `Timeout` / `ResizeBuffers` 失败全都收敛到它）；`window is in use`
/// 是 dx12 `ResizeBuffers` 失败时 hal 侧的原话，会先被 `wgpu-core` 用 `log::error!` 打印，
/// 这里一并认账以便上游文案变化时仍能命中。
const SURFACE_ERROR_MARKERS: &[&str] = &[
    "Invalid surface",
    "window is in use",
    "surface configuration failed",
];

/// `classify` 的结论：只分「表面配置类（可恢复）」与「其余（保持致命）」两类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorDisposition {
    RecoverableSurface,
    Fatal,
}

/// 判定一条 `RenderError` 是否属于「表面配置类」。
///
/// 纯函数（只读 `ty` 与 `description`），便于离线单测钉住白名单：新增降级类型必须先在这里
/// 显式加一句话，而不是靠宽松的「任何 Validation 都忽略」。
pub fn classify(error: &RenderError) -> ErrorDisposition {
    if error.ty == ErrorType::Validation
        && SURFACE_ERROR_MARKERS
            .iter()
            .any(|marker| error.description.contains(marker))
    {
        ErrorDisposition::RecoverableSurface
    } else {
        ErrorDisposition::Fatal
    }
}

/// 记录「因窗口最小化而暂停渲染」的当前状态，避免每帧刷同一条日志。
#[derive(Resource, Default)]
struct RenderSuspendState {
    /// 已经就「暂停渲染」提示过一次（窗口恢复时复位）。
    notified: bool,
}

/// 安装本策略（必须在 `DefaultPlugins` 之前调用：`RenderPlugin::build` 里是
/// `init_resource::<RenderErrorHandler>`，先插入者胜出）。
pub fn install(app: &mut App) {
    if std::env::var_os(DISABLE_ENV).is_some() {
        warn!(
            "[render-error] {DISABLE_ENV} 已设置：保留 bevy 默认 RenderErrorHandler\
             （阳性对照用：最小化窗口会退进程）"
        );
        return;
    }
    app.insert_resource(RenderErrorHandler(handle_render_error));
}

/// 本客户端对 wgpu 渲染错误的响应策略（签名固定为 bevy 的 `RenderErrorHandler`）。
fn handle_render_error(
    error: &RenderError,
    main_world: &mut World,
    _render_world: &mut World,
) -> RenderErrorPolicy {
    match classify(error) {
        ErrorDisposition::Fatal => {
            // 与 bevy 默认策略保持一致（含日志原文，便于既有夹具继续匹配「Quitting ...」）
            error!("Quitting the application due to {:?} RenderError", error.ty);
            main_world.write_message(AppExit::error());
            RenderErrorPolicy::StopRendering
        }
        ErrorDisposition::RecoverableSurface => {
            let degenerate = window_is_degenerate(main_world);
            let mut state = main_world.get_resource_or_insert_with(RenderSuspendState::default);
            if degenerate {
                if !state.notified {
                    state.notified = true;
                    info!(
                        "[render-error] 表面配置失败且窗口已最小化：暂停渲染（不退进程），\
                         窗口恢复后自动继续"
                    );
                }
                // 保持 `Errored`：渲染图停摆，但主世界逻辑照跑，且本策略每帧会被重新轮询。
                RenderErrorPolicy::StopRendering
            } else {
                if state.notified {
                    state.notified = false;
                    info!("[render-error] 窗口已恢复：渲染继续");
                } else {
                    warn!(
                        "[render-error] 表面配置失败已降级为可恢复（忽略本次）: {}",
                        error.description
                    );
                }
                RenderErrorPolicy::Ignore
            }
        }
    }
}

/// 主世界是否存在「退化尺寸」窗口（最小化 → winit 上报的 physical 尺寸为 0）。
fn window_is_degenerate(world: &mut World) -> bool {
    let mut windows = world.query::<&Window>();
    windows.iter(world).any(|window| {
        window.resolution.physical_width() == 0 || window.resolution.physical_height() == 0
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_error(ty: ErrorType, description: &str) -> RenderError {
        RenderError {
            ty,
            description: description.to_string(),
            source: None,
        }
    }

    #[test]
    fn minimize_signature_is_recoverable() {
        // 整改前的 master 就是这两条签名把进程带走的（见 l5t 夹具与 resize_defect 报告）
        assert_eq!(
            classify(&render_error(ErrorType::Validation, "Invalid surface")),
            ErrorDisposition::RecoverableSurface
        );
        assert_eq!(
            classify(&render_error(
                ErrorType::Validation,
                "surface configuration failed: window is in use"
            )),
            ErrorDisposition::RecoverableSurface
        );
    }

    #[test]
    fn unrelated_validation_errors_stay_fatal() {
        // 白名单必须窄：别的 Validation 错误仍按 bevy 默认退进程，不能被顺手吞掉
        assert_eq!(
            classify(&render_error(
                ErrorType::Validation,
                "Buffer binding size is too large"
            )),
            ErrorDisposition::Fatal
        );
    }

    #[test]
    fn non_validation_error_types_stay_fatal() {
        // 类型也对：OOM / DeviceLost / Internal 即使文案里带 surface 字样也不降级
        assert_eq!(
            classify(&render_error(ErrorType::OutOfMemory, "")),
            ErrorDisposition::Fatal
        );
        assert_eq!(
            classify(&render_error(ErrorType::DeviceLost, "Invalid surface")),
            ErrorDisposition::Fatal
        );
        assert_eq!(
            classify(&render_error(ErrorType::Internal, "Invalid surface")),
            ErrorDisposition::Fatal
        );
    }
}
