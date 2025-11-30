// ============================================================================
// MenuDialogHybrid - 游戏菜单对话框（混合版本）
// ============================================================================
//
// 【实现方式】
// - 使用 macroquad 原生 draw_* 函数绘制
// - 使用 DragHelper 实现拖拽功能
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use super::native_ui_utils::DragHelper;

/// 菜单项类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuAction {
    ReturnToCity, // 回城
    SaveGame,     // 存档
    LoadGame,     // 读档
    Options,      // 设置
    Help,         // 帮助
    Logout,       // 下线
    ExitGame,     // 退出游戏
}

/// 游戏菜单对话框（混合版本）
pub struct MenuDialogHybrid {
    /// 窗口位置
    position: Vec2,
    /// 是否可见
    visible: bool,
    /// 对话框尺寸
    size: Vec2,
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
    /// 悬停的按钮索引
    hovered_button: Option<usize>,
    /// 按下的按钮索引
    pressed_button: Option<usize>,
    /// 拖拽辅助器
    drag_helper: DragHelper,
}

impl MenuDialogHybrid {
    pub fn new() -> Self {
        Self {
            position: vec2(400.0, 200.0),
            visible: false,
            size: vec2(350.0, 450.0),
            online_time: 3661, // 1小时1分1秒
            server_name: "传奇服务器".to_string(),
            player_level: 45,
            experience: 450000,
            next_level_exp: 500000,
            hovered_button: None,
            pressed_button: None,
            drag_helper: DragHelper::new(),
        }
    }

    /// 显示对话框
    pub fn open(&mut self) {
        self.visible = true;
    }

    /// 关闭对话框
    pub fn close(&mut self) {
        self.visible = false;
    }

    /// 切换显示状态
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// 是否可见
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// 设置位置
    pub fn set_position(&mut self, pos: Vec2) {
        self.position = pos;
    }

    /// 获取位置
    pub fn get_position(&self) -> Vec2 {
        self.position
    }

    /// 检查点是否在对话框内
    pub fn contains(&self, point: Vec2) -> bool {
        if !self.visible {
            return false;
        }
        point.x >= self.position.x
            && point.x <= self.position.x + self.size.x
            && point.y >= self.position.y
            && point.y <= self.position.y + self.size.y
    }

    /// 格式化在线时间
    fn format_online_time(&self) -> String {
        let hours = self.online_time / 3600;
        let minutes = (self.online_time % 3600) / 60;
        let seconds = self.online_time % 60;
        format!("{}:{:02}:{:02}", hours, minutes, seconds)
    }

    /// 异步加载纹理
    pub async fn load_textures(&mut self) {
        // 预加载 Title 库中的按钮纹理
        for idx in [633, 634, 635, 636, 637, 638] {
            let _ = LibraryName::Title.get_texture(idx);
        }
    }

    /// 更新和绘制（返回触发的菜单动作）
    pub fn update_and_draw(&mut self) -> Option<MenuAction> {
        if !self.visible {
            return None;
        }

        let mouse_pos = vec2(mouse_position().0, mouse_position().1);
        let mut action = None;

        // 使用 DragHelper 实现拖拽
        let drag_area = Rect::new(self.position.x, self.position.y, self.size.x, 30.0);
        self.drag_helper.apply(drag_area, &mut self.position);

        // 绘制背景
        self.draw_background();

        // 绘制游戏信息
        self.draw_game_info();

        // 绘制菜单按钮并检测点击
        action = self.draw_menu_buttons(mouse_pos);

        // 绘制关闭按钮
        if self.draw_close_button(mouse_pos) {
            self.close();
        }

        action
    }

    /// 绘制背景
    fn draw_background(&self) {
        // 背景
        draw_rectangle(
            self.position.x,
            self.position.y,
            self.size.x,
            self.size.y,
            Color::from_rgba(25, 25, 35, 240),
        );

        // 边框
        draw_rectangle_lines(
            self.position.x,
            self.position.y,
            self.size.x,
            self.size.y,
            2.0,
            Color::from_rgba(100, 100, 100, 255),
        );

        // 标题
        draw_text(
            "游戏菜单",
            self.position.x + 20.0,
            self.position.y + 25.0,
            20.0,
            Color::from_rgba(255, 215, 0, 255),
        );
    }

    /// 绘制游戏信息
    fn draw_game_info(&self) {
        let info_x = self.position.x + 20.0;
        let info_y = self.position.y + 50.0;
        let info_width = 310.0;
        let info_height = 100.0;

        // 信息区域背景
        draw_rectangle(
            info_x,
            info_y,
            info_width,
            info_height,
            Color::from_rgba(20, 20, 30, 200),
        );
        draw_rectangle_lines(
            info_x,
            info_y,
            info_width,
            info_height,
            1.0,
            Color::from_rgba(80, 80, 80, 255),
        );

        let mut y = info_y + 20.0;
        let line_height = 20.0;

        // 服务器信息
        draw_text(
            &format!("服务器: {}", self.server_name),
            info_x + 15.0,
            y,
            14.0,
            WHITE,
        );
        y += line_height;

        // 在线时间
        draw_text(
            &format!("在线时间: {}", self.format_online_time()),
            info_x + 15.0,
            y,
            14.0,
            Color::from_rgba(0, 255, 0, 255),
        );
        y += line_height;

        // 等级信息
        draw_text(
            &format!("等级: {} 级", self.player_level),
            info_x + 15.0,
            y,
            14.0,
            Color::from_rgba(255, 215, 0, 255),
        );
        y += line_height;

        // 经验信息
        let exp_percent = (self.experience as f32 / self.next_level_exp as f32 * 100.0) as u32;
        draw_text(
            &format!(
                "经验: {}% ({}/{})",
                exp_percent, self.experience, self.next_level_exp
            ),
            info_x + 15.0,
            y,
            14.0,
            Color::from_rgba(0, 200, 255, 255),
        );
    }

