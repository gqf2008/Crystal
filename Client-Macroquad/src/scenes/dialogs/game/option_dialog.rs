/// 选项对话框 - 游戏设置和配置
/// 对应原工程中的选项系统
/// 
/// 功能：
/// - 游戏设置调整
/// - 音效和音乐控制
/// - 显示设置
/// - 键位绑定
/// - 保存和应用设置

use egui_macroquad::egui;
use crate::resources::LibraryName;
use crate::scenes::dialogs::Dialog;

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
    pub show_player_names: bool,        // 显示玩家姓名
    pub show_monster_names: bool,       // 显示怪物姓名
    pub show_item_names: bool,          // 显示掉落物品名称
    pub auto_pickup_gold: bool,         // 自动拾取金币
    pub auto_pickup_items: bool,        // 自动拾取物品
    pub show_damage_numbers: bool,      // 显示伤害数字
    pub enable_pk_mode: bool,           // 启用PK模式
    pub show_guild_names: bool,         // 显示行会名称
}

/// 图形设置
#[derive(Debug, Clone)]
pub struct GraphicsSettings {
    pub fullscreen: bool,               // 全屏模式
    pub window_width: u32,              // 窗口宽度
    pub window_height: u32,             // 窗口高度
    pub vsync: bool,                    // 垂直同步
    pub show_fps: bool,                 // 显示FPS
    pub lighting_effects: bool,         // 光照效果
    pub particle_effects: bool,         // 粒子效果
    pub screen_shake: bool,             // 屏幕震动
}

/// 音频设置
#[derive(Debug, Clone)]
pub struct AudioSettings {
    pub master_volume: f32,             // 主音量 (0.0 - 1.0)
    pub music_volume: f32,              // 音乐音量
    pub sound_volume: f32,              // 音效音量
    pub voice_volume: f32,              // 语音音量
    pub mute_in_background: bool,       // 后台静音
}

/// 控制设置
#[derive(Debug, Clone)]  
pub struct ControlSettings {
    pub mouse_sensitivity: f32,         // 鼠标灵敏度
    pub enable_mouse_look: bool,        // 启用鼠标查看
    pub invert_mouse: bool,             // 反转鼠标
    pub auto_run: bool,                 // 自动跑步
    pub key_bindings: Vec<KeyBinding>,  // 键位绑定
}

/// 键位绑定
#[derive(Debug, Clone)]
pub struct KeyBinding {
    pub action: String,                 // 动作名称
    pub key: String,                    // 绑定的键
    pub description: String,            // 描述
}

/// 选项对话框
pub struct OptionDialog {
    visible: bool,
    position: egui::Pos2,
    
    /// 当前标签页
    active_tab: OptionTab,
    
    /// 各类设置
    game_settings: GameSettings,
    graphics_settings: GraphicsSettings,
    audio_settings: AudioSettings,
    control_settings: ControlSettings,
    
    /// 窗口拖拽状态
    dragging: bool,
    drag_offset: egui::Vec2,
}

impl OptionDialog {
    pub fn new() -> Self {
        // 初始化默认设置
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
        
        let key_bindings = vec![
            KeyBinding { action: "inventory".to_string(), key: "I".to_string(), description: "打开背包".to_string() },
            KeyBinding { action: "character".to_string(), key: "C".to_string(), description: "角色信息".to_string() },
            KeyBinding { action: "skills".to_string(), key: "S".to_string(), description: "技能窗口".to_string() },
            KeyBinding { action: "quests".to_string(), key: "Q".to_string(), description: "任务日志".to_string() },
            KeyBinding { action: "guild".to_string(), key: "G".to_string(), description: "行会窗口".to_string() },
            KeyBinding { action: "chat".to_string(), key: "Enter".to_string(), description: "聊天输入".to_string() },
            KeyBinding { action: "run".to_string(), key: "Shift".to_string(), description: "跑步".to_string() },
            KeyBinding { action: "pickup".to_string(), key: "Ctrl".to_string(), description: "拾取物品".to_string() },
        ];
        
        let control_settings = ControlSettings {
            mouse_sensitivity: 1.0,
            enable_mouse_look: true,
            invert_mouse: false,
            auto_run: false,
            key_bindings,
        };
        
        Self {
            visible: false,
            position: egui::pos2(250.0, 200.0),
            active_tab: OptionTab::Game,
            game_settings,
            graphics_settings,
            audio_settings,
            control_settings,
            dragging: false,
            drag_offset: egui::vec2(0.0, 0.0),
        }
    }
    
