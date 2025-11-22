// ============================================================================
// 删除角色对话框组件
// ============================================================================
// 
// 【功能说明】
// 本组件实现角色删除确认流程：
// 1. 第一步：显示确认对话框 "Are you sure you want to Delete the character {name}?"
// 2. 第二步：要求输入角色名称进行二次确认
// 3. 发送删除请求到服务器
//
// 【对话框流程】
// Step 1: 确认对话框 (YesNo)
//   - Yes → 进入 Step 2
//   - No → 取消
// 
// Step 2: 输入框对话框 (InputBox)
//   - 输入正确的角色名 → 发送删除请求
//   - 输入错误 → 显示错误消息
//
// ============================================================================

use super::{Dialog, MessageBox, MessageBoxButtons};
use egui_macroquad::egui;

/// 删除角色对话框事件
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeleteCharacterEvent {
    None,
    Delete(i32), // 角色索引
    Cancel,
}

/// 删除角色对话框（两步确认流程）
pub struct DeleteCharacterDialog {
    // 角色信息
    pub character_name: String,
    pub character_index: i32,
    
    // 对话框步骤
    step: DeleteStep,
    
    // 输入框状态
    input_text: String,
    
    // 子对话框
    confirm_box: MessageBox,
    input_box: MessageBox,
    error_box: MessageBox,
    
    // 事件
    last_event: DeleteCharacterEvent,
}

/// 删除流程步骤
#[derive(Debug, Clone, Copy, PartialEq)]
enum DeleteStep {
    Confirm,     // 第一步：确认删除
    InputName,   // 第二步：输入角色名
    Error,       // 显示错误
}

impl DeleteCharacterDialog {
    pub fn new() -> Self {
        Self {
            character_name: String::new(),
            character_index: -1,
            step: DeleteStep::Confirm,
            input_text: String::new(),
            confirm_box: MessageBox::new_with_id("", "", MessageBoxButtons::YesNo, "delete_confirm"),
            input_box: MessageBox::new_with_id("", "", MessageBoxButtons::OkCancel, "delete_input"),
            error_box: MessageBox::new_with_id("", "", MessageBoxButtons::Ok, "delete_error"),
            last_event: DeleteCharacterEvent::None,
        }
    }
    
    /// 开始删除流程
    pub fn start_delete(&mut self, character_name: String, character_index: i32) {
        self.character_name = character_name.clone();
        self.character_index = character_index;
        self.step = DeleteStep::Confirm;
        self.input_text.clear();
        
        self.confirm_box.title = "确认删除".to_string();
        self.confirm_box.text = format!("确定要删除角色 {} 吗？", character_name);
    }
    
    /// 获取并清除事件
    pub fn take_event(&mut self) -> DeleteCharacterEvent {
        let event = self.last_event;
        self.last_event = DeleteCharacterEvent::None;
        event
    }
    
    /// 重置对话框
    pub fn reset(&mut self) {
        self.character_name.clear();
        self.character_index = -1;
        self.step = DeleteStep::Confirm;
        self.input_text.clear();
        self.last_event = DeleteCharacterEvent::None;
    }
    