    /// 绘制菜单按钮（返回点击的动作）
    fn draw_menu_buttons(&mut self, mouse_pos: Vec2) -> Option<MenuAction> {
        let buttons_x = self.position.x + 35.0;
        let buttons_y = self.position.y + 170.0;
        let button_width = 280.0;
        let button_height = 30.0;
        let button_spacing = 10.0;

        let menu_items = [
            (MenuAction::ReturnToCity, "回城", "立即传送回城", Color::from_rgba(100, 150, 100, 255)),
            (MenuAction::SaveGame, "存档", "保存当前游戏进度", Color::from_rgba(100, 100, 150, 255)),
            (MenuAction::LoadGame, "读档", "加载之前的存档", Color::from_rgba(100, 100, 150, 255)),
            (MenuAction::Options, "设置", "打开游戏设置", Color::from_rgba(120, 120, 120, 255)),
            (MenuAction::Help, "帮助", "查看游戏帮助", Color::from_rgba(100, 120, 150, 255)),
            (MenuAction::Logout, "下线", "安全下线到角色选择", Color::from_rgba(150, 150, 100, 255)),
            (MenuAction::ExitGame, "退出游戏", "完全退出游戏", Color::from_rgba(150, 100, 100, 255)),
        ];

        let mut result = None;
        let mouse_pressed = is_mouse_button_pressed(MouseButton::Left);
        let mouse_down = is_mouse_button_down(MouseButton::Left);

        for (i, (action, label, _description, color)) in menu_items.iter().enumerate() {
            let button_y = buttons_y + i as f32 * (button_height + button_spacing);
            let button_rect = Rect::new(buttons_x, button_y, button_width, button_height);

            let is_hovered = button_rect.contains(mouse_pos);
            let is_pressed = is_hovered && mouse_down;

            // 按钮背景
            let bg_color = if is_pressed {
                Color::from_rgba(
                    (color.r * 255.0) as u8 + 40,
                    (color.g * 255.0) as u8 + 40,
                    (color.b * 255.0) as u8 + 40,
                    255,
                )
            } else if is_hovered {
                Color::from_rgba(
                    (color.r * 255.0) as u8 + 20,
                    (color.g * 255.0) as u8 + 20,
                    (color.b * 255.0) as u8 + 20,
                    255,
                )
            } else {
                *color
            };

            draw_rectangle(
                button_rect.x,
                button_rect.y,
                button_rect.w,
                button_rect.h,
                bg_color,
            );
            draw_rectangle_lines(
                button_rect.x,
                button_rect.y,
                button_rect.w,
                button_rect.h,
                1.0,
                Color::from_rgba(150, 150, 150, 255),
            );

            // 按钮文字
            let text_x = button_rect.x + 15.0;
            let text_y = button_rect.y + button_rect.h / 2.0 + 5.0;
            draw_text(label, text_x, text_y, 16.0, WHITE);

            // 检测点击
            if is_hovered && mouse_pressed {
                result = Some(*action);
            }
        }

        result
    }

    /// 绘制关闭按钮（返回是否点击）
    fn draw_close_button(&self, mouse_pos: Vec2) -> bool {
        let close_size = 20.0;
        let close_x = self.position.x + self.size.x - 25.0;
        let close_y = self.position.y + 5.0;
        let close_rect = Rect::new(close_x, close_y, close_size, close_size);

        let is_hovered = close_rect.contains(mouse_pos);

        // 关闭按钮背景
        let bg_color = if is_hovered {
            Color::from_rgba(200, 70, 70, 255)
        } else {
            Color::from_rgba(150, 50, 50, 255)
        };
        draw_rectangle(close_x, close_y, close_size, close_size, bg_color);
        draw_rectangle_lines(
            close_x,
            close_y,
            close_size,
            close_size,
            1.0,
            Color::from_rgba(200, 100, 100, 255),
        );

        // X 符号
        draw_text("×", close_x + 4.0, close_y + 16.0, 18.0, WHITE);

        is_hovered && is_mouse_button_pressed(MouseButton::Left)
    }

    /// 处理菜单动作
    pub fn handle_action(&self, action: MenuAction) {
        match action {
            MenuAction::ReturnToCity => {
                println!("🏠 执行回城操作");
            }
            MenuAction::SaveGame => {
                println!("💾 保存游戏");
            }
            MenuAction::LoadGame => {
                println!("📁 加载游戏");
            }
            MenuAction::Options => {
                println!("⚙️ 打开设置");
            }
            MenuAction::Help => {
                println!("❓ 显示帮助");
            }
            MenuAction::Logout => {
                println!("🚪 安全下线");
            }
            MenuAction::ExitGame => {
                println!("❌ 退出游戏");
                std::process::exit(0);
            }
        }
    }
}
