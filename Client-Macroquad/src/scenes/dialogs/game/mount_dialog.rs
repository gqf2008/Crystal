// ============================================================================
// MountDialogHybrid - 坐骑对话框
// ============================================================================
//
// C# 参考：Client/MirScenes/Dialogs/MountDialog.cs (263 行)
// - 背景：Prguse[960]
// - 标题：Title[20] at (18, 9)
// - 关闭按钮：Title[193/194/195] at (200, 256)
// - 坐骑列表、骑乘/下马切换、坐骑信息显示
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use super::native_ui_utils::DragHelper;

/// 坐骑信息
#[derive(Debug, Clone)]
pub struct MountEntry {
    pub name: String,
    pub mount_type: i16,
    pub owned: bool,
}

/// 坐骑对话框动作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountDialogAction {
    None,
    Ride,
    Dismount,
    SelectMount(usize),
}

pub struct MountDialogHybrid {
    position: Vec2,
    visible: bool,
    size: Vec2,
    mounts: Vec<MountEntry>,
    selected_mount: Option<usize>,
    is_riding: bool,
    scroll_offset: f32,
    bg_texture: Option<Texture2D>,
    title_texture: Option<Texture2D>,
    close_button_textures: [Option<Texture2D>; 3],
    drag_helper: DragHelper,
    pending_action: MountDialogAction,
    /// 本地玩家对象 ID（用于 Ride/Dismount 发包）
    local_object_id: u32,
}

impl MountDialogHybrid {
    const MOUNT_START_Y: f32 = 50.0;
    const MOUNT_ITEM_H: f32 = 22.0;
    const BUTTON_Y: f32 = 210.0;

    pub fn new() -> Self {
        Self {
            position: vec2(300.0, 100.0),
            visible: false,
            size: vec2(280.0, 290.0),
            mounts: Vec::new(),
            selected_mount: None,
            is_riding: false,
            scroll_offset: 0.0,
            bg_texture: None,
            title_texture: None,
            close_button_textures: [None, None, None],
            drag_helper: DragHelper::new(),
            pending_action: MountDialogAction::None,
            local_object_id: 0,
        }
    }

