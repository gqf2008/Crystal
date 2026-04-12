// ============================================================================
// InspectDialogHybrid - 查看玩家装备对话框
// ============================================================================
// 显示目标玩家的装备和属性信息
// ============================================================================

use macroquad::prelude::*;
use crate::ui::text_renderer::draw_text_cn;
use super::native_ui_utils::*;

#[derive(Debug)]
pub enum InspectDialogAction {
    None,
    Close,
}

#[derive(Debug, Clone)]
pub struct InspectEquipSlot {
    pub slot_name: String,
    pub item_name: String,
}

pub struct InspectDialogHybrid {
    position: Vec2,
    size: Vec2,
    visible: bool,
    drag_helper: DragHelper,

    target_name: String,
    target_level: u16,
    target_class: String,
    equipment: Vec<InspectEquipSlot>,

    hovered_close: bool,

    close_btn: ButtonTextures,
    pending_action: InspectDialogAction,
}

impl Default for InspectDialogHybrid {
    fn default() -> Self { Self::new() }
}

impl InspectDialogHybrid {
    const WIDTH: f32 = 280.0;
    const HEIGHT: f32 = 380.0;
    const SLOT_H: f32 = 26.0;

    pub fn new() -> Self {
        Self {
            position: vec2(250.0, 140.0),
            size: vec2(Self::WIDTH, Self::HEIGHT),
            visible: false,
            drag_helper: DragHelper::new(),
            target_name: String::new(),
            target_level: 0,
            target_class: String::new(),
            equipment: Vec::new(),
            hovered_close: false,
            close_btn: ButtonTextures::new(),
            pending_action: InspectDialogAction::None,
        }
    }

    pub fn open(&mut self, target_name: &str) {
        if !self.visible {
            self.visible = true;
            self.target_name = target_name.to_string();
        }
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.pending_action = InspectDialogAction::Close;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn contains(&self, pos: Vec2) -> bool {
        self.visible && Rect::new(self.position.x, self.position.y, self.size.x, self.size.y).contains(pos)
    }

    pub fn take_action(&mut self) -> InspectDialogAction {
        std::mem::replace(&mut self.pending_action, InspectDialogAction::None)
    }

    pub fn set_target_info(&mut self, name: &str, level: u16, class: &str, equipment: Vec<InspectEquipSlot>) {
        self.target_name = name.to_string();
        self.target_level = level;
        self.target_class = class.to_string();
        self.equipment = equipment;
    }

    pub fn update_and_draw(&mut self) {
        if !self.visible {
            return;
        }

        let mouse = mouse_pos();

        // 窗口拖动
        let drag_area = Rect::new(self.position.x, self.position.y, self.size.x - 24.0, 30.0);
        self.drag_helper.apply(drag_area, &mut self.position);

        // 关闭按钮
        self.hovered_close = Rect::new(self.position.x + self.size.x - 24.0, self.position.y + 4.0, 20.0, 20.0).contains(mouse);
        if is_mouse_button_pressed(MouseButton::Left) && self.hovered_close {
            self.close();
            return;
        }

        // 背景
        draw_rectangle(self.position.x, self.position.y, self.size.x, self.size.y, Color::from_rgba(30, 30, 40, 240));
        draw_rectangle_lines(self.position.x, self.position.y, self.size.x, self.size.y, 1.0, Color::from_rgba(100, 100, 120, 255));

        // 标题
        draw_text_cn("查看装备", self.position.x + 100.0, self.position.y + 8.0, 16.0, YELLOW);

        // 玩家信息
        let info_y = self.position.y + 30.0;
        draw_text_cn(&self.target_name, self.position.x + 20.0, info_y, 14.0, YELLOW);
        draw_text_cn(&format!("Lv.{}", self.target_level), self.position.x + 140.0, info_y, 14.0, WHITE);
        draw_text_cn(&self.target_class, self.position.x + 200.0, info_y, 14.0, WHITE);

        // 装备列表
        let list_y = info_y + 22.0;
        draw_rectangle_lines(self.position.x + 10.0, list_y, self.size.x - 20.0, self.size.y - (list_y - self.position.y) - 10.0, 1.0, Color::from_rgba(80, 80, 100, 255));

        for (i, slot) in self.equipment.iter().enumerate() {
            let y = list_y + 5.0 + i as f32 * Self::SLOT_H;
            if y + Self::SLOT_H > self.position.y + self.size.y - 10.0 {
                continue;
            }

            // 槽位背景
            let has_item = !slot.item_name.is_empty();
            let bg_color = if has_item {
                Color::from_rgba(50, 50, 60, 200)
            } else {
                Color::from_rgba(35, 35, 45, 150)
            };
            let slot_rect = Rect::new(self.position.x + 15.0, y, self.size.x - 30.0, Self::SLOT_H - 2.0);
            draw_rectangle(slot_rect.x, slot_rect.y, slot_rect.w, slot_rect.h, bg_color);

            // 槽位名称
            draw_text_cn(&slot.slot_name, slot_rect.x + 8.0, slot_rect.y + 6.0, 12.0, Color::from_rgba(180, 180, 200, 255));

            // 物品名称
            let item_color = if has_item { YELLOW } else { Color::from_rgba(100, 100, 100, 255) };
            draw_text_cn(&slot.item_name, slot_rect.x + 70.0, slot_rect.y + 6.0, 12.0, item_color);
        }

        // 关闭按钮
        if let Some(ref tex) = self.close_btn.textures[0] {
            draw_texture(tex, self.position.x + self.size.x - 22.0, self.position.y + 4.0, WHITE);
        }
    }

    pub fn load_textures(&mut self) {
        if let Some(tex) = crate::resources::LibraryName::Prguse2.get_texture(360).and_then(|i| i.image) {
            self.close_btn.textures[0] = Some(tex);
        }
    }
}
