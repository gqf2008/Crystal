// ============================================================================
// IME 状态管理
// ============================================================================

use crate::input_support::ImeInputState;

impl ImeInputState {
    /// 处理 Windows IME 消息（暂时禁用，需要winapi依赖）
    #[cfg(target_os = "windows")]
    pub fn handle_windows_ime(&mut self, _msg: u32, _wparam: usize, _lparam: isize) -> bool {
        // TODO: 需要添加 winapi 依赖才能实现
        // 暂时返回 false，表示未处理
        false
    }

    /// 处理 macOS IME 事件
    #[cfg(target_os = "macos")]
    pub fn handle_macos_ime(&mut self, text: &str, marked_range: std::ops::Range<usize>) -> bool {
        if marked_range.is_empty() {
            // 确认输入
            self.reset();
            false
        } else {
            // 更新组合文本
            self.composition_text = text[marked_range].to_string();
            self.is_composing = true;
            true
        }
    }

    /// 处理 Linux IME 事件
    #[cfg(target_os = "linux")]
    pub fn handle_linux_ime(&mut self, event_type: &str, text: &str) -> bool {
        match event_type {
            "preedit_start" => {
                self.is_composing = true;
                true
            }
            "preedit_changed" => {
                self.composition_text = text.to_string();
                self.is_composing = true;
                true
            }
            "preedit_end" => {
                self.reset();
                true
            }
            "commit" => {
                self.reset();
                false
            }
            _ => false,
        }
    }

    /// 通用的文本组合处理
    pub fn handle_composition_event(&mut self, 
        composition_text: Option<String>, 
        committed_text: Option<String>) -> Option<String> {
        
        if let Some(text) = committed_text {
            // 有确认的文本
            self.reset();
            return Some(text);
        }

        if let Some(comp_text) = composition_text {
            if comp_text.is_empty() {
                self.reset();
            } else {
                self.composition_text = comp_text;
                self.is_composing = true;
            }
        }

        None
    }
}