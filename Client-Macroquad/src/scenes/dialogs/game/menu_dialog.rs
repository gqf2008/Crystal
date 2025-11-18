// ============================================================================
// MenuDialog - 游戏菜单对话框
// ============================================================================
// 
// 【功能说明】
// 1. 游戏主菜单：存档、读档、设置、退出等
// 2. 快速功能入口：回城、下线、退出游戏
// 3. 游戏信息：在线时间、服务器状态等
// 
// ============================================================================

use egui_macroquad::egui;
use crate::resources::LibraryName;
use crate::scenes::dialogs::Dialog;

/// 菜单项类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuAction {
    ReturnToCity,  // 回城
    SaveGame,      // 存档
    LoadGame,      // 读档
    Options,       // 设置
    Help,          // 帮助
    Logout,        // 下线
    ExitGame,      // 退出游戏
}

/// 游戏菜单对话框
pub struct MenuDialog {
    /// 是否可见
    visible: bool,
    /// 窗口位置
    position: egui::Pos2,
    /// 是否正在拖拽
    dragging: bool,
    /// 拖拽偏移
    drag_offset: egui::Vec2,
    /// 在线时间（秒）
    online_time: u64,
    /// 服务器名称
    server_name: String,
    /// 玩家等级
    player_level: u32,
    /// 经验值
    experience: u64,
    /// 下次升级所需经验
    next_level_exp: u64,
}

