// ============================================================================
// BuffDialogHybrid - 增益/Buff 显示对话框（纯 Native 版本）
// ============================================================================
//
// C# 参考：Client/MirScenes/Dialogs/BuffDialog.cs (899 行)
// - 屏幕右上角固定位置，不可拖拽
// - Buff 图标水平排列，10 个换行
// - 展开/折叠模式
// - 自动淡入淡出
//
// PoisonBuffDialog 结构类似，使用 Prguse2[40] 背景
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;

/// 单个 Buff 数据
#[derive(Debug, Clone)]
pub struct BuffEntry {
    pub buff_type: u32,
    pub icon_index: u32,
    pub name: String,
    pub remaining_secs: f32,
    pub is_paused: bool,
    pub caster: String,
}

pub struct BuffDialogHybrid {
    position: Vec2,
    visible: bool,
    expanded: bool,
    opacity: f32,
    target_opacity: f32,
    buffs: Vec<BuffEntry>,
    bg_textures: Vec<Option<Texture2D>>,
    expand_texture: Option<Texture2D>,
    hover_expand_texture: Option<Texture2D>,
    pressed_expand_texture: Option<Texture2D>,
}

impl Default for BuffDialogHybrid {
    fn default() -> Self {
        Self::new()
    }
}

impl BuffDialogHybrid {
    const ICON_SIZE: f32 = 32.0;
    const ICON_PADDING: f32 = 4.0;
    const ICONS_PER_ROW: usize = 10;

    pub fn new() -> Self {
        Self {
            position: Vec2::ZERO,
            visible: true,
            expanded: false,
            opacity: 0.0,
            target_opacity: 0.0,
            buffs: Vec::new(),
            bg_textures: vec![None; 14],
            expand_texture: None,
            hover_expand_texture: None,
            pressed_expand_texture: None,
        }
    }

    pub fn close(&mut self) {
        self.visible = false;
    }

    pub fn open(&mut self) {
        self.visible = true;
    }

    pub fn set_screen_position(&mut self, screen_w: f32) {
        self.position = Vec2::new(screen_w - 170.0, 0.0);
    }

    pub fn is_visible(&self) -> bool {
        self.visible && !self.buffs.is_empty()
    }

    pub fn contains(&self, point: Vec2) -> bool {
        if !self.is_visible() {
            return false;
        }
        let rows = if self.expanded {
            self.buffs.len().div_ceil(Self::ICONS_PER_ROW)
        } else {
            1
        };
        let h = 34.0 + rows as f32 * (Self::ICON_SIZE + Self::ICON_PADDING);
        Rect::new(self.position.x, self.position.y, 170.0, h).contains(point)
    }

    /// 添加或更新 Buff
    pub fn add_buff(&mut self, entry: BuffEntry) {
        if let Some(existing) = self.buffs.iter_mut().find(|b| b.buff_type == entry.buff_type) {
            *existing = entry;
        } else {
            self.buffs.push(entry);
        }
        self.target_opacity = 1.0;
    }

    /// 移除 Buff
    pub fn remove_buff(&mut self, buff_type: u32) {
        self.buffs.retain(|b| b.buff_type != buff_type);
        if self.buffs.is_empty() {
            self.target_opacity = 0.0;
        }
    }

    /// 暂停/恢复 Buff
    pub fn set_buff_paused(&mut self, buff_type: u32, paused: bool) {
        if let Some(buff) = self.buffs.iter_mut().find(|b| b.buff_type == buff_type) {
            buff.is_paused = paused;
        }
    }

    /// 加载纹理
    pub fn load_textures(&mut self) {
        // 动态背景: Prguse2[20-33]
        for i in 20..=33 {
            if let Some(texture) = LibraryName::Prguse2.get_texture(i) {
                if let Some(tex) = texture.image {
                    if i - 20 < self.bg_textures.len() {
                        self.bg_textures[i - 20] = Some(tex);
                    }
                }
            }
        }

        // 展开/折叠按钮: Prguse2[7/8/9]
        if let Some(texture) = LibraryName::Prguse2.get_texture(7) {
            self.expand_texture = texture.image;
        }
        if let Some(texture) = LibraryName::Prguse2.get_texture(8) {
            self.hover_expand_texture = texture.image;
        }
        if let Some(texture) = LibraryName::Prguse2.get_texture(9) {
            self.pressed_expand_texture = texture.image;
        }
    }

