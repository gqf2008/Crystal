// ============================================================================
// 文本组合支持
// ============================================================================

use super::ImeInputState;

/// 文本组合状态
pub struct TextComposition {
    /// 原始文本
    pub text: String,
    /// 光标位置
    pub cursor_pos: usize,
    /// IME 状态
    pub ime_state: ImeInputState,
}

impl TextComposition {
    pub fn new(text: String) -> Self {
        Self {
            cursor_pos: text.len(),
            text,
            ime_state: ImeInputState::new(),
        }
    }

    /// 插入文本到光标位置
    pub fn insert_text(&mut self, text: &str) {
        self.text.insert_str(self.cursor_pos, text);
        self.cursor_pos += text.len();
    }

    /// 删除光标前的一个字符
    pub fn backspace(&mut self) -> bool {
        if self.ime_state.is_composing {
            // 在组合模式下，删除组合文本
            if !self.ime_state.composition_text.is_empty() {
                self.ime_state.composition_text.pop();
                return true;
            } else {
                self.ime_state.reset();
            }
        }

        if self.cursor_pos > 0 {
            // 找到前一个字符的边界
            let mut char_boundary = self.cursor_pos - 1;
            while char_boundary > 0 && !self.text.is_char_boundary(char_boundary) {
                char_boundary -= 1;
            }
            
            self.text.drain(char_boundary..self.cursor_pos);
            self.cursor_pos = char_boundary;
            true
        } else {
            false
        }
    }

    /// 删除光标后的一个字符
    pub fn delete(&mut self) -> bool {
        if self.cursor_pos < self.text.len() {
            // 找到下一个字符的边界
            let mut char_boundary = self.cursor_pos + 1;
            while char_boundary < self.text.len() && !self.text.is_char_boundary(char_boundary) {
                char_boundary += 1;
            }
            
            self.text.drain(self.cursor_pos..char_boundary);
            true
        } else {
            false
        }
    }

    /// 移动光标
    pub fn move_cursor(&mut self, delta: isize) {
        let new_pos = if delta < 0 {
            self.cursor_pos.saturating_sub((-delta) as usize)
        } else {
            (self.cursor_pos + delta as usize).min(self.text.len())
        };

        // 确保光标在字符边界上
        self.cursor_pos = self.find_char_boundary(new_pos);
    }

    /// 找到最近的字符边界
    fn find_char_boundary(&self, pos: usize) -> usize {
        let pos = pos.min(self.text.len());
        
        // 向前找到字符边界
        let mut boundary = pos;
        while boundary > 0 && !self.text.is_char_boundary(boundary) {
            boundary -= 1;
        }
        boundary
    }

    /// 获取显示文本（包含组合文本）
    pub fn get_display_text(&self) -> String {
        if self.ime_state.is_composing {
            let mut result = self.text.clone();
            result.insert_str(self.cursor_pos, &self.ime_state.composition_text);
            result
        } else {
            self.text.clone()
        }
    }

    /// 获取光标的显示位置
    pub fn get_display_cursor_pos(&self) -> usize {
        if self.ime_state.is_composing {
            self.cursor_pos + self.ime_state.composition_cursor
        } else {
            self.cursor_pos
        }
    }

    /// 确认组合文本
    pub fn commit_composition(&mut self) {
        if let Some(candidate) = self.ime_state.get_selected_candidate() {
            let text = candidate.clone();
            self.insert_text(&text);
        } else if !self.ime_state.composition_text.is_empty() {
            let text = self.ime_state.composition_text.clone();
            self.insert_text(&text);
        }
        self.ime_state.reset();
    }

    /// 取消组合
    pub fn cancel_composition(&mut self) {
        self.ime_state.reset();
    }
}