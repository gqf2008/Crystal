// ============================================================================
// OptionDialogHybrid - 选项对话框（混合版本）
// ============================================================================
//
// 【实现方式】
// - 使用 macroquad 原生 draw_* 函数绘制
// - 使用 DragHelper 实现拖拽功能
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use super::native_ui_utils::DragHelper;

/// 选项对话框标签页
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionTab {
    Game,       // 游戏设置
    Graphics,   // 图形设置
    Audio,      // 音频设置
    Controls,   // 控制设置
}

/// 游戏设置
#[derive(Debug, Clone)]
pub struct GameSettings {
    pub show_player_names: bool,
    pub show_monster_names: bool,
    pub show_item_names: bool,
    pub auto_pickup_gold: bool,
    pub auto_pickup_items: bool,
    pub show_damage_numbers: bool,
    pub enable_pk_mode: bool,
    pub show_guild_names: bool,
}

/// 图形设置
#[derive(Debug, Clone)]
pub struct GraphicsSettings {
    pub fullscreen: bool,
    pub window_width: u32,
    pub window_height: u32,
    pub vsync: bool,
    pub show_fps: bool,
    pub lighting_effects: bool,
    pub particle_effects: bool,
    pub screen_shake: bool,
}

/// 音频设置
#[derive(Debug, Clone)]
pub struct AudioSettings {
    pub master_volume: f32,
    pub music_volume: f32,
    pub sound_volume: f32,
    pub voice_volume: f32,
    pub mute_in_background: bool,
}

/// 控制设置
#[derive(Debug, Clone)]
pub struct ControlSettings {
    pub mouse_sensitivity: f32,
    pub enable_mouse_look: bool,
    pub invert_mouse: bool,
    pub auto_run: bool,
}

/// 选项对话框（混合版本）
pub struct OptionDialogHybrid {
    /// 窗口位置
    position: Vec2,
    /// 是否可见
    visible: bool,
    /// 对话框尺寸
    size: Vec2,
    /// 当前标签页
    active_tab: OptionTab,
    /// 游戏设置
    game_settings: GameSettings,
    /// 图形设置
    graphics_settings: GraphicsSettings,
    /// 音频设置
    audio_settings: AudioSettings,
    /// 控制设置
    control_settings: ControlSettings,
    /// 背景纹理
    bg_texture: Option<Texture2D>,
    /// 拖拽辅助器
    drag_helper: DragHelper,
}

impl OptionDialogHybrid {
    pub fn new() -> Self {
        let game_settings = GameSettings {
            show_player_names: true,
            show_monster_names: true,
            show_item_names: true,
            auto_pickup_gold: false,
            auto_pickup_items: false,
            show_damage_numbers: true,
            enable_pk_mode: false,
            show_guild_names: true,
        };

        let graphics_settings = GraphicsSettings {
            fullscreen: false,
            window_width: 1024,
            window_height: 768,
            vsync: true,
            show_fps: false,
            lighting_effects: true,
            particle_effects: true,
            screen_shake: true,
        };

        let audio_settings = AudioSettings {
            master_volume: 0.8,
            music_volume: 0.6,
            sound_volume: 0.8,
            voice_volume: 0.7,
            mute_in_background: false,
        };

        let control_settings = ControlSettings {
            mouse_sensitivity: 1.0,
            enable_mouse_look: true,
            invert_mouse: false,
            auto_run: false,
        };

        Self {
            position: vec2(250.0, 200.0),
            visible: false,
            size: vec2(450.0, 400.0),
            active_tab: OptionTab::Game,
            game_settings,
            graphics_settings,
            audio_settings,
            control_settings,
            bg_texture: None,
            drag_helper: DragHelper::new(),
        }
    }

    /// 显示对话框
    pub fn open(&mut self) {
        self.visible = true;
    }

    /// 关闭对话框
    pub fn close(&mut self) {
        self.visible = false;
    }

    /// 切换显示状态
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        println!("⚙️ 选项对话框: {}", if self.visible { "显示" } else { "隐藏" });
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
    pub async fn load_textures(&mut self) {
        // 预加载选项对话框纹理
        if let Some(texture) = LibraryName::Prguse.get_texture(1002) {
            self.size = vec2(texture.width as f32, texture.height as f32);
            if let Some(tex) = texture.image {
                self.bg_texture = Some(tex);
            }
        }
    }

