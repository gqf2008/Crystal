// ============================================================================
// IME 输入支持模块
// ============================================================================
//
// 为 macroquad + egui 提供中文输入法支持
// 包括组合字符处理、候选词显示等功能
//
// 使用方式：
// ```rust
// let mut ime_state = ImeInputState::new();
// ime_state.update(ctx);
// ime_state.handle_text_input(&mut text_string);
// ```

pub mod ime_state;
pub mod ime_handler;
pub mod text_composition;

pub use ime_state::*;
pub use ime_handler::*;
pub use text_composition::*;

/// IME 输入状态
#[derive(Debug, Clone)]
pub struct ImeInputState {
    /// 当前组合文本（拼音）
    pub composition_text: String,
    /// 组合文本的光标位置
    pub composition_cursor: usize,
    /// 候选词列表
    pub candidates: Vec<String>,
    /// 当前选中的候选词索引
    pub selected_candidate: Option<usize>,
    /// 是否正在输入中文
    pub is_composing: bool,
}

impl Default for ImeInputState {
    fn default() -> Self {
        Self::new()
    }
}

impl ImeInputState {
    pub fn new() -> Self {
        Self {
            composition_text: String::new(),
            composition_cursor: 0,
            candidates: Vec::new(),
            selected_candidate: None,
            is_composing: false,
        }
    }

    /// 重置IME状态
    pub fn reset(&mut self) {
        self.composition_text.clear();
        self.composition_cursor = 0;
        self.candidates.clear();
        self.selected_candidate = None;
        self.is_composing = false;
    }

    /// 更新组合文本
    pub fn update_composition(&mut self, text: String, cursor_pos: usize) {
        self.composition_text = text;
        self.composition_cursor = cursor_pos;
        self.is_composing = !self.composition_text.is_empty();
    }

    /// 设置候选词
    pub fn set_candidates(&mut self, candidates: Vec<String>) {
        self.candidates = candidates;
        self.selected_candidate = if self.candidates.is_empty() { 
            None 
        } else { 
            Some(0) 
        };
    }

    /// 选择下一个候选词
    pub fn select_next_candidate(&mut self) {
        if let Some(current) = self.selected_candidate {
            self.selected_candidate = Some((current + 1) % self.candidates.len());
        }
    }

    /// 选择上一个候选词
    pub fn select_previous_candidate(&mut self) {
        if let Some(current) = self.selected_candidate {
            let len = self.candidates.len();
            self.selected_candidate = Some((current + len - 1) % len);
        }
    }

    /// 获取当前选中的候选词
    pub fn get_selected_candidate(&self) -> Option<&String> {
        self.selected_candidate
            .and_then(|idx| self.candidates.get(idx))
    }

    /// 确认当前候选词
    pub fn commit_candidate(&mut self) -> Option<String> {
        let result = self.get_selected_candidate().cloned();
        self.reset();
        result
    }
}