// ============================================================================
// ChatControlBar - 聊天控制栏（位于 ChatDialog 上方）
// ============================================================================
// 
// 【功能说明】
// 1. 聊天频道切换按钮（全体、喊话、私聊、夫妻、师徒、组队、行会）
// 2. 功能按钮（大小调整、设置、交易、举报）
// 3. 显示当前选中的聊天频道
// 
// 【位置】
// - 原工程：MainDialog.X + 230, ScreenHeight - 112
// - 尺寸：根据分辨率 (800: Prguse[2035], 1024+: Prguse[2034])
// 
// ============================================================================

use egui_macroquad::egui;
use crate::resources::LibraryName;

/// 聊天频道类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatFilter {
    All,      // 全部
    Shout,    // 喊话 (!)
    Whisper,  // 私聊 (/)
    Lover,    // 夫妻 (:))
    Mentor,   // 师徒 (!#)
    Group,    // 组队 (!!)
    Guild,    // 行会 (!~)
}

impl ChatFilter {
    /// 获取聊天前缀
    pub fn prefix(&self) -> &'static str {
        match self {
            Self::All => "",
            Self::Shout => "!",
            Self::Whisper => "/",
            Self::Lover => ":)",
            Self::Mentor => "!#",
            Self::Group => "!!",
            Self::Guild => "!~",
        }
    }
}

/// 聊天控制栏
pub struct ChatControlBar {
    visible: bool,
    resolution_index: usize,
    position: egui::Pos2,
    
    /// 当前选中的聊天频道
    active_filter: ChatFilter,
}

impl ChatControlBar {
    /// 创建聊天控制栏
    /// 
    /// 原工程：
    /// ```csharp
    /// Index = Settings.Resolution != 800 ? 2034 : 2035;
    /// Library = Libraries.Prguse;
    /// Location = new Point(GameScene.Scene.MainDialog.Location.X + 230, Settings.ScreenHeight - 112);
    /// ```
    pub fn new(main_dialog_x: f32, screen_height: f32, resolution_index: usize) -> Self {
        // 位置：MainDialog.X + 230, ScreenHeight - 112
        let position = egui::pos2(main_dialog_x + 230.0, screen_height - 112.0);
        
        Self {
            visible: true,
            resolution_index,
            position,
            active_filter: ChatFilter::All,
        }
    }
    
    /// 切换聊天频道
    pub fn set_filter(&mut self, filter: ChatFilter) {
        self.active_filter = filter;
    }
    
    /// 设置位置（当 ChatDialog 改变大小时需要同步更新）
    pub fn set_position(&mut self, pos: egui::Pos2) {
        self.position = pos;
    }
    
