// ============================================================================
// TexturedMessageBox - 消息框组件
// ============================================================================

use egui_macroquad::egui;
use crate::resources::LibraryName;
use super::{TexturedButton, TexturedDialog, DialogType};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MessageBoxButtons {
    OK,
    OKCancel,
    YesNo,
    YesNoCancel,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MessageBoxResult {
    None,
    OK,
    Cancel,
    Yes,
    No,
}

pub struct TexturedMessageBox {
    dialog: TexturedDialog,
    message: String,
    buttons: MessageBoxButtons,
    
    // 按钮组件
    btn_ok: TexturedButton,
    btn_cancel: TexturedButton,
    btn_yes: TexturedButton,
    btn_no: TexturedButton,
}

impl TexturedMessageBox {
    pub fn new(message: impl Into<String>, buttons: MessageBoxButtons) -> Self {
        let message = message.into();
        
        // 基础对话框配置
        // MirMessageBox: Index = 360, Library = Libraries.Prguse
        let dialog = TexturedDialog::new("message_box", "系统提示")
            .with_type(DialogType::Modal) // 消息框通常是模态的
            .with_background(LibraryName::Prguse, 360)
            .with_rect(
                egui::pos2(0.0, 0.0), // 位置通常居中，draw时计算
                egui::vec2(460.0, 200.0) // 假设尺寸，根据背景图调整
            )
            .with_close_button(None); // 消息框通常没有右上角关闭按钮，或者由按钮控制

        // 初始化所有可能的按钮
        // OK: Title 200-202
        let btn_ok = TexturedButton::new()
            .with_library(LibraryName::Title)
            .with_states(200, Some(201), Some(202), None)
            .with_size(egui::vec2(80.0, 25.0)); // 估算尺寸

        // Cancel: Title 203-205
        let btn_cancel = TexturedButton::new()
            .with_library(LibraryName::Title)
            .with_states(203, Some(204), Some(205), None)
            .with_size(egui::vec2(80.0, 25.0));

        // Yes: Title 206-208
        let btn_yes = TexturedButton::new()
            .with_library(LibraryName::Title)
            .with_states(206, Some(207), Some(208), None)
            .with_size(egui::vec2(80.0, 25.0));

        // No: Title 210-212
        let btn_no = TexturedButton::new()
            .with_library(LibraryName::Title)
            .with_states(210, Some(211), Some(212), None)
            .with_size(egui::vec2(80.0, 25.0));

        Self {
            dialog,
            message,
            buttons,
            btn_ok,
            btn_cancel,
            btn_yes,
            btn_no,
        }
    }

    pub fn show(&mut self) {
        self.dialog.show();
        // 居中显示
        // 注意：这里无法直接获取屏幕尺寸，需要在draw时处理，或者传入ctx
    }

    pub fn hide(&mut self) {
        self.dialog.hide();
    }

    pub fn draw(&mut self, ctx: &egui::Context) -> MessageBoxResult {
        if !self.dialog.visible {
            return MessageBoxResult::None;
        }

        // 首次显示居中处理
        let screen_rect = ctx.screen_rect();
        if self.dialog.position == egui::pos2(0.0, 0.0) {
            self.dialog.position = screen_rect.center() - self.dialog.size / 2.0;
        }

        let mut result = MessageBoxResult::None;

        // 使用 TexturedDialog 的 draw_base 绘制背景和模态遮罩
        // 注意：我们不使用它的关闭逻辑，而是自己处理按钮
        self.dialog.draw_base(ctx);

        // 在对话框区域内绘制内容
        let content_area = egui::Area::new(egui::Id::new("msg_box_content"))
            .fixed_pos(self.dialog.position)
            .order(egui::Order::Foreground); // 确保在最上层

        content_area.show(ctx, |ui| {
            // 设置布局区域
            ui.allocate_ui_at_rect(
                egui::Rect::from_min_size(egui::pos2(0.0, 0.0), self.dialog.size), 
                |ui| {
                    // 1. 绘制消息文本
                    // 文本区域：(35, 35), Size(390, 110)
                    let text_rect = egui::Rect::from_min_size(
                        egui::pos2(35.0, 35.0),
                        egui::vec2(390.0, 110.0)
                    );
                    
                    ui.allocate_ui_at_rect(text_rect, |ui| {
                        ui.centered_and_justified(|ui| {
                            ui.label(egui::RichText::new(&self.message).color(egui::Color32::WHITE));
                        });
                    });

                    // 2. 绘制按钮
                    // 按钮位置参考 C# 代码
                    match self.buttons {
                        MessageBoxButtons::OK => {
                            // OK: (360, 157)
                            Self::draw_button(ui, &mut self.btn_ok, egui::pos2(360.0, 157.0), &mut result, MessageBoxResult::OK);
                        },
                        MessageBoxButtons::OKCancel => {
                            // OK: (260, 157)
                            Self::draw_button(ui, &mut self.btn_ok, egui::pos2(260.0, 157.0), &mut result, MessageBoxResult::OK);
                            // Cancel: (360, 157)
                            Self::draw_button(ui, &mut self.btn_cancel, egui::pos2(360.0, 157.0), &mut result, MessageBoxResult::Cancel);
                        },
                        MessageBoxButtons::YesNo => {
                            // Yes: (260, 157)
                            Self::draw_button(ui, &mut self.btn_yes, egui::pos2(260.0, 157.0), &mut result, MessageBoxResult::Yes);
                            // No: (360, 157)
                            Self::draw_button(ui, &mut self.btn_no, egui::pos2(360.0, 157.0), &mut result, MessageBoxResult::No);
                        },
                        MessageBoxButtons::YesNoCancel => {
                            // Yes: (160, 157)
                            Self::draw_button(ui, &mut self.btn_yes, egui::pos2(160.0, 157.0), &mut result, MessageBoxResult::Yes);
                            // No: (260, 157)
                            Self::draw_button(ui, &mut self.btn_no, egui::pos2(260.0, 157.0), &mut result, MessageBoxResult::No);
                            // Cancel: (360, 157)
                            Self::draw_button(ui, &mut self.btn_cancel, egui::pos2(360.0, 157.0), &mut result, MessageBoxResult::Cancel);
                        },
                        MessageBoxButtons::Cancel => {
                             // Cancel: (360, 157)
                             Self::draw_button(ui, &mut self.btn_cancel, egui::pos2(360.0, 157.0), &mut result, MessageBoxResult::Cancel);
                        }
                    }
                }
            );
        });

        if result != MessageBoxResult::None {
            self.hide();
        }

        result
    }

    fn draw_button(
        ui: &mut egui::Ui, 
        btn: &mut TexturedButton, 
        pos: egui::Pos2, 
        result: &mut MessageBoxResult, 
        target_result: MessageBoxResult
    ) {
        // 使用绝对定位放置按钮
        // 注意：这里的 pos 是相对于对话框左上角的
        let btn_size = btn.size.unwrap_or(egui::vec2(80.0, 25.0));
        let btn_rect = egui::Rect::from_min_size(pos, btn_size);
        
        let mut child_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(btn_rect)
                .layout(*ui.layout())
        );
        
        if btn.draw(&mut child_ui) {
            *result = target_result;
        }
    }
}
