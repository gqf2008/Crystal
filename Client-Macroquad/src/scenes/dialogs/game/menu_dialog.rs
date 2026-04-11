// ============================================================================
// MenuDialogHybrid - 游戏菜单对话框（混合版本）
// ============================================================================
//
// 【实现方式】
// - 使用 macroquad 原生 draw_* 函数绘制
// - 使用 DragHelper 实现拖拽功能
// - C#: Index = 567, Library = Libraries.Title
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use super::native_ui_utils::DragHelper;

/// 菜单项类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuAction {
    Exit,        // 退出游戏
    Logout,      // 下线
    Help,        // 帮助
    Keyboard,    // 键盘设置
    Ranking,     // 排名
    Creature,    // 宠物
    Mount,       // 坐骑
    Fishing,     // 钓鱼
    Friends,     // 好友
    Mentor,      // 师徒
    Relationship,// 关系
    Group,       // 组队
    Guild,       // 行会
}

/// 菜单按钮数据
struct MenuButton {
    action: MenuAction,
    label: &'static str,
    // C#: Prguse 或 Prguse2 库中的纹理索引
    normal_idx: usize,
    hover_idx: usize,
    pressed_idx: usize,
    library: LibraryName,
    y_offset: f32,
}

/// 游戏菜单对话框（混合版本）
pub struct MenuDialogHybrid {
    /// 窗口位置
    position: Vec2,
    /// 是否可见
    visible: bool,
    /// 对话框尺寸
    size: Vec2,
    /// 背景纹理
    bg_texture: Option<Texture2D>,
    /// 拖拽辅助器
    drag_helper: DragHelper,
}

impl Default for MenuDialogHybrid {
    fn default() -> Self {
        Self::new()
    }
}