    pub fn update_and_draw(&mut self, dt: f32, screen_w: f32) {
        self.set_screen_position(screen_w);

        // 自动淡入淡出
        let mouse_pos = vec2(mouse_position().0, mouse_position().1);
        if self.contains(mouse_pos) {
            self.target_opacity = 1.0;
        } else if self.expanded {
            self.target_opacity = 0.6;
        }

        // 平滑过渡
        let rate = 5.0;
        if self.opacity < self.target_opacity {
            self.opacity = (self.opacity + dt * rate).min(self.target_opacity);
        } else if self.opacity > self.target_opacity {
            self.opacity = (self.opacity - dt * rate).max(self.target_opacity);
        }

        if !self.visible || self.buffs.is_empty() {
            return;
        }

        let alpha = (self.opacity * 255.0) as u8;
        let tint = Color::new(1.0, 1.0, 1.0, self.opacity);

        // 绘制背景
        let bg_count = self.buffs.len().min(14);
        if let Some(tex) = &self.bg_textures[bg_count.saturating_sub(1)] {
            draw_texture_ex(tex, self.position.x, self.position.y, tint, DrawTextureParams::default());
        }

        // 展开/折叠按钮
        let expand_x = self.position.x + 148.0;
        let expand_y = self.position.y + 2.0;
        let expand_rect = Rect::new(expand_x, expand_y, 20.0, 20.0);
        let is_expand_hovered = expand_rect.contains(mouse_pos);
        let is_expand_pressed = is_expand_hovered && is_mouse_button_down(MouseButton::Left);

        let expand_tex = if is_expand_pressed {
            self.pressed_expand_texture.as_ref().or(self.expand_texture.as_ref())
        } else if is_expand_hovered {
            self.hover_expand_texture.as_ref().or(self.expand_texture.as_ref())
        } else {
            self.expand_texture.as_ref()
        };
        if let Some(tex) = expand_tex {
            draw_texture_ex(tex, expand_x, expand_y, tint, DrawTextureParams::default());
        }

        if is_expand_hovered && is_mouse_button_pressed(MouseButton::Left) {
            self.expanded = !self.expanded;
        }

        // 绘制 Buff 图标
        let start_y = self.position.y + 36.0;
        let max_icons = if self.expanded { self.buffs.len() } else { 1 };

        for (i, buff) in self.buffs.iter().enumerate().take(max_icons) {
            let col = i % Self::ICONS_PER_ROW;
            let row = i / Self::ICONS_PER_ROW;
            let icon_x = self.position.x + 8.0 + col as f32 * (Self::ICON_SIZE + Self::ICON_PADDING);
            let icon_y = start_y + row as f32 * (Self::ICON_SIZE + Self::ICON_PADDING);

            // 获取图标纹理
            let _icon_idx = self.resolve_buff_icon(buff.icon_index);
            let is_flashing = buff.remaining_secs > 0.0 && buff.remaining_secs < 5.0
                && (get_time() as u32).is_multiple_of(2);

            let icon_alpha = if is_flashing {
                (self.opacity * 0.5).max(0.1)
            } else {
                self.opacity
            };
            let icon_tint = Color::new(1.0, 1.0, 1.0, icon_alpha);

            // 绘制图标边框
            draw_rectangle_lines(icon_x, icon_y, Self::ICON_SIZE, Self::ICON_SIZE, 1.0,
                Color::from_rgba(100, 100, 100, alpha));

            // 尝试加载并绘制图标
            if let Some(texture) = LibraryName::BuffIcon.get_texture(buff.icon_index as usize) {
                if let Some(tex) = &texture.image {
                    draw_texture_ex(tex, icon_x + 2.0, icon_y + 2.0, icon_tint,
                        DrawTextureParams {
                            dest_size: Some(Vec2::new(Self::ICON_SIZE - 4.0, Self::ICON_SIZE - 4.0)),
                            ..Default::default()
                        });
                } else if is_flashing {
                    draw_text_cn("?", icon_x + 12.0, icon_y + 20.0, 16.0,
                        Color::from_rgba(255, 200, 100, alpha));
                }
            } else if is_flashing {
                draw_text_cn("?", icon_x + 12.0, icon_y + 20.0, 16.0,
                    Color::from_rgba(255, 200, 100, alpha));
            }

            // 暂停标记
            if buff.is_paused {
                draw_text_cn("⏸", icon_x + 10.0, icon_y + 10.0, 10.0,
                    Color::from_rgba(200, 200, 200, alpha));
            }

            // 悬停时显示 tooltip
            let buff_rect = Rect::new(icon_x, icon_y, Self::ICON_SIZE, Self::ICON_SIZE);
            if buff_rect.contains(mouse_pos) {
                self.draw_tooltip(icon_x, icon_y + Self::ICON_SIZE + 4.0, buff, alpha);
            }
        }

        // 折叠模式下显示剩余数量
        if !self.expanded && self.buffs.len() > 1 {
            draw_text_cn(
                &format!("+{}", self.buffs.len() - 1),
                self.position.x + 44.0,
                self.position.y + 8.0,
                10.0,
                Color::from_rgba(255, 255, 100, alpha),
            );
        }
    }

    fn draw_tooltip(&self, x: f32, y: f32, buff: &BuffEntry, alpha: u8) {
        let text = if buff.remaining_secs > 0.0 {
            format!("{}\n{:.0}s", buff.name, buff.remaining_secs)
        } else {
            buff.name.clone()
        };
        draw_text_cn(&text, x, y, 10.0, Color::from_rgba(255, 255, 255, alpha));
    }

    /// 解析 Buff 图标索引到实际纹理
    fn resolve_buff_icon(&self, icon_index: u32) -> usize {
        if icon_index >= 20000 {
            (icon_index - 20000) as usize
        } else if icon_index >= 10000 {
            (icon_index - 10000) as usize
        } else {
            icon_index as usize
        }
    }
}