    /// 更新和绘制（返回是否应用了设置）
    pub fn update_and_draw(&mut self) -> bool {
        if !self.visible {
            return false;
        }

        let mouse_pos = vec2(mouse_position().0, mouse_position().1);
        let mut applied = false;

        // 使用 DragHelper 实现拖拽
        let drag_area = Rect::new(self.position.x, self.position.y, self.size.x, 30.0);
        self.drag_helper.apply(drag_area, &mut self.position);

        // 绘制背景
        self.draw_background();

        // 绘制标签页按钮
        self.draw_tab_buttons(mouse_pos);

        // 根据当前标签页绘制内容
        match self.active_tab {
            OptionTab::Game => self.draw_game_settings(mouse_pos),
            OptionTab::Graphics => self.draw_graphics_settings(mouse_pos),
            OptionTab::Audio => self.draw_audio_settings(mouse_pos),
            OptionTab::Controls => self.draw_control_settings(mouse_pos),
        }

        // 绘制底部按钮
        if self.draw_bottom_buttons(mouse_pos) {
            applied = true;
            self.apply_settings();
            self.close();
        }

        applied
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
        } else {
            // 降级：绘制默认背景
            draw_rectangle(
                self.position.x,
                self.position.y,
                self.size.x,
                self.size.y,
                Color::from_rgba(40, 40, 45, 250),
            );
            draw_rectangle_lines(
                self.position.x,
                self.position.y,
                self.size.x,
                self.size.y,
                2.0,
                Color::from_rgba(100, 100, 100, 255),
            );
        }

