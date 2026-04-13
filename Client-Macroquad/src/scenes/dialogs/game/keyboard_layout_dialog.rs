// ============================================================================
// KeyboardLayoutDialogHybrid - 键位设置对话框
// ============================================================================
//
// C# 参考: Client/MIRScenes/Dialogs/KeyboardLayoutDialog.cs (~438 行)
// - 键位配置编辑器
// - 支持按键重新绑定
// - 严格/宽松模式切换
//
// ============================================================================

use macroquad::prelude::*;
use crate::ui::text_renderer::draw_text_cn;
use macroquad::input::KeyCode;

/// 键位绑定配置
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyBinding {
    pub action_name: String,
    pub key: KeyCode,
    pub group: String,
}

/// 键位设置对话框
pub struct KeyboardLayoutDialogHybrid {
    pub visible: bool,
    pub bindings: Vec<KeyBinding>,
    /// 当前正在重新绑定的条目索引
    rebinding_index: Option<usize>,
    scroll_offset: f32,
    strict_mode: bool,
}

impl Default for KeyboardLayoutDialogHybrid {
    fn default() -> Self {
        Self {
            visible: false,
            bindings: Self::default_bindings(),
            rebinding_index: None,
            scroll_offset: 0.0,
            strict_mode: false,
        }
    }
}

impl KeyboardLayoutDialogHybrid {
    pub fn new() -> Self {
        Self::default()
    }