impl MenuDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: egui::pos2(400.0, 200.0),
            dragging: false,
            drag_offset: egui::Vec2::ZERO,
            online_time: 3661, // 1小时1分1秒
            server_name: "传奇服务器".to_string(),
            player_level: 45,
            experience: 450000,
            next_level_exp: 500000,
        }
    }

    /// 切换可见性
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        println!("📋 菜单对话框: {}", if self.visible { "打开" } else { "关闭" });
    }

    /// 获取可见性
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// 设置可见性
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// 格式化在线时间
    fn format_online_time(&self) -> String {
        let hours = self.online_time / 3600;
        let minutes = (self.online_time % 3600) / 60;
        let seconds = self.online_time % 60;
        format!("{}:{:02}:{:02}", hours, minutes, seconds)
    }

    /// 绘制对话框背景
    fn draw_background(&self, ui: &mut egui::Ui, ctx: &egui::Context) -> egui::Rect {
        let bg_size = egui::vec2(350.0, 450.0);
        let bg_rect = egui::Rect::from_min_size(self.position, bg_size);
        
        // 绘制背景
        ui.painter().rect_filled(
            bg_rect,
            5.0,
            egui::Color32::from_rgba_premultiplied(25, 25, 35, 240),
        );
        ui.painter().rect_stroke(
            bg_rect,
            5.0,
            egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 100, 100)),
            egui::epaint::StrokeKind::Outside,
        );

        // 绘制标题
        ui.painter().text(
            egui::pos2(bg_rect.min.x + 20.0, bg_rect.min.y + 15.0),
            egui::Align2::LEFT_CENTER,
            "📋 游戏菜单",
            egui::FontId::proportional(16.0),
            egui::Color32::from_rgb(255, 215, 0),
        );

        bg_rect
    }

    /// 绘制游戏信息
    fn draw_game_info(&self, ui: &mut egui::Ui, bg_rect: &egui::Rect) {
        let info_area = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 20.0, bg_rect.min.y + 50.0),
            egui::vec2(310.0, 100.0)
        );

        ui.painter().rect_filled(
            info_area,
            3.0,
            egui::Color32::from_rgba_premultiplied(20, 20, 30, 200),
        );
        ui.painter().rect_stroke(
            info_area,
            3.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80)),
            egui::epaint::StrokeKind::Outside,
        );

        let mut y = info_area.min.y + 15.0;
        let line_height = 20.0;

        // 服务器信息
        ui.painter().text(
            egui::pos2(info_area.min.x + 15.0, y),
            egui::Align2::LEFT_TOP,
            &format!("服务器: {}", self.server_name),
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );
        y += line_height;

        // 在线时间
        ui.painter().text(
            egui::pos2(info_area.min.x + 15.0, y),
            egui::Align2::LEFT_TOP,
            &format!("在线时间: {}", self.format_online_time()),
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(0, 255, 0),
        );
        y += line_height;

        // 等级信息
        ui.painter().text(
            egui::pos2(info_area.min.x + 15.0, y),
            egui::Align2::LEFT_TOP,
            &format!("等级: {} 级", self.player_level),
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(255, 215, 0),
        );
        y += line_height;

        // 经验信息
        let exp_percent = (self.experience as f32 / self.next_level_exp as f32 * 100.0) as u32;
        ui.painter().text(
            egui::pos2(info_area.min.x + 15.0, y),
            egui::Align2::LEFT_TOP,
            &format!("经验: {}% ({}/{})", exp_percent, self.experience, self.next_level_exp),
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(0, 200, 255),
        );
    }

    /// 绘制菜单按钮
    fn draw_menu_buttons(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        let buttons_area = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 20.0, bg_rect.min.y + 170.0),
            egui::vec2(310.0, 250.0)
        );

        let button_width = 280.0;
        let button_height = 30.0;
        let button_spacing = 10.0;

        let menu_items = [
            (MenuAction::ReturnToCity, "🏠 回城", "立即传送回城", egui::Color32::from_rgb(100, 150, 100), None),
            (MenuAction::SaveGame, "💾 存档", "保存当前游戏进度", egui::Color32::from_rgb(100, 100, 150), None),
            (MenuAction::LoadGame, "📁 读档", "加载之前的存档", egui::Color32::from_rgb(100, 100, 150), None),
            (MenuAction::Options, "⚙️ 设置", "打开游戏设置", egui::Color32::from_rgb(120, 120, 120), None),
            (MenuAction::Help, "❓ 帮助", "查看游戏帮助", egui::Color32::from_rgb(100, 120, 150), None),
            (MenuAction::Logout, "🚪 下线", "安全下线到角色选择", egui::Color32::from_rgb(150, 150, 100), Some((636, 637, 638))), // Title库的下线按钮
            (MenuAction::ExitGame, "❌ 退出游戏", "完全退出游戏", egui::Color32::from_rgb(150, 100, 100), Some((633, 634, 635))), // Title库的退出按钮
        ];

        for (i, (action, label, description, color, texture_indices)) in menu_items.iter().enumerate() {
            let button_y = buttons_area.min.y + i as f32 * (button_height + button_spacing);
            let button_rect = egui::Rect::from_min_size(
                egui::pos2(buttons_area.min.x + 15.0, button_y),
                egui::vec2(button_width, button_height)
            );

            let response = ui.interact(button_rect, egui::Id::new(format!("menu_{:?}", action)), egui::Sense::click());
            
            // 尝试使用纹理按钮
            let mut used_texture = false;
            if let Some((normal, hover, pressed)) = texture_indices {
                let texture_index = if response.clicked() {
                    *pressed
                } else if response.hovered() {
                    *hover
                } else {
                    *normal
                };
                
                if let Some(info) = LibraryName::Title.get_egui_texture(ctx, texture_index) {
                    if let Some(btn_texture) = info.egui_texture {
                        ui.painter().image(
                            btn_texture.id(),
                            button_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                        used_texture = true;
                    }
                }
            }
            
            // 降级：使用自定义按钮
            if !used_texture {
                let bg_color = if response.hovered() {
                    egui::Color32::from_rgb(color.r() + 20, color.g() + 20, color.b() + 20)
                } else {
                    *color
                };

                ui.painter().rect_filled(button_rect, 5.0, bg_color);
            }
            ui.painter().rect_stroke(
                button_rect,
                5.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(150, 150, 150)),
                egui::epaint::StrokeKind::Outside,
            );

            // 按钮文字
            ui.painter().text(
                egui::pos2(button_rect.min.x + 15.0, button_rect.center().y),
                egui::Align2::LEFT_CENTER,
                label,
                egui::FontId::proportional(13.0),
                egui::Color32::WHITE,
            );

            // 处理点击
            if response.clicked() {
                self.handle_menu_action(*action);
            }

            // 悬停提示
            if response.hovered() {
                response.on_hover_text(*description);
            }
        }
    }

    /// 处理菜单动作
    fn handle_menu_action(&mut self, action: MenuAction) {
        match action {
            MenuAction::ReturnToCity => {
                println!("🏠 执行回城操作");
                // TODO: 实现回城功能
            },
            MenuAction::SaveGame => {
                println!("💾 保存游戏");
                // TODO: 实现存档功能
            },
            MenuAction::LoadGame => {
                println!("📁 加载游戏");
                // TODO: 实现读档功能
            },
            MenuAction::Options => {
                println!("⚙️ 打开设置");
                // TODO: 打开设置对话框
            },
            MenuAction::Help => {
                println!("❓ 显示帮助");
                // TODO: 打开帮助对话框
            },
            MenuAction::Logout => {
                println!("🚪 安全下线");
                // TODO: 实现下线功能
            },
            MenuAction::ExitGame => {
                println!("❌ 退出游戏");
                // TODO: 实现退出游戏功能
                std::process::exit(0);
            },
        }
    }

    /// 绘制关闭按钮
    fn draw_close_button(&self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) -> bool {
        // 关闭按钮位置（右上角）
        let close_size = egui::vec2(20.0, 20.0);
        let close_rect = egui::Rect::from_min_size(
            egui::pos2(bg_rect.max.x - 25.0, bg_rect.min.y + 5.0),
            close_size
        );

        // 绘制关闭按钮背景
        ui.painter().rect_filled(close_rect, 2.0, egui::Color32::from_rgb(150, 50, 50));
        ui.painter().rect_stroke(
            close_rect,
            2.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(200, 100, 100)),
            egui::epaint::StrokeKind::Outside,
        );

        // 绘制关闭符号 "×"
        ui.painter().text(
            close_rect.center(),
            egui::Align2::CENTER_CENTER,
            "×",
            egui::FontId::proportional(14.0),
            egui::Color32::WHITE,
        );

        let response = ui.interact(close_rect, egui::Id::new("menu_close"), egui::Sense::click());
        let is_clicked = response.clicked();
        if response.hovered() {
            response.on_hover_text("关闭");
        }

        is_clicked
    }

    /// 处理窗口拖拽
    fn handle_window_dragging(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        // 标题栏区域作为拖拽区域
        let title_area = egui::Rect::from_min_size(
            bg_rect.min,
            egui::vec2(bg_rect.width(), 30.0),
        );
        
        let drag_response = ui.interact(title_area, egui::Id::new("menu_drag"), egui::Sense::drag());
        
        if drag_response.drag_started() {
            self.dragging = true;
            if let Some(pointer_pos) = ctx.pointer_interact_pos() {
                self.drag_offset = self.position.to_vec2() - pointer_pos.to_vec2();
            }
        }
        
        if self.dragging {
            if let Some(pointer_pos) = ctx.pointer_interact_pos() {
                self.position = (pointer_pos.to_vec2() + self.drag_offset).to_pos2();
            }
        }
        
        if drag_response.drag_stopped() {
            self.dragging = false;
        }
    }
}

impl Dialog for MenuDialog {
    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        if !self.visible {
            *open = false;
            return;
        }
        
        // 使用 Area 创建自由浮动窗口
        egui::Area::new(egui::Id::new("menu_dialog"))
            .fixed_pos(self.position)
            .movable(false)  // 使用自定义拖拽
            .show(ctx, |ui| {
                // 绘制背景
                let bg_rect = self.draw_background(ui, ctx);
                
                // 处理窗口拖拽
                self.handle_window_dragging(ui, ctx, &bg_rect);
                
                // 绘制游戏信息
                self.draw_game_info(ui, &bg_rect);
                
        // 绘制菜单按钮
        self.draw_menu_buttons(ui, ctx, &bg_rect);                // 绘制关闭按钮
                if self.draw_close_button(ui, ctx, &bg_rect) {
                    self.visible = false;
                    *open = false;
                }
            });
        
        *open = self.visible;
    }
}