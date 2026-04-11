// ============================================================================
// SocketDialogHybrid - 宝石镶嵌对话框
// ============================================================================
//
// C# 参考: Client/MIRScenes/Dialogs/SocketDialog.cs (~123 行)
// - 显示装备的宝石孔位
// - 允许插入/取出宝石
// - 本地 UI，无网络消息
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use super::native_ui_utils::DragHelper;

/// 宝石孔位
#[derive(Debug, Clone)]
pub struct SocketSlot {
    pub index: usize,
    pub has_gem: bool,
    pub gem_name: String,
    pub gem_type: u32,
}

/// 对话框动作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketAction {
    None,
    InsertGem(usize),
    RemoveGem(usize),
    Close,
}

pub struct SocketDialogHybrid {
    position: Vec2,
    visible: bool,
    size: Vec2,
    sockets: Vec<SocketSlot>,
    item_name: String,
    drag_helper: DragHelper,
    pending_action: SocketAction,
    // 纹理
    bg_texture: Option<Texture2D>,
    close_texture: Option<Texture2D>,
    gem_texture: Option<Texture2D>,
    _empty_socket_texture: Option<Texture2D>,
}

impl Default for SocketDialogHybrid {
    fn default() -> Self {
        Self::new()
    }
}

impl SocketDialogHybrid {
    const SLOT_SIZE: f32 = 40.0;
    const SLOT_GAP: f32 = 8.0;
    const START_X: f32 = 20.0;
    const START_Y: f32 = 50.0;

    pub fn new() -> Self {
        Self {
            position: vec2(300.0, 150.0),
            visible: false,
            size: vec2(250.0, 200.0),
            sockets: Vec::new(),
            item_name: String::new(),
            drag_helper: DragHelper::new(),
            pending_action: SocketAction::None,
            bg_texture: None,
            close_texture: None,
            gem_texture: None,
            _empty_socket_texture: None,
        }
    }

    pub fn open(&mut self) {
        self.visible = true;
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.pending_action = SocketAction::None;
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

    /// 更新孔位数据
    pub fn update_sockets(&mut self, item_name: String, sockets: Vec<SocketSlot>) {
        self.item_name = item_name;
        self.sockets = sockets;
    }

    /// 获取待处理动作
    pub fn take_action(&mut self) -> SocketAction {
        std::mem::replace(&mut self.pending_action, SocketAction::None)
    }

    /// 加载纹理
    pub fn load_textures(&mut self) {
        // 背景: Title[468]
        if let Some(texture) = LibraryName::Title.get_texture(468) {
            if let Some(tex) = texture.image {
                self.bg_texture = Some(tex);
                self.size = vec2(texture.width as f32, texture.height as f32);
            }
        }

        // 关闭按钮: Prguse2[360]
        if let Some(texture) = LibraryName::Prguse2.get_texture(360) {
            self.close_texture = texture.image;
        }

        // 宝石: Prguse2[427]
        if let Some(texture) = LibraryName::Prguse2.get_texture(427) {
            self.gem_texture = texture.image;
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

        self.draw_background();
        self.draw_title(mouse_pos);
        self.draw_socket_slots(mouse_pos);
    }

    fn draw_background(&self) {
        if let Some(tex) = &self.bg_texture {
            draw_texture_ex(tex, self.position.x, self.position.y, WHITE, DrawTextureParams::default());
        } else {
            draw_rectangle(
                self.position.x, self.position.y, self.size.x, self.size.y,
                Color::from_rgba(30, 30, 40, 230),
            );
            draw_rectangle_lines(
                self.position.x, self.position.y, self.size.x, self.size.y,
                2.0, Color::from_rgba(80, 80, 100, 255),
            );
        }
    }

    fn draw_title(&mut self, mouse_pos: Vec2) {
        // 物品名称
        draw_text_cn(
            &self.item_name,
            self.position.x + 10.0,
            self.position.y + 18.0,
            12.0,
            Color::from_rgba(200, 200, 100, 255),
        );

        // 关闭按钮
        let close_x = self.position.x + self.size.x - 30.0;
        let close_y = self.position.y + 5.0;
        let close_rect = Rect::new(close_x, close_y, 24.0, 24.0);
        let is_close_hovered = close_rect.contains(mouse_pos);

        if let Some(tex) = &self.close_texture {
            draw_texture_ex(tex, close_x, close_y, WHITE, DrawTextureParams::default());
        } else {
            draw_rectangle(close_x, close_y, 24.0, 24.0, Color::from_rgba(150, 60, 60, 200));
            draw_text_cn("X", close_x + 8.0, close_y + 16.0, 12.0, WHITE);
        }

        if is_close_hovered && is_mouse_button_pressed(MouseButton::Left) {
            self.pending_action = SocketAction::Close;
            self.close();
        }
    }

    fn draw_socket_slots(&mut self, mouse_pos: Vec2) {
        let mut clicked_insert: Option<usize> = None;
        let mut clicked_remove: Option<usize> = None;

        for (i, socket) in self.sockets.iter().enumerate() {
            let col = i % 4;
            let row = i / 4;
            let slot_x = self.position.x + Self::START_X + col as f32 * (Self::SLOT_SIZE + Self::SLOT_GAP);
            let slot_y = self.position.y + Self::START_Y + row as f32 * (Self::SLOT_SIZE + Self::SLOT_GAP);
            let slot_rect = Rect::new(slot_x, slot_y, Self::SLOT_SIZE, Self::SLOT_SIZE);
            let is_hovered = slot_rect.contains(mouse_pos);

            // 孔位背景
            let bg_color = if socket.has_gem {
                Color::from_rgba(60, 40, 80, 200)
            } else {
                Color::from_rgba(40, 40, 40, 200)
            };
            draw_rectangle(slot_x, slot_y, Self::SLOT_SIZE, Self::SLOT_SIZE, bg_color);
            draw_rectangle_lines(slot_x, slot_y, Self::SLOT_SIZE, Self::SLOT_SIZE, 1.0,
                if is_hovered {
                    Color::from_rgba(200, 200, 100, 255)
                } else {
                    Color::from_rgba(80, 80, 80, 200)
                });

            if socket.has_gem {
                // 显示宝石
                if let Some(tex) = &self.gem_texture {
                    draw_texture_ex(tex, slot_x + 4.0, slot_y + 4.0, WHITE, DrawTextureParams {
                        dest_size: Some(vec2(Self::SLOT_SIZE - 8.0, Self::SLOT_SIZE - 8.0)),
                        ..Default::default()
                    });
                }
                // 宝石名称
                draw_text_cn(&socket.gem_name, slot_x + 2.0, slot_y + Self::SLOT_SIZE + 12.0, 8.0,
                    Color::from_rgba(180, 100, 255, 255));
            } else {
                // 空孔位标记
                draw_text_cn("空", slot_x + 12.0, slot_y + 24.0, 10.0,
                    Color::from_rgba(80, 80, 80, 200));
            }

            // 点击检测
            if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
                if socket.has_gem {
                    clicked_remove = Some(i);
                } else {
                    clicked_insert = Some(i);
                }
            }
        }

        if let Some(idx) = clicked_remove {
            self.pending_action = SocketAction::RemoveGem(idx);
        }
        if let Some(idx) = clicked_insert {
            self.pending_action = SocketAction::InsertGem(idx);
        }
    }
}