        // 标题
        draw_text(
            "游戏选项",
            self.position.x + self.size.x / 2.0 - 40.0,
            self.position.y + 25.0,
            20.0,
            YELLOW,
        );
    }

    /// 绘制标签页按钮
    fn draw_tab_buttons(&mut self, mouse_pos: Vec2) {
        let tab_y = self.position.y + 45.0;
        let tab_buttons = [
            (OptionTab::Game, "游戏", self.position.x + 20.0),
            (OptionTab::Graphics, "图形", self.position.x + 90.0),
            (OptionTab::Audio, "音频", self.position.x + 160.0),
            (OptionTab::Controls, "控制", self.position.x + 230.0),
        ];

        for (tab, label, x) in tab_buttons {
            let button_rect = Rect::new(x, tab_y, 60.0, 25.0);
            let is_active = self.active_tab == tab;
            let is_hovered = button_rect.contains(mouse_pos);

            let bg_color = if is_active {
                Color::from_rgba(80, 120, 160, 255)
            } else if is_hovered {
                Color::from_rgba(60, 60, 70, 255)
            } else {
                Color::from_rgba(50, 50, 55, 255)
            };

            draw_rectangle(button_rect.x, button_rect.y, button_rect.w, button_rect.h, bg_color);
            draw_rectangle_lines(
                button_rect.x,
                button_rect.y,
                button_rect.w,
                button_rect.h,
                1.0,
                Color::from_rgba(100, 100, 100, 255),
            );

            draw_text(
                label,
                button_rect.x + 10.0,
                button_rect.y + 17.0,
                14.0,
                WHITE,
            );

            if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
                self.active_tab = tab;
            }
        }
    }

    /// 绘制游戏设置页
    fn draw_game_settings(&mut self, mouse_pos: Vec2) {
        let content_x = self.position.x + 30.0;
        let mut y = self.position.y + 90.0;
        let line_height = 28.0;

        let settings = [
            ("显示玩家姓名", &mut self.game_settings.show_player_names as *mut bool),
            ("显示怪物姓名", &mut self.game_settings.show_monster_names as *mut bool),
            ("显示掉落物品名称", &mut self.game_settings.show_item_names as *mut bool),
            ("自动拾取金币", &mut self.game_settings.auto_pickup_gold as *mut bool),
            ("自动拾取物品", &mut self.game_settings.auto_pickup_items as *mut bool),
            ("显示伤害数字", &mut self.game_settings.show_damage_numbers as *mut bool),
            ("启用PK模式", &mut self.game_settings.enable_pk_mode as *mut bool),
            ("显示行会名称", &mut self.game_settings.show_guild_names as *mut bool),
        ];

        for (label, value_ptr) in settings {
            let value = unsafe { &mut *value_ptr };
            self.draw_checkbox(content_x, y, label, value, mouse_pos);
            y += line_height;
        }
    }

    /// 绘制图形设置页
    fn draw_graphics_settings(&mut self, mouse_pos: Vec2) {
        let content_x = self.position.x + 30.0;
        let mut y = self.position.y + 90.0;
        let line_height = 28.0;

        let settings = [
            ("全屏模式", &mut self.graphics_settings.fullscreen as *mut bool),
            ("垂直同步", &mut self.graphics_settings.vsync as *mut bool),
            ("显示FPS", &mut self.graphics_settings.show_fps as *mut bool),
            ("光照效果", &mut self.graphics_settings.lighting_effects as *mut bool),
            ("粒子效果", &mut self.graphics_settings.particle_effects as *mut bool),
            ("屏幕震动", &mut self.graphics_settings.screen_shake as *mut bool),
        ];

        for (label, value_ptr) in settings {
            let value = unsafe { &mut *value_ptr };
            self.draw_checkbox(content_x, y, label, value, mouse_pos);
            y += line_height;
        }

        y += 10.0;

        // 分辨率显示
        draw_text(
            &format!(
                "窗口分辨率: {}x{}",
                self.graphics_settings.window_width, self.graphics_settings.window_height
            ),
            content_x,
            y,
            14.0,
            WHITE,
        );
    }

    /// 绘制音频设置页
    fn draw_audio_settings(&mut self, mouse_pos: Vec2) {
        let content_x = self.position.x + 30.0;
        let mut y = self.position.y + 90.0;
        let line_height = 35.0;
        let slider_width = 150.0;
        let slider_height = 8.0;

        // 音量滑块 - 主音量
        draw_text_cn("主音量", content_x, y, 14.0, WHITE);
        let slider_x = content_x + 90.0;
        let slider_rect = Rect::new(slider_x, y - slider_height / 2.0, slider_width, slider_height);
        draw_rectangle(slider_rect.x, slider_rect.y, slider_rect.w, slider_rect.h, Color::from_rgba(40, 40, 40, 255));
        draw_rectangle(slider_rect.x, slider_rect.y, slider_width * self.audio_settings.master_volume, slider_rect.h, Color::from_rgba(100, 150, 100, 255));
        draw_text_cn(&format!("{:.0}%", self.audio_settings.master_volume * 100.0), slider_x + slider_width + 10.0, y, 12.0, GRAY);
        if slider_rect.contains(mouse_pos) && is_mouse_button_down(MouseButton::Left) {
            self.audio_settings.master_volume = ((mouse_pos.x - slider_rect.x) / slider_width).clamp(0.0, 1.0);
        }
        y += line_height;

        // 音乐音量
        draw_text_cn("音乐音量", content_x, y, 14.0, WHITE);
        let slider_rect = Rect::new(slider_x, y - slider_height / 2.0, slider_width, slider_height);
        draw_rectangle(slider_rect.x, slider_rect.y, slider_rect.w, slider_rect.h, Color::from_rgba(40, 40, 40, 255));
        draw_rectangle(slider_rect.x, slider_rect.y, slider_width * self.audio_settings.music_volume, slider_rect.h, Color::from_rgba(100, 150, 100, 255));
        draw_text_cn(&format!("{:.0}%", self.audio_settings.music_volume * 100.0), slider_x + slider_width + 10.0, y, 12.0, GRAY);
        if slider_rect.contains(mouse_pos) && is_mouse_button_down(MouseButton::Left) {
            self.audio_settings.music_volume = ((mouse_pos.x - slider_rect.x) / slider_width).clamp(0.0, 1.0);
        }
        y += line_height;

        // 音效音量
        draw_text_cn("音效音量", content_x, y, 14.0, WHITE);
        let slider_rect = Rect::new(slider_x, y - slider_height / 2.0, slider_width, slider_height);
        draw_rectangle(slider_rect.x, slider_rect.y, slider_rect.w, slider_rect.h, Color::from_rgba(40, 40, 40, 255));
        draw_rectangle(slider_rect.x, slider_rect.y, slider_width * self.audio_settings.sound_volume, slider_rect.h, Color::from_rgba(100, 150, 100, 255));
        draw_text_cn(&format!("{:.0}%", self.audio_settings.sound_volume * 100.0), slider_x + slider_width + 10.0, y, 12.0, GRAY);
        if slider_rect.contains(mouse_pos) && is_mouse_button_down(MouseButton::Left) {
            self.audio_settings.sound_volume = ((mouse_pos.x - slider_rect.x) / slider_width).clamp(0.0, 1.0);
        }
        y += line_height;

        // 语音音量
        draw_text_cn("语音音量", content_x, y, 14.0, WHITE);
        let slider_rect = Rect::new(slider_x, y - slider_height / 2.0, slider_width, slider_height);
        draw_rectangle(slider_rect.x, slider_rect.y, slider_rect.w, slider_rect.h, Color::from_rgba(40, 40, 40, 255));
        draw_rectangle(slider_rect.x, slider_rect.y, slider_width * self.audio_settings.voice_volume, slider_rect.h, Color::from_rgba(100, 150, 100, 255));
        draw_text_cn(&format!("{:.0}%", self.audio_settings.voice_volume * 100.0), slider_x + slider_width + 10.0, y, 12.0, GRAY);
        if slider_rect.contains(mouse_pos) && is_mouse_button_down(MouseButton::Left) {
            self.audio_settings.voice_volume = ((mouse_pos.x - slider_rect.x) / slider_width).clamp(0.0, 1.0);
        }
        y += line_height + 10.0;

        // 后台静音选项
        let checkbox_size = 14.0;
        let checkbox_rect = Rect::new(content_x, y, checkbox_size, checkbox_size);
        let is_hovered = checkbox_rect.contains(mouse_pos);
        draw_rectangle(checkbox_rect.x, checkbox_rect.y, checkbox_rect.w, checkbox_rect.h, Color::from_rgba(30, 30, 35, 255));
        draw_rectangle_lines(checkbox_rect.x, checkbox_rect.y, checkbox_rect.w, checkbox_rect.h, 1.0,
            if is_hovered { Color::from_rgba(150, 150, 150, 255) } else { Color::from_rgba(100, 100, 100, 255) });
        if self.audio_settings.mute_in_background {
            draw_text("✓", content_x + 2.0, y + 12.0, 14.0, GREEN);
        }
        draw_text("后台静音", content_x + checkbox_size + 10.0, y + 11.0, 14.0, WHITE);
        if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
            self.audio_settings.mute_in_background = !self.audio_settings.mute_in_background;
        }
    }

    /// 绘制控制设置页
    fn draw_control_settings(&mut self, mouse_pos: Vec2) {
        let content_x = self.position.x + 30.0;
        let mut y = self.position.y + 90.0;
        let line_height = 28.0;

        // 鼠标灵敏度
        draw_text(
            &format!("鼠标灵敏度: {:.1}", self.control_settings.mouse_sensitivity),
            content_x,
            y,
            14.0,
            WHITE,
        );
        y += line_height;

        let settings = [
            ("启用鼠标查看", &mut self.control_settings.enable_mouse_look as *mut bool),
            ("反转鼠标", &mut self.control_settings.invert_mouse as *mut bool),
            ("自动跑步", &mut self.control_settings.auto_run as *mut bool),
        ];

        for (label, value_ptr) in settings {
            let value = unsafe { &mut *value_ptr };
            self.draw_checkbox(content_x, y, label, value, mouse_pos);
            y += line_height;
        }
    }

    /// 绘制复选框
    fn draw_checkbox(&self, x: f32, y: f32, label: &str, value: &mut bool, mouse_pos: Vec2) {
        let checkbox_size = 14.0;
        let checkbox_rect = Rect::new(x, y, checkbox_size, checkbox_size);

        let is_hovered = checkbox_rect.contains(mouse_pos);

        // 复选框背景
        draw_rectangle(
            checkbox_rect.x,
            checkbox_rect.y,
            checkbox_rect.w,
            checkbox_rect.h,
            Color::from_rgba(30, 30, 35, 255),
        );
        draw_rectangle_lines(
            checkbox_rect.x,
            checkbox_rect.y,
            checkbox_rect.w,
            checkbox_rect.h,
            1.0,
            if is_hovered {
                Color::from_rgba(150, 150, 150, 255)
            } else {
                Color::from_rgba(100, 100, 100, 255)
            },
        );

        // 勾选标记
        if *value {
            draw_text("✓", x + 2.0, y + 12.0, 14.0, GREEN);
        }

        // 标签
        draw_text(label, x + checkbox_size + 10.0, y + 11.0, 14.0, WHITE);

        // 点击切换
        if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
            *value = !*value;
        }
    }

    /// 绘制音量滑块
    fn draw_volume_slider(&self, x: f32, y: f32, label: &str, volume: &mut f32, mouse_pos: Vec2) {
        let slider_width = 150.0;
        let slider_height = 8.0;

        // 标签
        draw_text(label, x, y, 14.0, WHITE);

        // 滑块背景
        let slider_x = x + 90.0;
        let slider_rect = Rect::new(slider_x, y - slider_height / 2.0, slider_width, slider_height);

        draw_rectangle(
            slider_rect.x,
            slider_rect.y,
            slider_rect.w,
            slider_rect.h,
            Color::from_rgba(40, 40, 40, 255),
        );

        // 滑块填充
        let filled_width = slider_width * *volume;
        draw_rectangle(
            slider_rect.x,
            slider_rect.y,
            filled_width,
            slider_rect.h,
            Color::from_rgba(100, 150, 100, 255),
        );

        // 音量数值
        draw_text(
            &format!("{:.0}%", *volume * 100.0),
            slider_x + slider_width + 10.0,
            y,
            12.0,
            GRAY,
        );

        // 滑块交互
        let is_hovered = slider_rect.contains(mouse_pos);
        if is_hovered && is_mouse_button_down(MouseButton::Left) {
            let new_value = ((mouse_pos.x - slider_rect.x) / slider_width).clamp(0.0, 1.0);
            *volume = new_value;
        }
    }

    /// 绘制底部按钮（返回是否点击确定）
    fn draw_bottom_buttons(&mut self, mouse_pos: Vec2) -> bool {
        let button_y = self.position.y + self.size.y - 40.0;
        let button_size = vec2(80.0, 25.0);

        // 确定按钮
        let ok_rect = Rect::new(
            self.position.x + self.size.x - 200.0,
            button_y,
            button_size.x,
            button_size.y,
        );
        let ok_hovered = ok_rect.contains(mouse_pos);

        draw_rectangle(
            ok_rect.x,
            ok_rect.y,
            ok_rect.w,
            ok_rect.h,
            if ok_hovered {
                Color::from_rgba(70, 100, 70, 255)
            } else {
                Color::from_rgba(50, 80, 50, 255)
            },
        );
        draw_rectangle_lines(ok_rect.x, ok_rect.y, ok_rect.w, ok_rect.h, 1.0, Color::from_rgba(100, 100, 100, 255));
        draw_text("确定", ok_rect.x + 25.0, ok_rect.y + 17.0, 14.0, WHITE);

        // 取消按钮
        let cancel_rect = Rect::new(
            self.position.x + self.size.x - 110.0,
            button_y,
            button_size.x,
            button_size.y,
        );
        let cancel_hovered = cancel_rect.contains(mouse_pos);

        draw_rectangle(
            cancel_rect.x,
            cancel_rect.y,
            cancel_rect.w,
            cancel_rect.h,
            if cancel_hovered {
                Color::from_rgba(100, 70, 70, 255)
            } else {
                Color::from_rgba(80, 50, 50, 255)
            },
        );
        draw_rectangle_lines(cancel_rect.x, cancel_rect.y, cancel_rect.w, cancel_rect.h, 1.0, Color::from_rgba(100, 100, 100, 255));
        draw_text("取消", cancel_rect.x + 25.0, cancel_rect.y + 17.0, 14.0, WHITE);

        if cancel_hovered && is_mouse_button_pressed(MouseButton::Left) {
            self.close();
        }

        ok_hovered && is_mouse_button_pressed(MouseButton::Left)
    }

    /// 应用设置
    fn apply_settings(&self) {
        println!("💾 应用游戏设置:");
        println!("  - 显示玩家姓名: {}", self.game_settings.show_player_names);
        println!("  - 主音量: {:.1}", self.audio_settings.master_volume);
        println!("  - 全屏模式: {}", self.graphics_settings.fullscreen);
        println!("  - 鼠标灵敏度: {:.1}", self.control_settings.mouse_sensitivity);
    }
}
