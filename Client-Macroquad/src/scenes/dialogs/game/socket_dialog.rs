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
use mir2_shared::enums::AwakeType;

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
    InsertGem { item_unique_id: u64, position_idx: usize, awake_type: AwakeType },
    RemoveGem { item_unique_id: u64, position_idx: usize },
    Close,
}

/// 宝石选择子面板的状态。
///
/// 嵌入在 SocketDialog 中。当用户点击空孔位时,弹出此面板
/// 列出可用的 AwakeType 选项(Dc/Mc/Sc/Ac/Mac/HpMp)。
/// 选择后产生 `SocketAction::InsertGem` 带正确的 awake_type 发包。
#[derive(Debug, Clone)]
struct GemPickerState {
    /// 当前等待用户选择 AwakeType 的孔位索引
    pending_slot: Option<usize>,
}

impl GemPickerState {
    fn new() -> Self {
        Self { pending_slot: None }
    }

    fn open(&mut self, slot: usize) {
        self.pending_slot = Some(slot);
    }

    fn close(&mut self) {
        self.pending_slot = None;
    }

    fn is_open(&self) -> bool {
        self.pending_slot.is_some()
    }
}

pub struct SocketDialogHybrid {
    position: Vec2,
    visible: bool,
    size: Vec2,
    sockets: Vec<SocketSlot>,
    item_name: String,
    /// 当前操作物品的 unique_id（用于发包）
    item_unique_id: Option<u64>,
    drag_helper: DragHelper,
    pending_action: SocketAction,
    /// 宝石选择子面板状态(用户点击空孔位时弹出)
    gem_picker: GemPickerState,
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
            item_unique_id: None,
            drag_helper: DragHelper::new(),
            pending_action: SocketAction::None,
            gem_picker: GemPickerState::new(),
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
        self.gem_picker.close();
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
    pub fn update_sockets(&mut self, item_unique_id: Option<u64>, item_name: String, sockets: Vec<SocketSlot>) {
        self.item_unique_id = item_unique_id;
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

        // 宝石选择器覆盖在主对话框之上
        if self.gem_picker.is_open() {
            self.draw_gem_picker(mouse_pos);
        }
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
            if let Some(uid) = self.item_unique_id {
                self.pending_action = SocketAction::RemoveGem { item_unique_id: uid, position_idx: idx };
            }
        }
        if let Some(idx) = clicked_insert {
            // 弹出宝石类型选择器,等待用户选择 AwakeType (Dc/Mc/Sc/Ac/Mac/HpMp)
            self.gem_picker.open(idx);
        }
    }

    /// 绘制宝石选择子面板。
    ///
    /// 列出可用的 AwakeType 按钮,每行一个(共 6 项 + 取消)。
    /// 玩家点击后产生 `SocketAction::InsertGem { .., awake_type }`。
    /// 修复 `[[memory:feedback_socket_gem.md]]` 提到的:之前直接发包缺 AwakeType 字段。
    fn draw_gem_picker(&mut self, mouse_pos: Vec2) {
        let slot = match self.gem_picker.pending_slot {
            Some(s) => s,
            None => return,
        };

        // 子面板居中覆盖在主对话框上
        let picker_w = 200.0;
        let picker_h = 260.0;
        let picker_x = self.position.x + (self.size.x - picker_w) / 2.0;
        let picker_y = self.position.y + (self.size.y - picker_h) / 2.0;

        // 半透明遮罩
        let sw = screen_width();
        let sh = screen_height();
        draw_rectangle(0.0, 0.0, sw, sh, Color::from_rgba(0, 0, 0, 120));

        // 面板背景
        draw_rectangle(picker_x, picker_y, picker_w, picker_h, Color::from_rgba(20, 20, 30, 240));
        draw_rectangle_lines(picker_x, picker_y, picker_w, picker_h, 2.0, Color::from_rgba(120, 120, 140, 255));

        // 标题
        draw_text_cn(
            &format!("选择宝石类型 (孔位 {})", slot + 1),
            picker_x + 10.0,
            picker_y + 22.0,
            12.0,
            Color::from_rgba(220, 220, 100, 255),
        );

        // AwakeType 选项 (跳过 None = 0)
        let options: [(AwakeType, &str); 6] = [
            (AwakeType::Dc,   "DC - 物理攻击"),
            (AwakeType::Mc,   "MC - 魔法攻击"),
            (AwakeType::Sc,   "SC - 道术攻击"),
            (AwakeType::Ac,   "AC - 物理防御"),
            (AwakeType::Mac,  "MAC - 魔法防御"),
            (AwakeType::HpMp, "HP/MP - 生命/魔法"),
        ];

        let btn_h = 28.0;
        let btn_w = picker_w - 20.0;
        let btn_x = picker_x + 10.0;
        let mut clicked: Option<AwakeType> = None;

        for (i, (aw, label)) in options.iter().enumerate() {
            let by = picker_y + 40.0 + i as f32 * (btn_h + 4.0);
            let rect = Rect::new(btn_x, by, btn_w, btn_h);
            let hover = rect.contains(mouse_pos);
            draw_rectangle(
                btn_x, by, btn_w, btn_h,
                if hover {
                    Color::from_rgba(70, 70, 110, 240)
                } else {
                    Color::from_rgba(50, 50, 80, 220)
                },
            );
            draw_rectangle_lines(btn_x, by, btn_w, btn_h, 1.0, Color::from_rgba(100, 100, 130, 255));
            draw_text_cn(label, btn_x + 10.0, by + 8.0, 11.0, WHITE);

            if hover && is_mouse_button_pressed(MouseButton::Left) {
                clicked = Some(*aw);
            }
        }

        // 取消按钮
        let cancel_y = picker_y + 40.0 + options.len() as f32 * (btn_h + 4.0);
        let cancel_rect = Rect::new(btn_x, cancel_y, btn_w, btn_h);
        let cancel_hover = cancel_rect.contains(mouse_pos);
        draw_rectangle(
            btn_x, cancel_y, btn_w, btn_h,
            if cancel_hover {
                Color::from_rgba(120, 40, 40, 240)
            } else {
                Color::from_rgba(80, 30, 30, 220)
            },
        );
        draw_text_cn("取消", btn_x + 80.0, cancel_y + 8.0, 11.0, WHITE);

        if cancel_hover && is_mouse_button_pressed(MouseButton::Left) {
            self.gem_picker.close();
            return;
        }

        // 处理 AwakeType 选择 — 产生带 awake_type 的 InsertGem action
        if let Some(aw) = clicked {
            if let Some(uid) = self.item_unique_id {
                self.pending_action = SocketAction::InsertGem {
                    item_unique_id: uid,
                    position_idx: slot,
                    awake_type: aw,
                };
            }
            self.gem_picker.close();
        }
    }
}
