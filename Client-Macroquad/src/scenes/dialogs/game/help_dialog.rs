// ============================================================================
// HelpDialogHybrid - 帮助文档对话框
// ============================================================================
// 显示游戏帮助信息（快捷键、操作说明等）
// ============================================================================

use macroquad::prelude::*;
use crate::ui::text_renderer::draw_text_cn;
use super::native_ui_utils::*;

#[derive(Debug)]
pub enum HelpDialogAction {
    None,
}

pub struct HelpDialogHybrid {
    position: Vec2,
    size: Vec2,
    visible: bool,
    drag_helper: DragHelper,

    scroll_offset: f32,
    hovered_close: bool,

    close_btn: ButtonTextures,
    pending_action: HelpDialogAction,
}

impl Default for HelpDialogHybrid {
    fn default() -> Self { Self::new() }
}

impl HelpDialogHybrid {
    const WIDTH: f32 = 400.0;
    const HEIGHT: f32 = 420.0;
    const LINE_H: f32 = 22.0;

    pub fn new() -> Self {
        Self {
            position: vec2(180.0, 120.0),
            size: vec2(Self::WIDTH, Self::HEIGHT),
            visible: false,
            drag_helper: DragHelper::new(),
            scroll_offset: 0.0,
            hovered_close: false,
            close_btn: ButtonTextures::new(),
            pending_action: HelpDialogAction::None,
        }
    }

    pub fn open(&mut self) {
        if !self.visible {
            self.visible = true;
            self.scroll_offset = 0.0;
        }
    }

    pub fn close(&mut self) {
        self.visible = false;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn contains(&self, pos: Vec2) -> bool {
        self.visible && Rect::new(self.position.x, self.position.y, self.size.x, self.size.y).contains(pos)
    }

    pub fn take_action(&mut self) -> HelpDialogAction {
        std::mem::replace(&mut self.pending_action, HelpDialogAction::None)
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
        draw_rectangle(self.position.x, self.position.y, self.size.x, self.size.y, Color::from_rgba(25, 25, 35, 245));
        draw_rectangle_lines(self.position.x, self.position.y, self.size.x, self.size.y, 1.0, Color::from_rgba(100, 100, 120, 255));

        // 标题
        draw_text_cn("操作帮助", self.position.x + 150.0, self.position.y + 8.0, 16.0, YELLOW);

        // 内容区域
        let content_y = self.position.y + 35.0;
        let content_h = self.size.y - (content_y - self.position.y) - 10.0;
        let content_rect = Rect::new(self.position.x + 10.0, content_y, self.size.x - 20.0, content_h);
        draw_rectangle_lines(content_rect.x, content_rect.y, content_rect.w, content_rect.h, 1.0, Color::from_rgba(80, 80, 100, 255));

        // 滚动
        let total_lines = Self::help_lines().len() as f32;
        let visible_lines = content_h / Self::LINE_H;
        let max_scroll = (total_lines - visible_lines).max(0.0) * Self::LINE_H;
        if max_scroll > 0.0 && content_rect.contains(mouse) {
            let wheel = mouse_wheel().1;
            if wheel != 0.0 {
                self.scroll_offset = (self.scroll_offset - wheel * 30.0).clamp(0.0, max_scroll);
            }
        }

        // 绘制帮助内容
        let lines = Self::help_lines();
        for (i, (key, desc)) in lines.iter().enumerate() {
            let y = content_y + 5.0 + i as f32 * Self::LINE_H - self.scroll_offset;
            if y + Self::LINE_H < content_y || y > content_y + content_h {
                continue;
            }
            draw_text_cn(key, content_rect.x + 10.0, y + 2.0, 13.0, YELLOW);
            draw_text_cn(desc, content_rect.x + 120.0, y + 2.0, 13.0, WHITE);
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

    fn help_lines() -> &'static [(&'static str, &'static str)] {
        &[
            ("I / C", "打开/关闭背包"),
            ("Tab", "打开/关闭角色面板"),
            ("Enter", "激活聊天输入"),
            ("M", "打开/关闭小地图"),
            ("H", "打开/关闭帮助"),
            ("G", "打开/关闭组队"),
            ("F", "打开/关闭好友"),
            ("B", "打开/关闭公会"),
            ("1/2/3", "背包标签页切换"),
            ("WASD / 方向键", "角色移动"),
            ("左键点击物品", "拾取/选择"),
            ("右键点击物品", "使用/发送快捷栏"),
            ("拖拽物品", "交换位置"),
            ("拖出窗口", "丢弃物品"),
            ("双击物品", "使用物品"),
            ("鼠标滚轮", "滚动列表"),
            ("ESC", "关闭弹窗/打开菜单"),
            ("Shift + 左键", "强制攻击"),
            ("Ctrl + 左键", "拾取物品"),
            ("Space", "使用快捷技能"),
        ]
    }
}
