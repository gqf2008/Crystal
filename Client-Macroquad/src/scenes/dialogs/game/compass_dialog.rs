// ============================================================================
// CompassDialogHybrid - 罗盘对话框
// ============================================================================
//
// C# 参考: Client/MIRScenes/Dialogs/CompassDialog.cs (~64 行)
// - 显示方向指示器 (8 方向: 东/南/西/北/东南/东北/西南/西北)
// - 固定在屏幕左上角
// - 点击可切换显示/隐藏
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use super::native_ui_utils::DragHelper;

/// 8 个方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompassDirection {
    East,       // 东
    SouthEast,  // 东南
    South,      // 南
    SouthWest,  // 西南
    West,       // 西
    NorthWest,  // 西北
    North,      // 北
    NorthEast,  // 东北
}

impl CompassDirection {
    fn label(&self) -> &'static str {
        match self {
            Self::East => "东",
            Self::SouthEast => "东南",
            Self::South => "南",
            Self::SouthWest => "西南",
            Self::West => "西",
            Self::NorthWest => "西北",
            Self::North => "北",
            Self::NorthEast => "东北",
        }
    }

    /// 从字节值转换 (0=东, 顺时针)
    pub fn from_u8(val: u8) -> Self {
        match val % 8 {
            0 => Self::East,
            1 => Self::SouthEast,
            2 => Self::South,
            3 => Self::SouthWest,
            4 => Self::West,
            5 => Self::NorthWest,
            6 => Self::North,
            7 => Self::NorthEast,
            _ => Self::East,
        }
    }

    /// 从坐标位置计算方向 (0=东, 顺时针)
    pub fn from_location(dx: i32, dy: i32) -> Self {
        // atan2 返回 -PI 到 PI，转换为 0-7 的 8 方向索引
        // 0=东, 顺时针
        let angle = (dy as f32).atan2(dx as f32); // -PI..PI
        let mut sector = ((angle + std::f32::consts::PI) / (std::f32::consts::PI / 4.0)).round() as i32 % 8;
        if sector < 0 { sector += 8; }
        Self::from_u8(sector as u8)
    }
}

pub struct CompassDialogHybrid {
    position: Vec2,
    visible: bool,
    size: Vec2,
    direction: CompassDirection,
    drag_helper: DragHelper,
    // 纹理
    bg_texture: Option<Texture2D>,
    _needle_texture: Option<Texture2D>,
}

impl Default for CompassDialogHybrid {
    fn default() -> Self {
        Self::new()
    }
}

impl CompassDialogHybrid {
    pub fn new() -> Self {
        Self {
            position: vec2(10.0, 10.0),
            visible: false,
            size: vec2(80.0, 80.0),
            direction: CompassDirection::North,
            drag_helper: DragHelper::new(),
            bg_texture: None,
            _needle_texture: None,
        }
    }

    pub fn open(&mut self) {
        self.visible = true;
    }

    pub fn close(&mut self) {
        self.visible = false;
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn set_position(&mut self, pos: Vec2) {
        self.position = pos;
    }

    pub fn get_position(&self) -> Vec2 {
        self.position
    }

    pub fn contains(&self, point: Vec2) -> bool {
        if !self.visible {
            return false;
        }
        Rect::new(self.position.x, self.position.y, self.size.x, self.size.y).contains(point)
    }

    /// 更新方向（服务器推送）
    pub fn set_direction(&mut self, direction: CompassDirection) {
        self.direction = direction;
    }

    /// 加载纹理
    pub fn load_textures(&mut self) {
        // 罗盘背景: Title[?] (使用通用圆形背景)
        // 罗盘指针: Title[?]
        // 由于不确定具体索引，先用占位
        if let Some(texture) = LibraryName::Title.get_texture(468) {
            if let Some(tex) = texture.image {
                self.bg_texture = Some(tex);
            }
        }
    }

    pub fn update_and_draw(&mut self) {
        if !self.visible {
            return;
        }

        let mouse_pos = vec2(mouse_position().0, mouse_position().1);

        // 拖拽
        let drag_area = Rect::new(self.position.x, self.position.y, self.size.x, 20.0);
        self.drag_helper.apply(drag_area, &mut self.position);

        self.draw_compass(mouse_pos);
    }

    fn draw_compass(&self, mouse_pos: Vec2) {
        let is_hovered = Rect::new(self.position.x, self.position.y, self.size.x, self.size.y)
            .contains(mouse_pos);

        // 背景圆
        let center_x = self.position.x + self.size.x / 2.0;
        let center_y = self.position.y + self.size.y / 2.0;
        let radius = self.size.x / 2.0;

        if let Some(tex) = &self.bg_texture {
            draw_texture_ex(
                tex,
                self.position.x,
                self.position.y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(self.size.x, self.size.y)),
                    ..Default::default()
                },
            );
        } else {
            // 绘制罗盘圆形背景
            draw_circle(center_x, center_y, radius, Color::from_rgba(40, 40, 50, 220));
            draw_circle_lines(center_x, center_y, radius, 2.0, Color::from_rgba(100, 100, 120, 255));
        }

        // 方向文字
        let dir_label = self.direction.label();
        let angle = match self.direction {
            CompassDirection::East => 0.0,
            CompassDirection::SouthEast => std::f32::consts::FRAC_PI_4,
            CompassDirection::South => std::f32::consts::FRAC_PI_2,
            CompassDirection::SouthWest => 3.0 * std::f32::consts::FRAC_PI_4,
            CompassDirection::West => std::f32::consts::PI,
            CompassDirection::NorthWest => 5.0 * std::f32::consts::FRAC_PI_4,
            CompassDirection::North => -std::f32::consts::FRAC_PI_2,
            CompassDirection::NorthEast => -std::f32::consts::FRAC_PI_4,
        };

        // 绘制指针
        let needle_len = radius * 0.7;
        let needle_x = center_x + angle.cos() * needle_len;
        let needle_y = center_y + angle.sin() * needle_len;
        draw_line(center_x, center_y, needle_x, needle_y, 2.5, Color::from_rgba(255, 80, 80, 255));

        // 方向标签
        crate::ui::text_renderer::draw_text_cn(
            dir_label,
            center_x - 10.0,
            center_y + 4.0,
            10.0,
            WHITE,
        );

        // 悬停高亮
        if is_hovered {
            draw_circle_lines(center_x, center_y, radius, 2.5, Color::from_rgba(200, 200, 100, 255));
        }

        // 点击切换
        if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
            // 切换逻辑由外部处理
        }
    }
}