    fn default_bindings() -> Vec<KeyBinding> {
        vec![
            KeyBinding { action_name: "移动".to_string(), key: KeyCode::W, group: "移动".to_string() },
            KeyBinding { action_name: "移动".to_string(), key: KeyCode::A, group: "移动".to_string() },
            KeyBinding { action_name: "移动".to_string(), key: KeyCode::S, group: "移动".to_string() },
            KeyBinding { action_name: "移动".to_string(), key: KeyCode::D, group: "移动".to_string() },
            KeyBinding { action_name: "攻击".to_string(), key: KeyCode::Q, group: "战斗".to_string() },
            KeyBinding { action_name: "拾取".to_string(), key: KeyCode::Space, group: "交互".to_string() },
            KeyBinding { action_name: "背包".to_string(), key: KeyCode::B, group: "界面".to_string() },
            KeyBinding { action_name: "角色".to_string(), key: KeyCode::C, group: "界面".to_string() },
            KeyBinding { action_name: "技能".to_string(), key: KeyCode::K, group: "界面".to_string() },
            KeyBinding { action_name: "行会".to_string(), key: KeyCode::G, group: "界面".to_string() },
            KeyBinding { action_name: "技能栏1-8".to_string(), key: KeyCode::F1, group: "技能".to_string() },
            KeyBinding { action_name: "切换攻击模式".to_string(), key: KeyCode::LeftAlt, group: "战斗".to_string() },
            KeyBinding { action_name: "聊天".to_string(), key: KeyCode::Enter, group: "交互".to_string() },
        ]
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn close(&mut self) {
        self.rebinding_index = None;
        self.visible = false;
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// 是否正在等待用户按键
    pub fn is_rebinding(&self) -> bool {
        self.rebinding_index.is_some()
    }

    /// 按键码显示名称
    fn key_name(key: &KeyCode) -> String {
        match key {
            KeyCode::Space => "空格".to_string(),
            KeyCode::LeftAlt => "左Alt".to_string(),
            KeyCode::RightAlt => "右Alt".to_string(),
            KeyCode::LeftControl => "左Ctrl".to_string(),
            KeyCode::RightControl => "右Ctrl".to_string(),
            KeyCode::LeftShift => "左Shift".to_string(),
            KeyCode::RightShift => "右Shift".to_string(),
            _ => format!("{:?}", key),
        }
    }

    /// 处理输入并绘制
    pub fn draw(&mut self, screen_w: f32, screen_h: f32, mouse_pos: Vec2,
                mouse_wheel: f32, left_clicked: bool, any_key_pressed: Option<KeyCode>) -> bool {
        if !self.visible {
            return false;
        }

        let padding = 15.0;
        let title_h = 30.0;
        let item_h = 22.0;
        let btn_h = 30.0;
        let dialog_w = 350.0;
        let dialog_h = 400.0;
        let dialog_x = (screen_w - dialog_w) / 2.0;
        let dialog_y = (screen_h - dialog_h) / 2.0;

        let mouse_over = mouse_pos.x >= dialog_x && mouse_pos.x <= dialog_x + dialog_w
            && mouse_pos.y >= dialog_y && mouse_pos.y <= dialog_y + dialog_h;

        if mouse_over && mouse_wheel != 0.0 {
            self.scroll_offset = (self.scroll_offset - mouse_wheel * 15.0).max(0.0);
        }

        // 处理重新绑定
        if let Some(idx) = self.rebinding_index {
            if let Some(key) = any_key_pressed {
                self.bindings[idx].key = key;
                self.rebinding_index = None;
            }
        }

        // 背景
        draw_rectangle(dialog_x, dialog_y, dialog_w, dialog_h, Color::from_rgba(25, 25, 35, 240));

        // 标题
        draw_text_cn("键位设置", dialog_x + 15.0, dialog_y + 10.0, 16.0,
            Color::from_rgba(255, 220, 100, 255));

        // 严格/宽松模式切换
        let mode_text = if self.strict_mode { "严格模式" } else { "宽松模式" };
        draw_text_cn(&format!("模式: {}", mode_text), dialog_x + dialog_w - 120.0, dialog_y + 10.0, 12.0, WHITE);

        // 键位列表
        let content_y = dialog_y + title_h + padding;
        let content_h = dialog_h - title_h - btn_h - padding * 3.0;

        for (i, binding) in self.bindings.iter().enumerate() {
            let y = content_y + i as f32 * item_h - self.scroll_offset;
            if y < content_y || y + item_h > content_y + content_h {
                continue;
            }

            // 检查是否是重新绑定的条目
            let is_rebinding = self.rebinding_index == Some(i);
            let key_display = if is_rebinding {
                "...".to_string()
            } else {
                Self::key_name(&binding.key)
            };

            let bg_color = if is_rebinding {
                Color::from_rgba(80, 60, 20, 200)
            } else {
                Color::from_rgba(30, 30, 40, 150)
            };
            draw_rectangle(dialog_x + 10.0, y, dialog_w - 20.0, item_h, bg_color);

            draw_text_cn(&binding.action_name, dialog_x + 15.0, y + 4.0, 12.0,
                Color::from_rgba(200, 200, 200, 255));

            let key_x = dialog_x + dialog_w - 100.0;
            draw_text_cn(&key_display, key_x, y + 4.0, 12.0,
                if is_rebinding { Color::from_rgba(255, 100, 50, 255) }
                else { Color::from_rgba(100, 200, 255, 255) });

            // 点击重新绑定
            if left_clicked && mouse_pos.x >= key_x && mouse_pos.x <= key_x + 80.0
                && mouse_pos.y >= y && mouse_pos.y <= y + item_h {
                self.rebinding_index = Some(i);
            }
        }

        // 底部按钮
        let btn_y = dialog_y + dialog_h - btn_h - padding;
        let btn_w = 80.0;

        // 重置按钮
        let reset_x = dialog_x + 20.0;
        let mouse_over_reset = mouse_pos.x >= reset_x && mouse_pos.x <= reset_x + btn_w
            && mouse_pos.y >= btn_y && mouse_pos.y <= btn_y + btn_h;
        draw_rectangle(reset_x, btn_y, btn_w, btn_h,
            if mouse_over_reset { Color::from_rgba(100, 80, 40, 255) } else { Color::from_rgba(70, 60, 30, 255) });
        draw_text_cn("重置", reset_x + 20.0, btn_y + 7.0, 14.0, WHITE);

        if left_clicked && mouse_over_reset {
            self.bindings = Self::default_bindings();
        }

        // 关闭按钮
        let close_x = dialog_x + dialog_w - btn_w - 20.0;
        let mouse_over_close = mouse_pos.x >= close_x && mouse_pos.x <= close_x + btn_w
            && mouse_pos.y >= btn_y && mouse_pos.y <= btn_y + btn_h;
        draw_rectangle(close_x, btn_y, btn_w, btn_h,
            if mouse_over_close { Color::from_rgba(150, 50, 50, 255) } else { Color::from_rgba(100, 30, 30, 255) });
        draw_text_cn("关闭", close_x + 20.0, btn_y + 7.0, 14.0, WHITE);

        if left_clicked && mouse_over_close {
            self.close();
        }

        mouse_over
    }
}
