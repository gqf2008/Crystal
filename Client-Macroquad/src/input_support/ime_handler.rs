// ============================================================================
// IME 事件处理器
// ============================================================================

use super::ImeInputState;
use macroquad::prelude::*;
use egui_macroquad::egui;

/// IME 事件处理器
pub struct ImeHandler {
    /// IME 状态
    pub state: ImeInputState,
    /// 是否启用 IME
    pub enabled: bool,
}

impl Default for ImeHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ImeHandler {
    pub fn new() -> Self {
        Self {
            state: ImeInputState::new(),
            enabled: true,
        }
    }

    /// 启用/禁用 IME
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.state.reset();
        }
    }

    /// 处理键盘输入事件
    pub fn handle_key_event(&mut self, key: KeyCode) -> bool {
        if !self.enabled {
            return false;
        }

        match key {
            KeyCode::Escape => {
                if self.state.is_composing {
                    self.state.reset();
                    return true;
                }
            }
            KeyCode::Enter => {
                if self.state.is_composing {
                    // 确认当前候选词或组合文本
                    return true;
                }
            }
            KeyCode::Space => {
                if self.state.is_composing && !self.state.candidates.is_empty() {
                    // 选择当前候选词
                    return true;
                }
            }
            KeyCode::Up => {
                if self.state.is_composing && !self.state.candidates.is_empty() {
                    self.state.select_previous_candidate();
                    return true;
                }
            }
            KeyCode::Down => {
                if self.state.is_composing && !self.state.candidates.is_empty() {
                    self.state.select_next_candidate();
                    return true;
                }
            }
            KeyCode::Tab => {
                if self.state.is_composing && !self.state.candidates.is_empty() {
                    self.state.select_next_candidate();
                    return true;
                }
            }
            _ => {}
        }

        false
    }

    /// 处理字符输入
    pub fn handle_char_input(&mut self, ch: char) -> Option<String> {
        if !self.enabled {
            return Some(ch.to_string());
        }

        // 检查是否为ASCII字符
        if ch.is_ascii() {
            if self.state.is_composing {
                // 在组合模式下添加到组合文本
                self.state.composition_text.push(ch);
                self.update_candidates();
                None
            } else {
                // 直接输入ASCII字符
                Some(ch.to_string())
            }
        } else {
            // 中文字符直接输入
            Some(ch.to_string())
        }
    }

    /// 处理文本输入事件（用于接收IME组合结果）
    pub fn handle_text_input(&mut self, text: &str) -> Option<String> {
        if !self.enabled {
            return Some(text.to_string());
        }

        if self.state.is_composing {
            // 如果正在组合，这可能是确认的结果
            self.state.reset();
            Some(text.to_string())
        } else {
            Some(text.to_string())
        }
    }

    /// 更新候选词（简单的拼音匹配示例）
    fn update_candidates(&mut self) {
        // 这里是一个简化的拼音匹配示例
        // 实际应用中应该使用专业的拼音输入法库
        let pinyin = &self.state.composition_text;
        
        let candidates = match pinyin.as_str() {
            "ni" => vec!["你".to_string(), "尼".to_string(), "泥".to_string()],
            "hao" => vec!["好".to_string(), "号".to_string(), "豪".to_string()],
            "shi" => vec!["是".to_string(), "十".to_string(), "石".to_string()],
            "wo" => vec!["我".to_string(), "握".to_string(), "卧".to_string()],
            "de" => vec!["的".to_string(), "得".to_string(), "德".to_string()],
            "zai" => vec!["在".to_string(), "再".to_string(), "载".to_string()],
            "you" => vec!["有".to_string(), "又".to_string(), "右".to_string()],
            "le" => vec!["了".to_string(), "乐".to_string(), "勒".to_string()],
            _ => {
                // 检查是否为常见拼音前缀
                if pinyin.len() >= 2 {
                    match &pinyin[..2] {
                        "zh" | "ch" | "sh" => vec![], // 等待更多输入
                        _ => vec![],
                    }
                } else {
                    vec![]
                }
            }
        };

        self.state.set_candidates(candidates);
    }

    /// 获取当前显示的文本（组合文本或正常文本）
    pub fn get_display_text(&self, base_text: &str) -> String {
        if self.state.is_composing {
            format!("{}{}", base_text, self.state.composition_text)
        } else {
            base_text.to_string()
        }
    }

    /// 渲染候选词窗口
    pub fn render_candidates(&self, ctx: &egui::Context, cursor_pos: egui::Pos2) {
        if !self.state.is_composing || self.state.candidates.is_empty() {
            return;
        }

        egui::Area::new(egui::Id::new("ime_candidates"))
            .fixed_pos(cursor_pos + egui::vec2(0.0, 20.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::popup(&ctx.style())
                    .inner_margin(4.0)
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.label(format!("拼音: {}", self.state.composition_text));
                            ui.separator();
                            
                            for (i, candidate) in self.state.candidates.iter().enumerate() {
                                let is_selected = self.state.selected_candidate == Some(i);
                                let response = ui.selectable_label(is_selected, 
                                    format!("{}. {}", i + 1, candidate));
                                
                                if response.clicked() {
                                    // 处理候选词点击
                                }
                            }
                        });
                    });
            });
    }
}