    /// 显示/隐藏对话框
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        println!("⚙️ 选项对话框: {}", if self.visible { "显示" } else { "隐藏" });
    }
    
    /// 获取可见状态
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    
    /// 绘制对话框背景
    fn draw_background(&self, ui: &mut egui::Ui, ctx: &egui::Context) -> egui::Rect {
        // 选项对话框背景纹理
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 1002) {
            if let Some(bg_texture) = info.egui_texture {
                let bg_size = bg_texture.size_vec2();
                let bg_rect = egui::Rect::from_min_size(self.position, bg_size);
                
                ui.painter().image(
                    bg_texture.id(),
                    bg_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                
                return bg_rect;
            }
        }
        
        // 降级：绘制默认背景
        let default_size = egui::vec2(450.0, 400.0);
        let default_rect = egui::Rect::from_min_size(self.position, default_size);
        ui.painter().rect_filled(
            default_rect,
            5.0,
            egui::Color32::from_rgb(40, 40, 45),
        );
        ui.painter().rect_stroke(
            default_rect,
            5.0,
            egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 100, 100)),
            egui::epaint::StrokeKind::Outside,
        );
        
        // 绘制标题
        ui.painter().text(
            egui::pos2(default_rect.center().x, default_rect.min.y + 20.0),
            egui::Align2::CENTER_CENTER,
            "游戏选项",
            egui::FontId::proportional(16.0),
            egui::Color32::YELLOW,
        );
        
        default_rect
    }
    
    /// 绘制标签页按钮
    fn draw_tab_buttons(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context, bg_rect: &egui::Rect) {
        let tab_y = bg_rect.min.y + 45.0;
        let tab_buttons = [
            (OptionTab::Game, "游戏", bg_rect.min.x + 20.0),
            (OptionTab::Graphics, "图形", bg_rect.min.x + 90.0),
            (OptionTab::Audio, "音频", bg_rect.min.x + 160.0),
            (OptionTab::Controls, "控制", bg_rect.min.x + 230.0),
        ];
        
        for (tab, label, x) in tab_buttons {
            let is_active = self.active_tab == tab;
            let button_rect = egui::Rect::from_min_size(
                egui::pos2(x, tab_y),
                egui::vec2(60.0, 25.0)
            );
            
            let response = ui.interact(button_rect, egui::Id::new(format!("option_tab_{:?}", tab)), egui::Sense::click());
            
            let bg_color = if is_active {
                egui::Color32::from_rgb(80, 120, 160)
            } else if response.hovered() {
                egui::Color32::from_rgb(60, 60, 70)
            } else {
                egui::Color32::from_rgb(50, 50, 55)
            };
            
            ui.painter().rect_filled(button_rect, 3.0, bg_color);
            ui.painter().rect_stroke(
                button_rect,
                3.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 100)),
                egui::epaint::StrokeKind::Outside,
            );
            
            ui.painter().text(
                button_rect.center(),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(12.0),
                egui::Color32::WHITE,
            );
            
            if response.clicked() {
                self.active_tab = tab;
                println!("🔄 切换到选项标签页: {:?}", tab);
            }
        }
    }
    
    /// 绘制游戏设置页
    fn draw_game_settings(&mut self, ui: &mut egui::Ui, bg_rect: &egui::Rect) {
        let content_area = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 20.0, bg_rect.min.y + 80.0),
            egui::vec2(bg_rect.width() - 40.0, bg_rect.height() - 120.0)
        );
        
        // 各种游戏设置选项 - 使用ui.checkbox避免借用冲突
        ui.checkbox(&mut self.game_settings.show_player_names, "显示玩家姓名");
        ui.checkbox(&mut self.game_settings.show_monster_names, "显示怪物姓名");
        ui.checkbox(&mut self.game_settings.show_item_names, "显示掉落物品名称");
        ui.checkbox(&mut self.game_settings.auto_pickup_gold, "自动拾取金币");
        ui.checkbox(&mut self.game_settings.auto_pickup_items, "自动拾取物品");
        ui.checkbox(&mut self.game_settings.show_damage_numbers, "显示伤害数字");
        ui.checkbox(&mut self.game_settings.enable_pk_mode, "启用PK模式");
        ui.checkbox(&mut self.game_settings.show_guild_names, "显示行会名称");
    }
    
    /// 绘制图形设置页
    fn draw_graphics_settings(&mut self, ui: &mut egui::Ui, bg_rect: &egui::Rect) {
        let content_area = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 20.0, bg_rect.min.y + 80.0),
            egui::vec2(bg_rect.width() - 40.0, bg_rect.height() - 120.0)
        );
        
        let mut y_pos = content_area.min.y;
        // 图形设置选项 - 使用ui.checkbox
        ui.checkbox(&mut self.graphics_settings.fullscreen, "全屏模式");
        ui.checkbox(&mut self.graphics_settings.vsync, "垂直同步");
        ui.checkbox(&mut self.graphics_settings.show_fps, "显示FPS");
        ui.checkbox(&mut self.graphics_settings.lighting_effects, "光照效果");
        ui.checkbox(&mut self.graphics_settings.particle_effects, "粒子效果");
        ui.checkbox(&mut self.graphics_settings.screen_shake, "屏幕震动");
        
        y_pos += 10.0;
        
        // 分辨率设置
        ui.painter().text(
            egui::pos2(content_area.min.x, y_pos),
            egui::Align2::LEFT_CENTER,
            "窗口分辨率:",
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );
        
        ui.painter().text(
            egui::pos2(content_area.min.x + 100.0, y_pos),
            egui::Align2::LEFT_CENTER,
            format!("{}x{}", self.graphics_settings.window_width, self.graphics_settings.window_height),
            egui::FontId::proportional(12.0),
            egui::Color32::YELLOW,
        );
    }
    
    /// 绘制音频设置页
    fn draw_audio_settings(&mut self, ui: &mut egui::Ui, bg_rect: &egui::Rect) {
        let content_area = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 20.0, bg_rect.min.y + 80.0),
            egui::vec2(bg_rect.width() - 40.0, bg_rect.height() - 120.0)
        );
        
        // 音量滑块 - 使用ui.slider
        ui.add(egui::Slider::new(&mut self.audio_settings.master_volume, 0.0..=1.0).text("主音量"));
        ui.add(egui::Slider::new(&mut self.audio_settings.music_volume, 0.0..=1.0).text("音乐音量"));
        ui.add(egui::Slider::new(&mut self.audio_settings.sound_volume, 0.0..=1.0).text("音效音量"));
        ui.add(egui::Slider::new(&mut self.audio_settings.voice_volume, 0.0..=1.0).text("语音音量"));
        
        // 音频选项
        ui.checkbox(&mut self.audio_settings.mute_in_background, "后台静音");
    }
    
    /// 绘制控制设置页
    fn draw_control_settings(&mut self, ui: &mut egui::Ui, bg_rect: &egui::Rect) {
        let content_area = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 20.0, bg_rect.min.y + 80.0),
            egui::vec2(bg_rect.width() - 40.0, bg_rect.height() - 120.0)
        );
        
        let mut y_pos = content_area.min.y;
        let line_height = 25.0;
        
        // 鼠标设置
        ui.painter().text(
            egui::pos2(content_area.min.x, y_pos),
            egui::Align2::LEFT_CENTER,
            format!("鼠标灵敏度: {:.1}", self.control_settings.mouse_sensitivity),
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );
        y_pos += line_height;
        
        // 鼠标选项 - 使用ui.checkbox
        ui.checkbox(&mut self.control_settings.enable_mouse_look, "启用鼠标查看");  
        ui.checkbox(&mut self.control_settings.invert_mouse, "反转鼠标");
        ui.checkbox(&mut self.control_settings.auto_run, "自动跑步");
        
        y_pos += 10.0;
        
        // 键位绑定标题
        ui.painter().text(
            egui::pos2(content_area.min.x, y_pos),
            egui::Align2::LEFT_CENTER,
            "键位绑定:",
            egui::FontId::proportional(12.0),
            egui::Color32::YELLOW,
        );
        y_pos += 20.0;
        
        // 键位绑定列表（显示前几个）
        for (_i, binding) in self.control_settings.key_bindings.iter().enumerate().take(5) {
            ui.painter().text(
                egui::pos2(content_area.min.x, y_pos),
                egui::Align2::LEFT_CENTER,
                format!("{}: {}", binding.description, binding.key),
                egui::FontId::proportional(10.0),
                egui::Color32::WHITE,
            );
            y_pos += 18.0;
        }
    }
    
    /// 绘制复选框
    fn draw_checkbox(&self, ui: &mut egui::Ui, label: &str, value: &mut bool, pos: egui::Pos2) {
        let checkbox_size = 12.0;
        let checkbox_rect = egui::Rect::from_min_size(pos, egui::vec2(checkbox_size, checkbox_size));
        
        let response = ui.interact(checkbox_rect, egui::Id::new(format!("checkbox_{}", label)), egui::Sense::click());
        
        // 绘制复选框背景
        ui.painter().rect_filled(
            checkbox_rect,
            2.0,
            egui::Color32::from_rgb(30, 30, 35),
        );
        ui.painter().rect_stroke(
            checkbox_rect,
            2.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 100)),
            egui::epaint::StrokeKind::Outside,
        );
        
        // 绘制勾选标记
        if *value {
            ui.painter().text(
                checkbox_rect.center(),
                egui::Align2::CENTER_CENTER,
                "✓",
                egui::FontId::proportional(10.0),
                egui::Color32::GREEN,
            );
        }
        
        // 绘制标签
        ui.painter().text(
            egui::pos2(pos.x + checkbox_size + 10.0, pos.y + checkbox_size / 2.0),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );
        
        if response.clicked() {
            *value = !*value;
            println!("⚙️ 设置改变: {} = {}", label, *value);
        }
    }
    
    /// 绘制音量滑块
    fn draw_volume_slider(&self, ui: &mut egui::Ui, label: &str, volume: &mut f32, pos: egui::Pos2) {
        let slider_width = 150.0;
        let slider_height = 8.0;
        
        // 标签
        ui.painter().text(
            pos,
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );
        
        // 滑块背景
        let slider_rect = egui::Rect::from_min_size(
            egui::pos2(pos.x + 80.0, pos.y - slider_height / 2.0),
            egui::vec2(slider_width, slider_height)
        );
        
        ui.painter().rect_filled(
            slider_rect,
            2.0,
            egui::Color32::from_rgb(40, 40, 40),
        );
        
        // 滑块填充
        let filled_width = slider_width * *volume;
        let filled_rect = egui::Rect::from_min_size(
            slider_rect.min,
            egui::vec2(filled_width, slider_height)
        );
        
        ui.painter().rect_filled(
            filled_rect,
            2.0,
            egui::Color32::from_rgb(100, 150, 100),
        );
        
        // 音量数值
        ui.painter().text(
            egui::pos2(pos.x + 240.0, pos.y),
            egui::Align2::LEFT_CENTER,
            format!("{:.0}%", *volume * 100.0),
            egui::FontId::proportional(10.0),
            egui::Color32::GRAY,
        );
        
        // 滑块交互（简化实现）
        let response = ui.interact(slider_rect, egui::Id::new(format!("slider_{}", label)), egui::Sense::click());
        if response.clicked() {
            if let Some(click_pos) = response.interact_pointer_pos() {
                let new_value = ((click_pos.x - slider_rect.min.x) / slider_width).clamp(0.0, 1.0);
                *volume = new_value;
                println!("🔊 音量调整: {} = {:.1}", label, *volume);
            }
        }
    }
    
    /// 绘制底部按钮
    fn draw_bottom_buttons(&mut self, ui: &mut egui::Ui, bg_rect: &egui::Rect) {
        let button_y = bg_rect.max.y - 40.0;
        let button_size = egui::vec2(80.0, 25.0);
        
        // 确定按钮
        let ok_rect = egui::Rect::from_min_size(
            egui::pos2(bg_rect.max.x - 200.0, button_y),
            button_size
        );
        
        let ok_response = ui.interact(ok_rect, egui::Id::new("option_ok"), egui::Sense::click());
        
        ui.painter().rect_filled(
            ok_rect,
            3.0,
            if ok_response.hovered() { egui::Color32::from_rgb(70, 100, 70) } else { egui::Color32::from_rgb(50, 80, 50) }
        );
        ui.painter().rect_stroke(ok_rect, 3.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 100)), egui::epaint::StrokeKind::Outside);
        
        ui.painter().text(
            ok_rect.center(),
            egui::Align2::CENTER_CENTER,
            "确定",
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );
        
        if ok_response.clicked() {
            self.apply_settings();
            self.visible = false;
            println!("✅ 应用设置并关闭选项对话框");
        }
        
        // 取消按钮
        let cancel_rect = egui::Rect::from_min_size(
            egui::pos2(bg_rect.max.x - 110.0, button_y),
            button_size
        );
        
        let cancel_response = ui.interact(cancel_rect, egui::Id::new("option_cancel"), egui::Sense::click());
        
        ui.painter().rect_filled(
            cancel_rect,
            3.0,
            if cancel_response.hovered() { egui::Color32::from_rgb(100, 70, 70) } else { egui::Color32::from_rgb(80, 50, 50) }
        );
        ui.painter().rect_stroke(cancel_rect, 3.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 100)), egui::epaint::StrokeKind::Outside);
        
        ui.painter().text(
            cancel_rect.center(),
            egui::Align2::CENTER_CENTER,
            "取消",
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );
        
        if cancel_response.clicked() {
            self.visible = false;
            println!("❌ 取消设置更改");
        }
    }
    
    /// 应用设置
    fn apply_settings(&self) {
        println!("💾 应用游戏设置:");
        println!("  - 显示玩家姓名: {}", self.game_settings.show_player_names);
        println!("  - 主音量: {:.1}", self.audio_settings.master_volume);
        println!("  - 全屏模式: {}", self.graphics_settings.fullscreen);
        println!("  - 鼠标灵敏度: {:.1}", self.control_settings.mouse_sensitivity);
        // 这里可以添加实际的设置保存逻辑
    }
    
    /// 处理窗口拖拽
    fn handle_window_dragging(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        let title_area = egui::Rect::from_min_size(
            bg_rect.min,
            egui::vec2(bg_rect.width(), 40.0),
        );
        
        let drag_response = ui.interact(title_area, egui::Id::new("option_drag"), egui::Sense::drag());
        
        if drag_response.drag_started() {
            self.dragging = true;
            if let Some(pointer_pos) = ctx.pointer_interact_pos() {
                self.drag_offset = self.position.to_vec2() - pointer_pos.to_vec2();
            }
        }
        
        if self.dragging {
            if let Some(pointer_pos) = ctx.pointer_interact_pos() {
                self.position = (pointer_pos.to_vec2() + self.drag_offset).to_pos2();
            }
        }
        
        if drag_response.drag_stopped() {
            self.dragging = false;
        }
    }
}

impl Dialog for OptionDialog {
    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        if !self.visible {
            *open = false;
            return;
        }
        
        egui::Area::new(egui::Id::new("option_dialog"))
            .fixed_pos(self.position)
            .movable(false)
            .show(ctx, |ui| {
                // 绘制背景
                let bg_rect = self.draw_background(ui, ctx);
                
                // 处理窗口拖拽
                self.handle_window_dragging(ui, ctx, &bg_rect);
                
                // 绘制标签页按钮
                self.draw_tab_buttons(ui, ctx, &bg_rect);
                
                // 根据当前标签页绘制内容
                match self.active_tab {
                    OptionTab::Game => self.draw_game_settings(ui, &bg_rect),
                    OptionTab::Graphics => self.draw_graphics_settings(ui, &bg_rect),
                    OptionTab::Audio => self.draw_audio_settings(ui, &bg_rect),
                    OptionTab::Controls => self.draw_control_settings(ui, &bg_rect),
                }
                
                // 绘制底部按钮
                self.draw_bottom_buttons(ui, &bg_rect);
            });
        
        *open = self.visible;
    }
}