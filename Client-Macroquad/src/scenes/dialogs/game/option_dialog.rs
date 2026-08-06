// ============================================================================
// OptionDialogHybrid - 选项对话框（混合版本）
// ============================================================================
//
// 【C# 原版参考】
// - 背景: Title[411] (自动获取尺寸)
// - 关闭按钮: Prguse2[360/361/362] at (Size.Width - 26, 5)
// - 无标签页系统 - 只有一系列开/关按钮和音量条
//
// 按钮布局 (Prguse2):
// - SkillModeOn/Off: [451/454] at (159, 68) / (201, 68)
// - SkillBarOn/Off: [457/460] at (159, 93) / (201, 93)
// - EffectOn/Off: [457/460] at (159, 118) / (201, 118)
// - DropViewOn/Off: [457/460] at (159, 143) / (201, 143)
// - NameViewOn/Off: [457/460] at (159, 168) / (201, 168)
// - HPViewOn/Off: [463/466] at (159, 193) / (201, 193)
// - SoundBar: [468] at (159, 225)
// - VolumeBar: Prguse[20] at (155, 218)
// - MusicSoundBar: [468] at (159, 251)
// - MusicVolumeBar: Prguse[20] at (155, 244)
// - ObserveOn/Off: [457/460] at (159, 271) / (201, 271)
// - NewMoveOn/Off: Title[853/850] at (159, 296) / (201, 296)
//
// ============================================================================

use super::native_ui_utils::DragHelper;
use crate::resources::LibraryName;
use macroquad::prelude::*;

/// 选项对话框设置
#[derive(Debug, Clone)]
pub struct OptionSettings {
    pub skill_mode_ctrl: bool, // true = Ctrl模式, false = ~模式
    pub skill_bar_visible: bool,
    pub effect_enabled: bool,
    pub drop_view_enabled: bool,
    pub name_view_enabled: bool,
    pub hp_view_mode1: bool, // HP显示模式
    pub sound_volume: f32,   // 0.0 - 1.0
    pub music_volume: f32,
    pub allow_observe: bool,
    pub new_move_style: bool,
}

impl Default for OptionSettings {
    fn default() -> Self {
        Self {
            skill_mode_ctrl: true,
            skill_bar_visible: true,
            effect_enabled: true,
            drop_view_enabled: true,
            name_view_enabled: true,
            hp_view_mode1: true,
            sound_volume: 0.8,
            music_volume: 0.6,
            allow_observe: false,
            new_move_style: true,
        }
    }
}

/// 选项对话框（混合版本）
/// 按照 C# OptionDialog 实现 - 无标签页
pub struct OptionDialogHybrid {
    /// 窗口位置
    position: Vec2,
    /// 是否可见
    visible: bool,
    /// 对话框尺寸 (从纹理获取)
    size: Vec2,
    /// 设置
    settings: OptionSettings,
    /// 背景纹理 - Title[411]
    bg_texture: Option<Texture2D>,
    /// 关闭按钮纹理 - Prguse2[360/361/362]
    close_textures: [Option<Texture2D>; 3],
    /// 音量条纹理 - Prguse2[468]
    sound_bar_texture: Option<Texture2D>,
    /// 音量指示器纹理 - Prguse[20]
    volume_indicator_texture: Option<Texture2D>,
    /// 拖拽辅助器
    drag_helper: DragHelper,
}

impl Default for OptionDialogHybrid {
    fn default() -> Self {
        Self::new()
    }
}

impl OptionDialogHybrid {
    pub fn new() -> Self {
        Self {
            position: vec2(300.0, 200.0),
            visible: false,
            size: vec2(250.0, 330.0), // 默认值，会被纹理覆盖
            settings: OptionSettings::default(),
            bg_texture: None,
            close_textures: [None, None, None],
            sound_bar_texture: None,
            volume_indicator_texture: None,
            drag_helper: DragHelper::new(),
        }
    }

    /// 显示对话框
    pub fn open(&mut self) {
        // 注意：MainDialog 会在每帧同步时反复调用 open()。
        // 这里必须做到“幂等”，否则会导致：
        // - 对话框无法拖动（每帧被重新居中）
        // - 关闭按钮点击会触发 DragHelper 进入 dragging，随后 close 后没机会收到 mouse_released，
        //   下一次打开就会在鼠标位置“闪一下”（立刻又被 close）。
        if !self.visible {
            self.visible = true;
            // 仅在从关闭->打开时居中一次
            self.position = vec2(
                (screen_width() - self.size.x) / 2.0,
                (screen_height() - self.size.y) / 2.0,
            );
        }
    }

    /// 关闭对话框
    pub fn close(&mut self) {
        self.visible = false;
        // 强制停止拖动：避免关闭发生在拖动/按下期间导致 dragging 状态残留
        self.drag_helper.dragging = false;
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
        Rect::new(self.position.x, self.position.y, self.size.x, self.size.y).contains(point)
    }