impl MenuDialogHybrid {
    /// 菜单按钮配置 (基于 C# 源码)
    const MENU_BUTTONS: &'static [MenuButton] = &[
        // C#: ExitButton: Index=633, Location=(3, 12), Library=Title
        MenuButton { action: MenuAction::Exit, label: "退出", normal_idx: 633, hover_idx: 634, pressed_idx: 635, library: LibraryName::Title, y_offset: 12.0 },
        // C#: LogOutButton: Index=636, Location=(3, 31), Library=Title
        MenuButton { action: MenuAction::Logout, label: "下线", normal_idx: 636, hover_idx: 637, pressed_idx: 638, library: LibraryName::Title, y_offset: 31.0 },
        // C#: HelpButton: Index=1970, Location=(3, 50), Library=Prguse
        MenuButton { action: MenuAction::Help, label: "帮助", normal_idx: 1970, hover_idx: 1971, pressed_idx: 1972, library: LibraryName::Prguse, y_offset: 50.0 },
        // C#: KeyboardLayoutButton: Index=1973, Location=(3, 69), Library=Prguse
        MenuButton { action: MenuAction::Keyboard, label: "键盘", normal_idx: 1973, hover_idx: 1974, pressed_idx: 1975, library: LibraryName::Prguse, y_offset: 69.0 },
        // C#: RankingButton: Index=2000, Location=(3, 88), Library=Prguse
        MenuButton { action: MenuAction::Ranking, label: "排名", normal_idx: 2000, hover_idx: 2001, pressed_idx: 2002, library: LibraryName::Prguse, y_offset: 88.0 },
        // C#: IntelligentCreatureButton: Index=431, Location=(3, 126), Library=Prguse2
        MenuButton { action: MenuAction::Creature, label: "宠物", normal_idx: 431, hover_idx: 432, pressed_idx: 433, library: LibraryName::Prguse2, y_offset: 126.0 },
        // C#: RideButton: Index=1976, Location=(3, 145), Library=Prguse
        MenuButton { action: MenuAction::Mount, label: "坐骑", normal_idx: 1976, hover_idx: 1977, pressed_idx: 1978, library: LibraryName::Prguse, y_offset: 145.0 },
        // C#: FishingButton: Index=1979, Location=(3, 164), Library=Prguse
        MenuButton { action: MenuAction::Fishing, label: "钓鱼", normal_idx: 1979, hover_idx: 1980, pressed_idx: 1981, library: LibraryName::Prguse, y_offset: 164.0 },
        // C#: FriendButton: Index=1982, Location=(3, 183), Library=Prguse
        MenuButton { action: MenuAction::Friends, label: "好友", normal_idx: 1982, hover_idx: 1983, pressed_idx: 1984, library: LibraryName::Prguse, y_offset: 183.0 },
        // C#: MentorButton: Index=1985, Location=(3, 202), Library=Prguse
        MenuButton { action: MenuAction::Mentor, label: "师徒", normal_idx: 1985, hover_idx: 1986, pressed_idx: 1987, library: LibraryName::Prguse, y_offset: 202.0 },
    ];
    
    pub fn new() -> Self {
        // 初始位置将在显示时根据屏幕计算
        Self {
            position: vec2(400.0, 200.0),
            visible: false,
            size: vec2(44.0, 224.0), // 默认尺寸，将被纹理尺寸覆盖
            bg_texture: None,
            drag_helper: DragHelper::new(),
        }
    }

    /// 显示对话框
    pub fn open(&mut self) {
        if !self.visible {
            self.visible = true;
            // 更新位置到屏幕右侧
            let screen_w = screen_width() / screen_dpi_scale();
            let screen_h = screen_height() / screen_dpi_scale();
            // C#: Location = new Point(Settings.ScreenWidth - Size.Width, GameScene.Scene.MainDialog.Location.Y - this.Size.Height + 15)
            // MainDialog 高度约150, 位于屏幕底部
            let main_dialog_top = screen_h - 150.0;
            self.position = vec2(screen_w - self.size.x, main_dialog_top - self.size.y + 15.0);
        }
    }

    /// 关闭对话框
    pub fn close(&mut self) {
        self.visible = false;
    }

    /// 切换显示状态
    pub fn toggle(&mut self) {
        if self.visible {
            self.close();
        } else {
            self.open();
        }
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

    /// 异步加载纹理
    pub fn load_textures(&mut self) {
        // 加载背景纹理 Title[567]
        if let Some(texture) = LibraryName::Title.get_texture(567) {
            self.size = vec2(texture.width as f32, texture.height as f32);
            self.bg_texture = texture.image;
        }
        
        // 预加载所有按钮纹理
        for btn in Self::MENU_BUTTONS {
            let _ = btn.library.get_texture(btn.normal_idx);
            let _ = btn.library.get_texture(btn.hover_idx);
            let _ = btn.library.get_texture(btn.pressed_idx);
        }
    }

    /// 更新和绘制（返回触发的菜单动作）
    pub fn update_and_draw(&mut self) -> Option<MenuAction> {
        if !self.visible {
            return None;
        }

        let mouse_pos = vec2(mouse_position().0, mouse_position().1);

        // 使用 DragHelper 实现拖拽
        let drag_area = Rect::new(self.position.x, self.position.y, self.size.x, 12.0);
        self.drag_helper.apply(drag_area, &mut self.position);

        // 绘制背景
        self.draw_background();

        // 绘制菜单按钮并检测点击
        

        self.draw_menu_buttons(mouse_pos)
    }

    /// 绘制背景
    fn draw_background(&self) {
        if let Some(ref texture) = self.bg_texture {
            draw_texture_ex(
                texture,
                self.position.x,
                self.position.y,
                WHITE,
                DrawTextureParams::default(),
            );
        } else {
            // 降级：绘制默认背景
            draw_rectangle(
                self.position.x,
                self.position.y,
                self.size.x,
                self.size.y,
                Color::from_rgba(25, 25, 35, 240),
            );
            draw_rectangle_lines(
                self.position.x,
                self.position.y,
                self.size.x,
                self.size.y,
                2.0,
                Color::from_rgba(100, 100, 100, 255),
            );
            draw_text_cn("菜单", self.position.x + 5.0, self.position.y + 15.0, 12.0, WHITE);
        }
    }

    /// 绘制菜单按钮（返回点击的动作）
    fn draw_menu_buttons(&self, mouse_pos: Vec2) -> Option<MenuAction> {
        let mut result = None;
        let mouse_pressed = is_mouse_button_pressed(MouseButton::Left);
        let mouse_down = is_mouse_button_down(MouseButton::Left);

        for btn in Self::MENU_BUTTONS {
            // C#: Location = new Point(3, y_offset)
            let btn_x = self.position.x + 3.0;
            let btn_y = self.position.y + btn.y_offset;
            
            // 获取按钮尺寸
            let btn_size = if let Some(tex_info) = btn.library.get_texture(btn.normal_idx) {
                vec2(tex_info.width as f32, tex_info.height as f32)
            } else {
                vec2(38.0, 19.0) // 默认按钮尺寸
            };
            
            let btn_rect = Rect::new(btn_x, btn_y, btn_size.x, btn_size.y);
            let is_hovered = btn_rect.contains(mouse_pos);
            let is_pressed = is_hovered && mouse_down;

            // 选择纹理索引
            let texture_idx = if is_pressed {
                btn.pressed_idx
            } else if is_hovered {
                btn.hover_idx
            } else {
                btn.normal_idx
            };

            // 绘制按钮
            if let Some(tex_info) = btn.library.get_texture(texture_idx) {
                if let Some(ref tex) = tex_info.image {
                    draw_texture_ex(tex, btn_x, btn_y, WHITE, DrawTextureParams::default());
                }
            } else {
                // 降级绘制
                let bg_color = if is_pressed {
                    Color::from_rgba(100, 100, 150, 255)
                } else if is_hovered {
                    Color::from_rgba(70, 70, 100, 255)
                } else {
                    Color::from_rgba(50, 50, 70, 255)
                };
                draw_rectangle(btn_x, btn_y, btn_size.x, btn_size.y, bg_color);
                draw_text_cn(btn.label, btn_x + 2.0, btn_y + 14.0, 11.0, WHITE);
            }

            // 检测点击
            if is_hovered && mouse_pressed {
                result = Some(btn.action);
            }
        }

        result
    }
}
