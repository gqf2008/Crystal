// ============================================================================
// Windows IME 位置控制
// ============================================================================
//
// 使用 Windows API 设置输入法候选框位置
//
// ============================================================================

use std::ffi::c_void;

// Windows API 类型定义
type HWND = *mut c_void;
type HIMC = *mut c_void;
type DWORD = u32;
type LONG = i32;
type BOOL = i32;

// IME 常量
const CFS_POINT: DWORD = 0x0002;

/// POINT 结构
#[repr(C)]
#[derive(Default)]
struct POINT {
    x: LONG,
    y: LONG,
}

/// RECT 结构
#[repr(C)]
#[derive(Default)]
struct RECT {
    left: LONG,
    top: LONG,
    right: LONG,
    bottom: LONG,
}

/// COMPOSITIONFORM 结构 - 用于设置 IME 候选框位置
#[repr(C)]
struct COMPOSITIONFORM {
    dw_style: DWORD,
    pt_current_pos: POINT,
    rc_area: RECT,
}

#[link(name = "user32")]
extern "system" {
    fn GetForegroundWindow() -> HWND;
}

#[link(name = "imm32")]
extern "system" {
    fn ImmGetContext(hwnd: HWND) -> HIMC;
    fn ImmReleaseContext(hwnd: HWND, himc: HIMC) -> BOOL;
    fn ImmSetCompositionWindow(himc: HIMC, lpCompForm: *const COMPOSITIONFORM) -> BOOL;
}

/// 设置 IME 候选框位置
/// 
/// # 参数
/// - `x`: 屏幕 X 坐标（像素）
/// - `y`: 屏幕 Y 坐标（像素）
/// 
/// # 示例
/// ```
/// set_ime_position(100, 500);
/// ```
pub fn set_ime_position(x: i32, y: i32) {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return;
        }

        let himc = ImmGetContext(hwnd);
        if himc.is_null() {
            return;
        }

        let comp_form = COMPOSITIONFORM {
            dw_style: CFS_POINT,
            pt_current_pos: POINT { x, y },
            rc_area: RECT::default(),
        };

        ImmSetCompositionWindow(himc, &comp_form);
        ImmReleaseContext(hwnd, himc);
    }
}