    /// 绘制输入框按钮
    fn draw_input_button(
        &self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        normal_idx: usize,
        hover_idx: usize,
        pressed_idx: usize,
        pos: egui::Pos2,
    ) -> bool {
        use crate::resources::LibraryName;
        
        if let Some(info) = LibraryName::Title.get_egui_texture(ctx, normal_idx) {
            if let Some(texture) = info.egui_texture {
                let size = egui::vec2(texture.size()[0] as f32, texture.size()[1] as f32);
                let rect = egui::Rect::from_min_size(pos, size);
                let response = ui.interact(rect, egui::Id::new(format!("input_btn_{}", normal_idx)), egui::Sense::click());
                
                let texture_idx = if response.is_pointer_button_down_on() {
                    pressed_idx
                } else if response.hovered() {
                    hover_idx
                } else {
                    normal_idx
                };
                
                if let Some(btn_info) = LibraryName::Title.get_egui_texture(ctx, texture_idx) {
                    if let Some(btn_texture) = btn_info.egui_texture {
                        ui.painter().image(
                            btn_texture.id(),
                            rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }
                
                return response.clicked();
            }
        }
        false
    }
}

impl Dialog for DeleteCharacterDialog {
    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        if !*open {
            return;
        }
        
        match self.step {
            DeleteStep::Confirm => {
                // 第一步：确认删除
                let mut confirm_open = true;
                self.confirm_box.show(ctx, &mut confirm_open);
                
                if !confirm_open {
                    // 对话框关闭，检查结果
                    if let Some(result) = self.confirm_box.result() {
                        match result {
                            crate::scenes::dialogs::MessageBoxResult::Yes => {
                                // 用户点击 Yes，进入第二步
                                self.step = DeleteStep::InputName;
                                self.input_text.clear();
                                self.input_box.title = "验证删除".to_string();
                                self.input_box.text = format!("请输入角色名称以确认删除：");
                            }
                            crate::scenes::dialogs::MessageBoxResult::No |
                            crate::scenes::dialogs::MessageBoxResult::Cancel => {
                                // 用户取消
                                self.last_event = DeleteCharacterEvent::Cancel;
                                *open = false;
                            }
                            _ => {}
                        }
                    }
                }
            }
            
            DeleteStep::InputName => {
                // 第二步：输入角色名称（使用原工程 MirInputBox 布局）
                use crate::resources::LibraryName;
                
                // 获取背景纹理 Prguse[660]
                if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 660) {
                    if let Some(bg_texture) = info.egui_texture {
                        let (w, h) = LibraryName::Prguse.get_size(660).unwrap_or((286, 165));
                        
                        // 使用 Area 允许拖动
                        egui::Area::new(egui::Id::new("delete_input_area"))
                            .default_pos(egui::pos2(
                                (macroquad::prelude::screen_width() / macroquad::prelude::screen_dpi_scale() - w as f32) / 2.0,
                                (macroquad::prelude::screen_height() / macroquad::prelude::screen_dpi_scale() - h as f32) / 2.0
                            ))
                            .movable(true)  // 允许拖动
                            .interactable(true)
                            .order(egui::Order::Foreground)
                            .show(ctx, |ui| {
                                // 分配对话框空间
                                let rect = ui.allocate_rect(
                                    egui::Rect::from_min_size(ui.cursor().min, egui::vec2(w as f32, h as f32)),
                                    egui::Sense::hover()
                                ).rect;
                                
                                // 绘制背景纹理
                                ui.painter().image(
                                    bg_texture.id(),
                                    rect,
                                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                    egui::Color32::WHITE,
                                );
                                
                                // 提示文字 at (25, 25), Size(235, 40)
                                let text_rect = egui::Rect::from_min_size(
                                    egui::pos2(rect.min.x + 25.0, rect.min.y + 25.0),
                                    egui::vec2(235.0, 40.0)
                                );
                                ui.painter().text(
                                    text_rect.left_top(),
                                    egui::Align2::LEFT_TOP,
                                    &self.input_box.text,
                                    egui::FontId::proportional(14.0),
                                    egui::Color32::WHITE,
                                );
                                
                                // 输入框 at (23, 86), Size(240, 19) - 无边框样式
                                let input_rect = egui::Rect::from_min_size(
                                    egui::pos2(rect.min.x + 23.0, rect.min.y + 86.0),
                                    egui::vec2(240.0, 19.0)
                                );
                                
                                // 绘制输入框背景（黑色）
                                ui.painter().rect_filled(input_rect, 0.0, egui::Color32::BLACK);
                                
                                // 使用 allocate_ui_at_rect 在指定位置创建输入框
                                let ui_builder = egui::UiBuilder::new().max_rect(input_rect);
                                ui.allocate_new_ui(ui_builder, |ui| {
                                    ui.set_clip_rect(input_rect);
                                    let input_response = ui.add(
                                        egui::TextEdit::singleline(&mut self.input_text)
                                            .desired_width(235.0)
                                            .frame(false)  // 移除边框
                                            .margin(egui::vec2(2.0, 2.0))
                                    );
                                    
                                    // 自动聚焦
                                    if ctx.memory(|mem| mem.focused().is_none()) {
                                        input_response.request_focus();
                                    }
                                });
                                
                                // 处理 Enter 键
                                let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                                let esc_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));
                                
                                // OK 按钮 Title[200-202] at (60, 123)
                                let ok_clicked = self.draw_input_button(
                                    ui, ctx, 200, 201, 202,
                                    egui::pos2(rect.min.x + 60.0, rect.min.y + 123.0)
                                );
                                
                                // Cancel 按钮 Title[203-205] at (160, 123)
                                let cancel_clicked = self.draw_input_button(
                                    ui, ctx, 203, 204, 205,
                                    egui::pos2(rect.min.x + 160.0, rect.min.y + 123.0)
                                );
                                
                                // 处理按钮点击
                                if ok_clicked || enter_pressed {
                                    // 验证输入
                                    if self.input_text == self.character_name {
                                        // 输入正确，发送删除事件
                                        self.last_event = DeleteCharacterEvent::Delete(self.character_index);
                                        *open = false;
                                    } else {
                                        // 输入错误，显示错误消息
                                        self.step = DeleteStep::Error;
                                        self.error_box.title = "错误".to_string();
                                        self.error_box.text = "角色名称输入错误！".to_string();
                                    }
                                }
                                
                                if cancel_clicked || esc_pressed {
                                    self.last_event = DeleteCharacterEvent::Cancel;
                                    *open = false;
                                }
                            });
                    }
                }
            }
            
            DeleteStep::Error => {
                // 显示错误消息
                let mut error_open = true;
                self.error_box.show(ctx, &mut error_open);
                
                if !error_open {
                    // 错误消息关闭，返回输入步骤
                    self.step = DeleteStep::InputName;
                    self.input_text.clear();
                }
            }
        }
    }
}