    /// 获取当前聊天前缀
    pub fn get_chat_prefix(&self) -> &'static str {
        self.active_filter.prefix()
    }
    
    /// 显示控制栏（使用 egui）
    /// 返回：(size_button_clicked, settings_button_clicked)
    pub fn show(&mut self, ctx: &egui::Context, open: &mut bool) -> (bool, bool) {
        if !self.visible || !*open {
            return (false, false);
        }
        
        let mut size_clicked = false;
        let mut settings_clicked = false;
        
        let window = egui::Window::new("ChatControlBar")
            .title_bar(false)
            .resizable(false)
            .fixed_pos(self.position)
            .frame(egui::Frame::NONE);
        
        window.show(ctx, |ui| {
            // 移除所有 UI 间距
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            
            let (sz, st) = self.draw_control_bar(ui, ctx);
            size_clicked = sz;
            settings_clicked = st;
        });
        
        (size_clicked, settings_clicked)
    }
    
    /// 绘制控制栏
    /// 返回：(size_button_clicked, settings_button_clicked)
    fn draw_control_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) -> (bool, bool) {
        // 获取背景纹理索引 (800分辨率用2035，其他用2034)
        let bg_index = if self.resolution_index == 0 { 2035 } else { 2034 };
        
        // 绘制主背景
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, bg_index) {
            if let Some(bg_texture) = info.egui_texture {
                let bg_size = bg_texture.size_vec2();
                let bg_rect = egui::Rect::from_min_size(ui.cursor().min, bg_size);
                
                // 调试信息：打印背景尺寸（只打印一次）
                static mut DEBUG_PRINTED: bool = false;
                unsafe {
                    if !DEBUG_PRINTED {
                        println!("🔍 ChatControlBar 背景纹理尺寸: {}x{}", bg_size.x, bg_size.y);
                        println!("📍 ChatControlBar 绘制位置: {:?}", bg_rect);
                        DEBUG_PRINTED = true;
                    }
                }
                
                // 先绘制背景
                ui.painter().image(
                    bg_texture.id(),
                    bg_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                
                // 然后在背景之上绘制按钮（确保按钮在背景内部）
                self.draw_filter_buttons(ui, ctx, bg_rect);
                let (size_clicked, settings_clicked) = self.draw_function_buttons(ui, ctx, bg_rect);
                
                ui.allocate_rect(bg_rect, egui::Sense::hover());
                
                return (size_clicked, settings_clicked);
            }
        } else {
            println!("⚠️ ChatControlBar: 无法加载背景纹理 Prguse[{}]", bg_index);
            
            // 如果纹理加载失败，绘制临时背景
            let default_width = if self.resolution_index == 0 { 372.0 } else { 596.0 };
            let default_rect = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(default_width, 15.0));
            ui.painter().rect_filled(default_rect, 0.0, egui::Color32::from_rgba_premultiplied(50, 50, 50, 200));
            ui.allocate_rect(default_rect, egui::Sense::hover());
        }
        
        (false, false)
    }
    
    /// 绘制频道选择按钮
    fn draw_filter_buttons(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: egui::Rect) {
        // 按钮位置数组：(x_offset, 频道, 基础纹理索引)
        let button_configs = [
            (12.0, ChatFilter::All, 2036usize),      // NormalButton
            (34.0, ChatFilter::Shout, 2039usize),    // ShoutButton
            (56.0, ChatFilter::Whisper, 2042usize),  // WhisperButton
            (78.0, ChatFilter::Lover, 2045usize),    // LoverButton
            (100.0, ChatFilter::Mentor, 2048usize),  // MentorButton
            (122.0, ChatFilter::Group, 2051usize),   // GroupButton
            (144.0, ChatFilter::Guild, 2054usize),   // GuildButton
        ];
        
        for (x_offset, filter, base_index) in button_configs.iter() {
            // 检查按钮点击
            if self.draw_clickable_button(ui, ctx, bg_rect, *x_offset, 1.0, *base_index, *filter == self.active_filter) {
                self.active_filter = *filter;
                println!("📻 切换聊天频道: {:?} (前缀: '{}')", filter, filter.prefix());
            }
        }
        
        // TradeButton - 位置固定 (166, 1)
        if self.draw_clickable_button(ui, ctx, bg_rect, 166.0, 1.0, 2004, false) {
            println!("💱 打开交易窗口");
        }
    }
    
    /// 绘制功能按钮
    /// 返回：(size_button_clicked, settings_button_clicked)
    fn draw_function_buttons(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: egui::Rect) -> (bool, bool) {
        // SizeButton - 位置根据分辨率变化
        let size_btn_x = if self.resolution_index != 0 { 574.0 } else { 350.0 };
        let size_clicked = self.draw_clickable_button(ui, ctx, bg_rect, size_btn_x, 1.0, 2057, false);
        
        // SettingsButton - 位置根据分辨率变化
        let settings_btn_x = if self.resolution_index != 0 { 596.0 } else { 372.0 };
        let settings_clicked = self.draw_clickable_button(ui, ctx, bg_rect, settings_btn_x, 1.0, 2060, false);
        
        // ReportButton - 默认隐藏，暂不绘制
        // let report_btn_x = if self.resolution_index != 0 { 552.0 } else { 328.0 };
        
        (size_clicked, settings_clicked)
    }
    
    /// 绘制可点击按钮（返回是否被点击）
    fn draw_clickable_button(
        &self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        bg_rect: egui::Rect,
        x_offset: f32,
        y_offset: f32,
        base_index: usize,
        is_selected: bool,
    ) -> bool {
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, base_index) {
            if let Some(btn_texture) = info.egui_texture {
                let btn_size = btn_texture.size_vec2();
                let btn_pos = bg_rect.min + egui::vec2(x_offset, y_offset);
                let btn_rect = egui::Rect::from_min_size(btn_pos, btn_size);
                
                // 交互检测
                let response = ui.interact(
                    btn_rect,
                    egui::Id::new(format!("ctrl_btn_{}_{}", base_index, x_offset)),
                    egui::Sense::click(),
                );
                
                // 根据状态选择纹理：pressed(+2), hover(+1), normal(+0)
                let texture_idx = if is_selected {
                    base_index + 2  // 已选中状态
                } else if response.is_pointer_button_down_on() {
                    base_index + 2  // 按下状态
                } else if response.hovered() {
                    base_index + 1  // 悬停状态
                } else {
                    base_index      // 正常状态
                };
                
                // 绘制按钮
                if let Some(final_info) = LibraryName::Prguse.get_egui_texture(ctx, texture_idx) {
                    if let Some(final_texture) = final_info.egui_texture {
                        ui.painter().image(
                            final_texture.id(),
                            btn_rect,
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
