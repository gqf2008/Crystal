//! Windows IME 输入法支持
//! 
//! 提供设置 IME 候选窗口位置的功能

#[cfg(target_os = "windows")]
mod windows_ime {
    use std::ffi::c_void;
    
    #[repr(C)]
    struct POINT {
        x: i32,
        y: i32,
    }
    
    #[repr(C)]
    struct RECT {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    
    #[repr(C)]
    struct COMPOSITIONFORM {
        dw_style: u32,
        pt_current_pos: POINT,
        rc_area: RECT,
    }
    
    #[repr(C)]
    struct CANDIDATEFORM {
        dw_index: u32,
        dw_style: u32,
        pt_current_pos: POINT,
        rc_area: RECT,
    }
    
    const CFS_POINT: u32 = 0x0002;
    const CFS_CANDIDATEPOS: u32 = 0x0040;
    
    #[link(name = "user32")]
    extern "system" {
        fn GetForegroundWindow() -> *mut c_void;
    }
    
    #[link(name = "imm32")]
    extern "system" {
        fn ImmGetContext(hwnd: *mut c_void) -> *mut c_void;
        fn ImmReleaseContext(hwnd: *mut c_void, himc: *mut c_void) -> i32;
        fn ImmSetCompositionWindow(himc: *mut c_void, lpCompForm: *const COMPOSITIONFORM) -> i32;
        fn ImmSetCandidateWindow(himc: *mut c_void, lpCandidate: *const CANDIDATEFORM) -> i32;
    }
    
    /// 设置 IME 候选窗口位置
    /// 
    /// # 参数
    /// - `x`: 屏幕 X 坐标（像素）
    /// - `y`: 屏幕 Y 坐标（像素）
    pub fn set_ime_position(x: f32, y: f32) {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_null() {
                return;
            }
            
            let himc = ImmGetContext(hwnd);
            if himc.is_null() {
                return;
            }
            
            let x = x as i32;
            let y = y as i32;
            
            // 设置组合窗口位置（显示正在输入的拼音/五笔码）
            let comp_form = COMPOSITIONFORM {
                dw_style: CFS_POINT,
                pt_current_pos: POINT { x, y },
                rc_area: RECT { left: 0, top: 0, right: 0, bottom: 0 },
            };
            ImmSetCompositionWindow(himc, &comp_form);
            
            // 设置候选窗口位置（显示候选词列表）
            let candidate_form = CANDIDATEFORM {
                dw_index: 0,
                dw_style: CFS_CANDIDATEPOS,
                pt_current_pos: POINT { x, y: y + 20 }, // 稍微往下偏移
                rc_area: RECT { left: 0, top: 0, right: 0, bottom: 0 },
            };
            ImmSetCandidateWindow(himc, &candidate_form);
            
            ImmReleaseContext(hwnd, himc);
        }
    }
    
    /// 设置 IME 候选窗口位置（带输入框高度）
    /// 
    /// # 参数
    /// - `x`: 输入框 X 坐标
    /// - `y`: 输入框 Y 坐标  
    /// - `height`: 输入框高度，候选窗口会显示在输入框下方
    pub fn set_ime_position_with_height(x: f32, y: f32, height: f32) {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_null() {
                return;
            }
            
            let himc = ImmGetContext(hwnd);
            if himc.is_null() {
                return;
            }
            
            let x = x as i32;
            let y = y as i32;
            let bottom = (y as f32 + height) as i32;
            
            // 设置组合窗口位置
            let comp_form = COMPOSITIONFORM {
                dw_style: CFS_POINT,
                pt_current_pos: POINT { x, y },
                rc_area: RECT { left: 0, top: 0, right: 0, bottom: 0 },
            };
            ImmSetCompositionWindow(himc, &comp_form);
            
            // 设置候选窗口位置（在输入框下方）
            let candidate_form = CANDIDATEFORM {
                dw_index: 0,
                dw_style: CFS_CANDIDATEPOS,
                pt_current_pos: POINT { x, y: bottom + 2 },
                rc_area: RECT { left: 0, top: 0, right: 0, bottom: 0 },
            };
            ImmSetCandidateWindow(himc, &candidate_form);
            
            ImmReleaseContext(hwnd, himc);
        }
    }
}

#[cfg(target_os = "windows")]
pub use windows_ime::*;

// 非 Windows 平台的空实现
#[cfg(not(target_os = "windows"))]
pub fn set_ime_position(_x: f32, _y: f32) {}

#[cfg(not(target_os = "windows"))]
pub fn set_ime_position_with_height(_x: f32, _y: f32, _height: f32) {}