    pub fn open(&mut self) {
        self.visible = true;
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.selected_mount = None;
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

    /// 更新坐骑状态
    pub fn update_mount_state(&mut self, mount_type: i16, riding: bool) {
        self.is_riding = riding;
        // 尝试匹配已有的坐骑类型
        if riding && mount_type >= 0 {
            if let Some(idx) = self.mounts.iter().position(|m| m.mount_type == mount_type) {
                self.selected_mount = Some(idx);
            }
        }
    }

    /// 更新坐骑列表
    pub fn update_mounts(&mut self, mounts: Vec<MountEntry>) {
        self.mounts = mounts;
    }

    /// 添加坐骑
    pub fn add_mount(&mut self, mount: MountEntry) {
        self.mounts.push(mount);
    }

    /// 设置本地玩家对象 ID
    pub fn set_local_object_id(&mut self, id: u32) {
        self.local_object_id = id;
    }

    /// 获取待处理动作
    pub fn take_action(&mut self) -> MountDialogAction {
        std::mem::replace(&mut self.pending_action, MountDialogAction::None)
    }

    /// 获取当前选中的坐骑类型
    pub fn get_selected_mount_type(&self) -> Option<i16> {
        self.selected_mount
            .and_then(|i| self.mounts.get(i))
            .map(|m| m.mount_type)
    }

    /// 加载纹理
    pub fn load_textures(&mut self) {
        // 背景纹理 - Prguse[960]
        if let Some(texture) = LibraryName::Prguse.get_texture(960) {
            self.size = vec2(texture.width as f32, texture.height as f32);
            if let Some(tex) = texture.image {
                self.bg_texture = Some(tex);
            }
        }

        // 标题纹理 - Title[20]
        if let Some(texture) = LibraryName::Title.get_texture(20) {
            if let Some(tex) = texture.image {
                self.title_texture = Some(tex);
            }
        }

        // 关闭按钮 - Title[193/194/195]
        for (i, idx) in [193, 194, 195].iter().enumerate() {
            if let Some(texture) = LibraryName::Title.get_texture(*idx) {
                if let Some(tex) = texture.image {
                    self.close_button_textures[i] = Some(tex);
                }
            }
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

        // 绘制背景
        self.draw_background();

        // 绘制坐骑列表
        self.draw_mount_list(mouse_pos);

        // 绘制按钮
        self.draw_buttons(mouse_pos);

        // 绘制关闭按钮
        self.draw_close_button(mouse_pos);
    }

    fn draw_background(&self) {
        if let Some(texture) = &self.bg_texture {
            draw_texture_ex(
                texture,
                self.position.x,
                self.position.y,
                WHITE,
                DrawTextureParams::default(),
            );
        }

        if let Some(title_tex) = &self.title_texture {
            draw_texture_ex(
                title_tex,
                self.position.x + 18.0,
                self.position.y + 9.0,
                WHITE,
                DrawTextureParams::default(),
            );
        }

        // 骑乘状态
        let status_text = if self.is_riding { "骑乘中" } else { "未骑乘" };
        let status_color = if self.is_riding {
            Color::from_rgba(100, 255, 100, 255)
        } else {
            GRAY
        };
        draw_text_cn(
            status_text,
            self.position.x + 15.0,
            self.position.y + 30.0,
            12.0,
            status_color,
        );
    }

    fn draw_mount_list(&mut self, mouse_pos: Vec2) {
        let list_x = self.position.x + 10.0;
        let list_w = self.size.x - 20.0;
        let list_top = self.position.y + Self::MOUNT_START_Y;

        // 鼠标滚轮
        let list_bottom = self.position.y + Self::BUTTON_Y - 5.0;
        let list_h = (list_bottom - list_top).max(0.0);
        let list_rect = Rect::new(list_x, list_top, list_w, list_h);

        if list_rect.contains(mouse_pos) {
            let wheel = mouse_wheel().1;
            if wheel != 0.0 {
                self.scroll_offset = (self.scroll_offset - wheel * 20.0).max(0.0);
            }
        }

        let mut y = list_top - self.scroll_offset;
        let mut clicked: Option<usize> = None;

        for (i, mount) in self.mounts.iter().enumerate() {
            let item_rect = Rect::new(list_x + 5.0, y, list_w - 10.0, Self::MOUNT_ITEM_H);
            let item_visible = item_rect.y + item_rect.h > list_top && item_rect.y < list_bottom;

            if item_visible {
                let is_selected = self.selected_mount == Some(i);
                let is_hovered = item_rect.contains(mouse_pos);

                // 背景
                if is_selected || is_hovered {
                    let color = if is_selected {
                        Color::from_rgba(60, 80, 100, 150)
                    } else {
                        Color::from_rgba(50, 50, 60, 100)
                    };
                    draw_rectangle(item_rect.x, item_rect.y, item_rect.w, item_rect.h, color);
                }

                // 骑乘标记
                if self.is_riding && self.selected_mount == Some(i) {
                    draw_text_cn(
                        "[骑]",
                        item_rect.x,
                        item_rect.y + 15.0,
                        10.0,
                        Color::from_rgba(100, 255, 100, 255),
                    );
                }

                // 名称
                let name_color = if mount.owned { WHITE } else { GRAY };
                draw_text_cn(
                    &mount.name,
                    item_rect.x + 30.0,
                    item_rect.y + 15.0,
                    12.0,
                    name_color,
                );

                // 类型 ID
                let type_text = format!("类型:{}", mount.mount_type);
                draw_text_cn(&type_text, item_rect.x + 160.0, item_rect.y + 15.0, 10.0, GRAY);

                // 点击检测
                if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
                    clicked = Some(i);
                }
            }

            y += Self::MOUNT_ITEM_H;
        }

        if let Some(idx) = clicked {
            self.selected_mount = Some(idx);
            self.pending_action = MountDialogAction::SelectMount(idx);
        }

        // 限制滚动
        let content_h = self.mounts.len() as f32 * Self::MOUNT_ITEM_H;
        let max_scroll = (content_h - list_h).max(0.0);
        if self.scroll_offset > max_scroll {
            self.scroll_offset = max_scroll;
        }
    }

    fn draw_buttons(&mut self, mouse_pos: Vec2) {
        let btn_y = self.position.y + Self::BUTTON_Y;
        let btn_w = 80.0;
        let btn_h = 25.0;
        let btn_spacing = 10.0;

        let buttons: Vec<(&str, MountDialogAction)> = if self.is_riding {
            vec![("下马", MountDialogAction::Dismount)]
        } else {
            vec![("骑乘", MountDialogAction::Ride)]
        };

        let total_w = buttons.len() as f32 * (btn_w + btn_spacing) - btn_spacing;
        let start_x = self.position.x + (self.size.x - total_w) / 2.0;

        for (i, (label, action)) in buttons.iter().enumerate() {
            let btn_x = start_x + i as f32 * (btn_w + btn_spacing);
            let btn_rect = Rect::new(btn_x, btn_y, btn_w, btn_h);

            let is_hovered = btn_rect.contains(mouse_pos);
            let is_pressed = is_hovered && is_mouse_button_down(MouseButton::Left);

            let btn_color = if is_pressed {
                Color::from_rgba(100, 120, 140, 255)
            } else if is_hovered {
                Color::from_rgba(80, 100, 120, 255)
            } else {
                Color::from_rgba(60, 70, 80, 255)
            };
            draw_rectangle(btn_x, btn_y, btn_w, btn_h, btn_color);
            draw_rectangle_lines(btn_x, btn_y, btn_w, btn_h, 1.0, Color::from_rgba(100, 100, 120, 255));

            draw_text_cn(label, btn_x + 20.0, btn_y + 16.0, 12.0, WHITE);

            if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
                self.pending_action = *action;
            }
        }
    }

    fn draw_close_button(&mut self, mouse_pos: Vec2) {
        let btn_x = self.position.x + 200.0;
        let btn_y = self.position.y + 256.0;

        if let Some(normal) = &self.close_button_textures[0] {
            let btn_rect = Rect::new(btn_x, btn_y, normal.width(), normal.height());
            let is_hovered = btn_rect.contains(mouse_pos);
            let is_pressed = is_hovered && is_mouse_button_down(MouseButton::Left);

            let texture = if is_pressed {
                self.close_button_textures[2].as_ref().unwrap_or(normal)
            } else if is_hovered {
                self.close_button_textures[1].as_ref().unwrap_or(normal)
            } else {
                normal
            };

            draw_texture_ex(texture, btn_x, btn_y, WHITE, DrawTextureParams::default());

            if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
                self.close();
            }
        }
    }
}
