// ============================================================================
// FishingDialogHybrid - 钓鱼对话框（纯 Native 版本）
// ============================================================================
//
// C# 参考：Client/MirScenes/Dialogs/FishingDialog.cs (385 行)
// - 包含两个部分：
//   1. FishingDialog (装备管理): Prguse[1340] 背景，5 个装备槽
//   2. FishingStatusDialog (钓鱼中): Prguse[1341] 背景，进度条，鱼按钮
//
// ============================================================================

use super::native_ui_utils::DragHelper;
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use macroquad::prelude::*;

/// 钓鱼状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FishingState {
    Idle,
    Waiting,     // 等待咬钩
    Biting,      // 咬钩中
    Reeling,     // 收线中
    AutoCasting, // 自动抛竿
}

/// 钓鱼槽位
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FishingSlot {
    Hook = 0,   // 鱼钩
    Float = 1,  // 浮标
    Bait = 2,   // 鱼饵
    Finder = 3, // 探鱼器
    Reel = 4,   // 鱼轮
}

impl FishingSlot {
    pub fn label(&self) -> &'static str {
        match self {
            FishingSlot::Hook => "鱼钩",
            FishingSlot::Float => "浮标",
            FishingSlot::Bait => "鱼饵",
            FishingSlot::Finder => "探鱼器",
            FishingSlot::Reel => "鱼轮",
        }
    }
}

pub struct FishingDialogHybrid {
    position: Vec2,
    visible: bool,
    size: Vec2,
    fishing_state: FishingState,
    drag_helper: DragHelper,
    // 装备对话框纹理
    equip_bg_texture: Option<Texture2D>,
    // 状态对话框纹理
    status_bg_texture: Option<Texture2D>,
    chance_bar_texture: Option<Texture2D>,
    progress_bar_texture: Option<Texture2D>,
    auto_cast_checked: Option<Texture2D>,
    auto_cast_unchecked: Option<Texture2D>,
    // 钓鱼数据
    chance_percent: f32,
    progress_percent: f32,
    auto_cast_enabled: bool,
    rod_name: String,
    /// 待处理的自动抛竿切换
    pending_autocast_toggle: Option<bool>,
}

impl Default for FishingDialogHybrid {
    fn default() -> Self {
        Self::new()
    }
}