    /// 异步加载纹理
    pub fn load_textures(&mut self) {
        // 背景纹理 - Title[411]
        if let Some(texture) = LibraryName::Title.get_texture(411) {
            self.size = vec2(texture.width as f32, texture.height as f32);
            if let Some(tex) = texture.image {
                self.bg_texture = Some(tex);
            }
        }

        // 关闭按钮 - Prguse2[360/361/362]
        for (i, idx) in [360, 361, 362].iter().enumerate() {
            if let Some(texture) = LibraryName::Prguse2.get_texture(*idx) {
                if let Some(tex) = texture.image {
                    self.close_textures[i] = Some(tex);
                }
            }
        }

        // 音量条纹理 - Prguse2[468]
        if let Some(texture) = LibraryName::Prguse2.get_texture(468) {
            if let Some(tex) = texture.image {
                self.sound_bar_texture = Some(tex);
            }
        }

        // 音量指示器 - Prguse[20]
        if let Some(texture) = LibraryName::Prguse.get_texture(20) {
            if let Some(tex) = texture.image {
                self.volume_indicator_texture = Some(tex);
            }
        }
    }

    /// 更新和绘制（返回是否应用了设置）
    pub fn update_and_draw(&mut self) -> bool {
        if !self.visible {
            return false;
        }

        let mouse_pos = vec2(mouse_position().0, mouse_position().1);

        // 拖拽
        let drag_area = Rect::new(self.position.x, self.position.y, self.size.x, 30.0);
        self.drag_helper.apply(drag_area, &mut self.position);

        // 绘制背景
        self.draw_background();

        // 绘制音量条
        self.draw_volume_bars(mouse_pos);

        // 绘制关闭按钮
        self.draw_close_button(mouse_pos);

        false
    }

    /// 绘制背景
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
    }

    /// 绘制音量条
    fn draw_volume_bars(&mut self, mouse_pos: Vec2) {
        // 音效音量条 - SoundBar at (159, 225), VolumeBar at (155, 218)
        // 注：C# 的 Y 坐标可能需要微调
        self.draw_single_volume_bar(
            self.position.x + 159.0,
            self.position.y + 218.0, // bar_y 上移
            self.position.y + 211.0, // indicator_y 上移
            mouse_pos,
            true, // is_sound
        );

        // 音乐音量条 - MusicSoundBar at (159, 251), MusicVolumeBar at (155, 244)
        self.draw_single_volume_bar(
            self.position.x + 159.0,
            self.position.y + 244.0, // bar_y 上移
            self.position.y + 237.0, // indicator_y 上移
            mouse_pos,
            false, // is_music
        );
    }

    fn draw_single_volume_bar(
        &mut self,
        bar_x: f32,
        bar_y: f32,
        indicator_base_y: f32,
        mouse_pos: Vec2,
        is_sound: bool,
    ) {
        // 音量条背景 - Prguse2[468]
        if let Some(bar_tex) = &self.sound_bar_texture {
            let bar_rect = Rect::new(bar_x, bar_y, bar_tex.width(), bar_tex.height());

            // 获取当前音量
            let volume = if is_sound {
                self.settings.sound_volume
            } else {
                self.settings.music_volume
            };

            // C# 绘制填充部分: Size = (SoundBar.Size.Width - 2) * percent
            let fill_width = (bar_tex.width() - 2.0) * volume;
            if fill_width > 0.0 {
                draw_texture_ex(
                    bar_tex,
                    bar_x,
                    bar_y,
                    WHITE,
                    DrawTextureParams {
                        source: Some(Rect::new(0.0, 0.0, fill_width, bar_tex.height())),
                        ..Default::default()
                    },
                );
            }

            // 音量指示器 - Prguse[20]
            // C# 位置: VolumeBar.Location = new Point(159 + fill_width, 218)
            if let Some(indicator) = &self.volume_indicator_texture {
                let indicator_x = bar_x + fill_width;
                draw_texture_ex(
                    indicator,
                    indicator_x,
                    indicator_base_y,
                    WHITE,
                    DrawTextureParams::default(),
                );
            }

            // 交互
            if bar_rect.contains(mouse_pos) && is_mouse_button_down(MouseButton::Left) {
                let new_volume = ((mouse_pos.x - bar_x) / bar_tex.width()).clamp(0.0, 1.0);
                if is_sound {
                    self.settings.sound_volume = new_volume;
                } else {
                    self.settings.music_volume = new_volume;
                }
            }
        }
    }

    /// 绘制关闭按钮 - at (Size.Width - 26, 5)
    fn draw_close_button(&mut self, mouse_pos: Vec2) {
        let close_x = self.position.x + self.size.x - 26.0;
        let close_y = self.position.y + 5.0;

        if let Some(normal) = &self.close_textures[0] {
            let btn_rect = Rect::new(close_x, close_y, normal.width(), normal.height());
            let is_hovered = btn_rect.contains(mouse_pos);
            let is_pressed = is_hovered && is_mouse_button_down(MouseButton::Left);

            let texture = if is_pressed {
                self.close_textures[2].as_ref().unwrap_or(normal)
            } else if is_hovered {
                self.close_textures[1].as_ref().unwrap_or(normal)
            } else {
                normal
            };

            draw_texture_ex(
                texture,
                close_x,
                close_y,
                WHITE,
                DrawTextureParams::default(),
            );

            if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
                self.close();
            }
        }
    }

    /// 获取设置引用
    pub fn get_settings(&self) -> &OptionSettings {
        &self.settings
    }

    /// 获取可变设置引用
    pub fn get_settings_mut(&mut self) -> &mut OptionSettings {
        &mut self.settings
    }
}