impl FishingDialogHybrid {
    pub fn new() -> Self {
        Self {
            position: vec2(300.0, 100.0),
            visible: false,
            size: vec2(260.0, 280.0),
            fishing_state: FishingState::Idle,
            drag_helper: DragHelper::new(),
            equip_bg_texture: None,
            status_bg_texture: None,
            chance_bar_texture: None,
            progress_bar_texture: None,
            auto_cast_checked: None,
            auto_cast_unchecked: None,
            chance_percent: 0.0,
            progress_percent: 0.0,
            auto_cast_enabled: false,
            rod_name: String::new(),
            pending_autocast_toggle: None,
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

    /// 更新钓鱼状态
    pub fn update_fishing_state(&mut self, state: u8, chance: f32, progress: f32) {
        self.fishing_state = match state {
            0 => FishingState::Idle,
            1 => FishingState::Waiting,
            2 => FishingState::Biting,
            3 => FishingState::Reeling,
            _ => FishingState::Idle,
        };
        self.chance_percent = chance;
        self.progress_percent = progress;
    }

    /// 设置自动抛竿
    pub fn set_auto_cast(&mut self, enabled: bool) {
        self.auto_cast_enabled = enabled;
    }

    /// 设置鱼竿名称
    pub fn set_rod_name(&mut self, name: &str) {
        self.rod_name = name.to_string();
    }

    /// 获取待处理的自动抛竿切换
    pub fn take_pending_autocast(&mut self) -> Option<bool> {
        self.pending_autocast_toggle.take()
    }

    /// 加载纹理
    pub fn load_textures(&mut self) {
        // 装备对话框: Prguse[1340]
        if let Some(texture) = LibraryName::Prguse.get_texture(1340) {
            if let Some(tex) = texture.image {
                self.equip_bg_texture = Some(tex);
            }
        }

        // 状态对话框: Prguse[1341]
        if let Some(texture) = LibraryName::Prguse.get_texture(1341) {
            if let Some(tex) = texture.image {
                self.status_bg_texture = Some(tex);
                self.size = vec2(texture.width as f32, texture.height as f32);
            }
        }

        // 进度条: Prguse[1342=chance, 1349=progress]
        if let Some(texture) = LibraryName::Prguse.get_texture(1342) {
            self.chance_bar_texture = texture.image;
        }
        if let Some(texture) = LibraryName::Prguse.get_texture(1349) {
            self.progress_bar_texture = texture.image;
        }

        // 自动抛竿复选框: Prguse[1343/1344]
        if let Some(texture) = LibraryName::Prguse.get_texture(1343) {
            self.auto_cast_unchecked = texture.image;
        }
        if let Some(texture) = LibraryName::Prguse.get_texture(1344) {
            self.auto_cast_checked = texture.image;
        }
    }

    pub fn update_and_draw(&mut self) {
        if !self.visible {
            return;
        }

        let mouse_pos = vec2(mouse_position().0, mouse_position().1);

        // 拖拽
        let drag_area = Rect::new(self.position.x, self.position.y, self.size.x, 30.0);
        self.drag_helper.apply(drag_area, &mut self.position);

        // 根据状态绘制不同界面
        if self.fishing_state == FishingState::Idle {
            self.draw_equip_dialog(mouse_pos);
        } else {
            self.draw_status_dialog(mouse_pos);
        }
    }

    fn draw_equip_dialog(&mut self, mouse_pos: Vec2) {
        // 背景
        if let Some(tex) = &self.equip_bg_texture {
            draw_texture_ex(
                tex,
                self.position.x,
                self.position.y,
                WHITE,
                DrawTextureParams::default(),
            );
        }

        // 标题 - 鱼竿名称
        if !self.rod_name.is_empty() {
            draw_text_cn(
                &self.rod_name,
                self.position.x + 60.0,
                self.position.y + 12.0,
                14.0,
                Color::from_rgba(255, 255, 200, 255),
            );
        }

        // 5 个装备槽
        let slot_positions: [(f32, f32); 5] = [
            (17.0, 203.0),  // Hook
            (17.0, 241.0),  // Float
            (57.0, 241.0),  // Bait
            (97.0, 241.0),  // Finder
            (137.0, 241.0), // Reel
        ];

        for (i, (sx, sy)) in slot_positions.iter().enumerate() {
            let slot_rect = Rect::new(self.position.x + sx, self.position.y + sy, 36.0, 36.0);
            let is_hovered = slot_rect.contains(mouse_pos);

            // 槽位边框
            let border_color = if is_hovered {
                Color::from_rgba(200, 200, 100, 255)
            } else {
                Color::from_rgba(100, 100, 100, 200)
            };
            draw_rectangle_lines(
                slot_rect.x,
                slot_rect.y,
                slot_rect.w,
                slot_rect.h,
                1.0,
                border_color,
            );

            // 槽位标签
            let slot = match i {
                0 => FishingSlot::Hook,
                1 => FishingSlot::Float,
                2 => FishingSlot::Bait,
                3 => FishingSlot::Finder,
                _ => FishingSlot::Reel,
            };
            draw_text_cn(
                slot.label(),
                slot_rect.x + 2.0,
                slot_rect.y + 40.0,
                8.0,
                GRAY,
            );
        }

        // 关闭按钮
        self.draw_close_button(mouse_pos);
    }

    fn draw_status_dialog(&mut self, mouse_pos: Vec2) {
        // 背景
        if let Some(tex) = &self.status_bg_texture {
            draw_texture_ex(
                tex,
                self.position.x,
                self.position.y,
                WHITE,
                DrawTextureParams::default(),
            );
        }

        let bar_x = self.position.x + 20.0;
        let bar_y = self.position.y + 20.0;
        let bar_w = 216.0;
        let bar_h = 14.0;

        // 成功率条
        self.draw_bar(
            bar_x,
            bar_y,
            bar_w,
            bar_h,
            self.chance_percent,
            Color::from_rgba(60, 200, 60, 200),
            "几率",
        );

        // 进度条
        let progress_y = bar_y + 24.0;
        self.draw_bar(
            bar_x,
            progress_y,
            bar_w,
            bar_h,
            self.progress_percent,
            Color::from_rgba(60, 60, 200, 200),
            "进度",
        );

        // 自动抛竿复选框
        let checkbox_x = self.position.x + 20.0;
        let checkbox_y = self.position.y + 80.0;
        let checkbox_rect = Rect::new(checkbox_x, checkbox_y, 20.0, 20.0);
        let is_checkbox_hovered = checkbox_rect.contains(mouse_pos);

        let checkbox_tex = if self.auto_cast_enabled {
            &self.auto_cast_checked
        } else {
            &self.auto_cast_unchecked
        };
        if let Some(tex) = checkbox_tex {
            draw_texture_ex(
                tex,
                checkbox_x,
                checkbox_y,
                WHITE,
                DrawTextureParams::default(),
            );
        } else {
            let check_color = if self.auto_cast_enabled {
                Color::from_rgba(60, 200, 60, 255)
            } else {
                Color::from_rgba(100, 100, 100, 255)
            };
            draw_rectangle(checkbox_x, checkbox_y, 16.0, 16.0, check_color);
            draw_rectangle_lines(checkbox_x, checkbox_y, 16.0, 16.0, 1.0, WHITE);
        }

        draw_text_cn("自动抛竿", checkbox_x + 24.0, checkbox_y + 4.0, 12.0, WHITE);

        if is_checkbox_hovered && is_mouse_button_pressed(MouseButton::Left) {
            self.auto_cast_enabled = !self.auto_cast_enabled;
            self.pending_autocast_toggle = Some(self.auto_cast_enabled);
        }

        // 收线按钮（咬钩或收线状态时显示）
        if self.fishing_state == FishingState::Biting || self.fishing_state == FishingState::Reeling
        {
            let fish_btn_x = self.position.x + 80.0;
            let fish_btn_y = self.position.y + 120.0;
            let fish_btn_w = 100.0;
            let fish_btn_h = 30.0;
            let fish_rect = Rect::new(fish_btn_x, fish_btn_y, fish_btn_w, fish_btn_h);
            let is_fish_hovered = fish_rect.contains(mouse_pos);
            let is_fish_pressed = is_fish_hovered && is_mouse_button_down(MouseButton::Left);

            let fish_color = if is_fish_pressed {
                Color::from_rgba(120, 140, 80, 255)
            } else if is_fish_hovered {
                Color::from_rgba(100, 120, 60, 255)
            } else {
                Color::from_rgba(80, 100, 40, 255)
            };
            draw_rectangle(fish_btn_x, fish_btn_y, fish_btn_w, fish_btn_h, fish_color);
            draw_rectangle_lines(
                fish_btn_x,
                fish_btn_y,
                fish_btn_w,
                fish_btn_h,
                2.0,
                Color::from_rgba(200, 200, 100, 255),
            );
            draw_text_cn(
                "收线!",
                fish_btn_x + 25.0,
                fish_btn_y + 18.0,
                14.0,
                Color::from_rgba(255, 255, 200, 255),
            );
        }

        // 关闭按钮
        self.draw_close_button(mouse_pos);
    }

    fn draw_bar(&self, x: f32, y: f32, w: f32, h: f32, percent: f32, color: Color, label: &str) {
        let pct = percent.clamp(0.0, 1.0);
        let draw_w = w * pct;

        draw_rectangle(x, y, w, h, Color::from_rgba(40, 40, 40, 200));
        if draw_w > 0.0 {
            draw_rectangle(x, y, draw_w, h, color);
        }
        draw_rectangle_lines(x, y, w, h, 1.0, Color::from_rgba(100, 100, 100, 200));

        draw_text_cn(label, x - 26.0, y + 10.0, 10.0, WHITE);
    }

    fn draw_close_button(&mut self, mouse_pos: Vec2) {
        let btn_x = self.position.x + self.size.x - 30.0;
        let btn_y = self.position.y + 4.0;
        let btn_rect = Rect::new(btn_x, btn_y, 24.0, 24.0);
        let is_hovered = btn_rect.contains(mouse_pos);
        let is_pressed = is_hovered && is_mouse_button_down(MouseButton::Left);

        let close_color = if is_pressed {
            Color::from_rgba(200, 60, 60, 255)
        } else if is_hovered {
            Color::from_rgba(180, 40, 40, 255)
        } else {
            Color::from_rgba(150, 40, 40, 200)
        };
        draw_rectangle(btn_x, btn_y, 20.0, 20.0, close_color);
        draw_text_cn("X", btn_x + 6.0, btn_y + 14.0, 12.0, WHITE);

        if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
            self.close();
        }
    }
}
