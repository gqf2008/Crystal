// ============================================================================
// 角色选择场景 - 混合渲染架构
// ============================================================================
// 
// 【渲染架构说明】
// 本场景采用 macroquad + egui 混合渲染模式，职责分离如下：
//
// 1. macroquad 渲染层（背景和装饰）：
//    - 背景纹理 Prguse[65]
//    - 标题纹理 Title[40]
//    - 角色预览动画 ChrSel[220+]
//    - 所有静态背景元素
//
// 2. egui 交互层（UI 控件）：
//    - 角色按钮（4个角色槽位）
//    - 底部功能按钮（开始游戏、新建角色、删除角色、制作名单、退出）
//    - 文本标签（服务器名、最后登录时间等）
//    - 新建角色对话框
//    - 删除确认对话框
//    - 消息框
//
// 3. DPI 处理机制：
//    - macroquad: 通过 screen_dpi_scale() 自动处理物理像素
//    - egui: 通过 ctx.set_pixels_per_point(dpi_scale) 同步缩放
//    - 坐标对应: macroquad 物理坐标 = egui 逻辑坐标 × pixels_per_point
//
// ============================================================================

use crate::game::GameResult;
use crate::resources::LibraryName;
use crate::scenes::{Scene, SceneTransition};
use crate::scenes::dialogs::{
    Dialog, MessageBox, MessageBoxButtons,
    NewCharacterDialog, NewCharacterEvent,
    DeleteCharacterDialog, DeleteCharacterEvent,
};
use macroquad::prelude::*;
use egui_macroquad::egui;
use std::sync::Arc;

/// 角色信息
#[derive(Debug, Clone)]
pub struct CharacterInfo {
    pub index: i32,
    pub name: String,
    pub level: u16,
    pub class: u8,  // 0=Warrior, 1=Wizard, 2=Taoist
    pub gender: u8, // 0=Male, 1=Female
    pub last_access: String,
}

/// 角色选择场景
pub struct SelectScene {
    // 角色数据
    characters: Vec<CharacterInfo>,
    selected_index: Option<usize>,
    
    // 对话框组件
    new_character_dialog: NewCharacterDialog,
    delete_character_dialog: DeleteCharacterDialog,
    message_box: MessageBox,
    
    // 对话框状态
    show_new_character: bool,
    show_delete_character: bool,
    show_message_box: bool,
    
    // 角色预览动画
    animation_frame: usize,
    animation_timer: f32,
    animation_delay: f32,
}

impl SelectScene {
    pub fn new(characters: Vec<CharacterInfo>) -> GameResult<Self> {
        let selected_index = if !characters.is_empty() { Some(0) } else { None };
        
        Ok(Self {
            characters,
            selected_index,
            
            new_character_dialog: NewCharacterDialog::new(),
            delete_character_dialog: DeleteCharacterDialog::new(),
            message_box: MessageBox::new_with_id("", "", MessageBoxButtons::Ok, "select_msgbox"),
            
            show_new_character: false,
            show_delete_character: false,
            show_message_box: false,
            
            animation_frame: 0,
            animation_timer: 0.0,
            animation_delay: 0.25,
        })
    }
    
    /// 显示消息框
    fn show_message(&mut self, title: &str, message: &str) {
        self.message_box.title = title.to_string();
        self.message_box.text = message.to_string();
        self.show_message_box = true;
    }
    
    /// 【macroquad 职责】绘制背景
    fn draw_background(&mut self) {
        // 背景 Prguse[65] - 背景图片从(0,0)开始,应用ImageInfo的偏移值
        if let Some(info) = LibraryName::Prguse.get_texture(65) {
            if let Some(ref texture) = info.image {
                draw_texture(texture, 0.0 + info.offset_x as f32, 0.0 + info.offset_y as f32, WHITE);
            }
        }
        
        // 标题 Title[40] at (468, 20)
        if let Some(info) = LibraryName::Title.get_texture(40) {
            if let Some(ref texture) = info.image {
                // 使用基础位置 + 偏移值
                draw_texture(texture, 468.0 + info.offset_x as f32, 20.0 + info.offset_y as f32, WHITE);
            }
        }
        
        // 服务器名称文字 "Legend of Mir 2" at (432, 60)
        let text = "Legend of Mir 2";
        let font_size = 17.0;
        let text_params = TextParams {
            font_size: font_size as u16,
            color: WHITE,
            ..Default::default()
        };
        let text_dims = measure_text(text, None, font_size as u16, 1.0);
        // 居中显示在155宽度的区域内
        let x = 432.0 + (155.0 - text_dims.width) / 2.0;
        let y = 60.0 + font_size;
        draw_text_ex(text, x, y, text_params);
    }
    
    /// 【macroquad 职责】绘制角色预览动画
    fn draw_character_preview(&mut self) {
        if let Some(selected_idx) = self.selected_index {
            if selected_idx < self.characters.len() {
                let character = &self.characters[selected_idx];
                
                // 计算角色预览帧索引
                // ChrSel 资源布局: 根据C#源码SelectScene.cs
                // 大部分职业: base = 20 + class*20 + gender*280
                // 但弓箭手(class=4)使用特殊索引：男100 / 女140
                let base_index = if character.class == 4 {
                    // 弓箭手特殊处理
                    if character.gender == 0 { 100 } else { 140 }
                } else {
                    // 其他职业使用通用公式
                    20 + (character.class as usize * 20) + (character.gender as usize * 280)
                };
                let frame_index = base_index + self.animation_frame;  // 动画帧0-15
                
                if let Some(info) = LibraryName::ChrSel.get_texture(frame_index) {
                    if let Some(ref texture) = info.image {
                        // C# 原版位置: (260, 420) + 偏移值
                        // 放大1.2倍显示，使用线性过滤
                        let scale = 1.2;
                        let x = 260.0 + info.offset_x as f32 * scale;
                        let y = 420.0 + info.offset_y as f32 * scale;
                        let w = texture.width() * scale;
                        let h = texture.height() * scale;
                        draw_texture_ex(texture, x, y, WHITE, DrawTextureParams {
                            dest_size: Some(Vec2::new(w, h)),
                            ..Default::default()
                        });
                        
                        // 光晕效果 (对于法师)
                        if character.class == 1 {
                            let glow_index = frame_index + 560;
                            if let Some(glow_info) = LibraryName::ChrSel.get_texture(glow_index) {
                                if let Some(ref glow_texture) = glow_info.image {
                                    let glow_x = 260.0 + glow_info.offset_x as f32 * scale;
                                    let glow_y = 420.0 + glow_info.offset_y as f32 * scale;
                                    let glow_w = glow_texture.width() * scale;
                                    let glow_h = glow_texture.height() * scale;
                                    draw_texture_ex(glow_texture, glow_x, glow_y, Color::new(1.0, 1.0, 1.0, 0.5), DrawTextureParams {
                                        dest_size: Some(Vec2::new(glow_w, glow_h)),
                                        ..Default::default()
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// 【macroquad 职责】绘制角色信息（Last Online等）
    fn draw_character_info(&mut self) {
        if let Some(selected_idx) = self.selected_index {
            if selected_idx < self.characters.len() {
                let character = &self.characters[selected_idx];
                
                // "Last Online:" 标签 at (200, 609)
                let label_text = "Last Online:";
                let label_x = 200.0;
                let label_y = 609.0;
                let font_size = 14.0;
                
                draw_text_ex(
                    label_text,
                    label_x,
                    label_y + font_size,
                    TextParams {
                        font_size: font_size as u16,
                        color: WHITE,
                        ..Default::default()
                    }
                );
                
                // 最后登录时间 at (265, 609)
                let time_x = 265.0;
                let time_y = 609.0;
                
                draw_text_ex(
                    &character.last_access,
                    time_x,
                    time_y + font_size,
                    TextParams {
                        font_size: font_size as u16,
                        color: WHITE,
                        ..Default::default()
                    }
                );
            }
        }
    }
    
    /// 【egui 职责】绘制角色按钮
    fn draw_character_buttons(&mut self, ctx: &egui::Context) {
        // C# 原版坐标: (637, 194), (637, 298), (637, 402), (637, 506)
        let positions = [(637.0, 194.0), (637.0, 298.0), (637.0, 402.0), (637.0, 506.0)];
        
        for (i, &(x, y)) in positions.iter().enumerate() {
            let has_character = i < self.characters.len();
            let is_selected = self.selected_index == Some(i);
            
            if has_character {
                let character = &self.characters[i];
                
                // 获取角色职业对应的纹理索引
                let base_index = match character.class {
                    0 => 660, // 战士
                    1 => 661, // 法师
                    2 => 662, // 道士
                    3 => 663, // 刺客
                    4 => 664, // 弓手
                    _ => 660,
                };
                let texture_index = if is_selected {
                    base_index + 5
                } else {
                    base_index
                };
                
                // 绘制角色按钮纹理
                if let Some(info) = LibraryName::Title.get_egui_texture(ctx, texture_index) {
                    if let Some(texture) = info.egui_texture {
                        let (w, h) = LibraryName::Title.get_size(texture_index).unwrap_or((280, 90));
                        
                        // 绘制纹理
                        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Middle, egui::Id::new(format!("char_btn_{}", i))));
                        let rect = egui::Rect::from_min_size(
                            egui::pos2(x, y),
                            egui::vec2(w as f32, h as f32)
                        );
                        painter.image(
                            texture.id(),
                            rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                        
                        // 检测点击
                        let clicked = ctx.input(|i| {
                            if i.pointer.any_click() {
                                if let Some(pos) = i.pointer.interact_pos() {
                                    if rect.contains(pos) {
                                        return true;
                                    }
                                }
                            }
                            false
                        });
                        
                        if clicked {
                            self.selected_index = Some(i);
                            println!("✅ 选择角色 {}: {}", i, character.name);
                        }
                        
                        // 绘制文字（使用Middle层级，避免遮挡对话框）
                        let text_painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Middle, egui::Id::new(format!("char_text_{}", i))));
                        
                        // 名称
                        text_painter.text(
                            egui::pos2(x + 107.0, y + 9.0 + 9.0 - 2.5),
                            egui::Align2::LEFT_CENTER,
                            &character.name,
                            egui::FontId::proportional(13.0),
                            egui::Color32::from_rgb(220, 220, 220),
                        );
                        
                        // 等级
                        text_painter.text(
                            egui::pos2(x + 107.0, y + 28.0 + 9.0 - 2.5),
                            egui::Align2::LEFT_CENTER,
                            format!("{}", character.level),
                            egui::FontId::proportional(11.0),
                            egui::Color32::from_rgb(200, 200, 200),
                        );
                        
                        // 职业
                        let class_name = match character.class {
                            0 => "战士",
                            1 => "法师",
                            2 => "道士",
                            3 => "刺客",
                            4 => "弓手",
                            _ => "未知",
                        };
                        text_painter.text(
                            egui::pos2(x + 178.0, y + 28.0 + 9.0 - 2.5),
                            egui::Align2::LEFT_CENTER,
                            class_name,
                            egui::FontId::proportional(11.0),
                            egui::Color32::from_rgb(200, 200, 200),
                        );
                    }
                }
            } else {
                // 空槽位 - Prguse[44]
                if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 44) {
                    if let Some(texture) = info.egui_texture {
                        let (w, h) = LibraryName::Prguse.get_size(44).unwrap_or((280, 90));
                        
                        // 绘制纹理
                        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Middle, egui::Id::new(format!("empty_btn_{}", i))));
                        let rect = egui::Rect::from_min_size(
                            egui::pos2(x, y),
                            egui::vec2(w as f32, h as f32)
                        );
                        painter.image(
                            texture.id(),
                            rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                        
                        // 检测点击
                        let clicked = ctx.input(|i| {
                            if i.pointer.any_click() {
                                if let Some(pos) = i.pointer.interact_pos() {
                                    if rect.contains(pos) {
                                        return true;
                                    }
                                }
                            }
                            false
                        });
                        
                        if clicked {
                            println!("✅ 点击空槽位 {}", i);
                            if self.characters.len() < 4 {
                                self.show_new_character = true;
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// 【egui 职责】绘制底部按钮
    fn draw_bottom_buttons(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        // 计算按钮位置（C# 原版逻辑）
        let screen_w = screen_width() / screen_dpi_scale();
        let x_point = (screen_w - 200.0) / 5.0;
        let y = screen_height() / screen_dpi_scale() - 32.0;
        
        // 开始游戏 Title[340-342]
        if self.selected_index.is_some() {
            if self.draw_image_button(ui, ctx, LibraryName::Title, 340, 341, 342, 
                egui::pos2(100.0 + x_point - x_point / 2.0 - 50.0, y)) {
                println!("🎮 开始游戏");
                // TODO: 进入游戏
            }
        }
        
        // 新建角色 Title[343-345]
        if self.draw_image_button(ui, ctx, LibraryName::Title, 343, 344, 345,
            egui::pos2(100.0 + x_point * 2.0 - x_point / 2.0 - 50.0, y)) {
            if self.characters.len() < 4 {
                println!("📝 打开创建角色对话框");
                self.show_new_character = true;
            } else {
                self.show_message("提示", "最多只能创建4个角色！");
            }
        }
        
        // 删除角色 Title[346-348]
        if self.draw_image_button(ui, ctx, LibraryName::Title, 346, 347, 348,
            egui::pos2(100.0 + x_point * 3.0 - x_point / 2.0 - 50.0, y)) {
            if let Some(idx) = self.selected_index {
                if idx < self.characters.len() {
                    let character = &self.characters[idx];
                    println!("🗑️ 开始删除角色: {}", character.name);
                    self.delete_character_dialog.start_delete(
                        character.name.clone(),
                        character.index
                    );
                    self.show_delete_character = true;
                }
            }
        }
        
        // 制作名单 Title[349-351]
        if self.draw_image_button(ui, ctx, LibraryName::Title, 349, 350, 351,
            egui::pos2(100.0 + x_point * 4.0 - x_point / 2.0 - 50.0, y)) {
            println!("📜 制作名单");
        }
        
        // 退出 Title[352-354]
        if self.draw_image_button(ui, ctx, LibraryName::Title, 352, 353, 354,
            egui::pos2(100.0 + x_point * 5.0 - x_point / 2.0 - 50.0, y)) {
            println!("🚪 退出游戏");
            std::process::exit(0);
        }
    }
    
    /// 绘制图像按钮
    fn draw_image_button(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        lib_name: LibraryName,
        normal_idx: usize,
        hover_idx: usize,
        pressed_idx: usize,
        pos: egui::Pos2,
    ) -> bool {
        if let Some(info) = lib_name.get_egui_texture(ctx, normal_idx) {
            if let Some(handle) = info.egui_texture {
                let size = egui::vec2(handle.size()[0] as f32, handle.size()[1] as f32);
                let button_rect = egui::Rect::from_min_size(pos, size);
                let response = ui.allocate_rect(button_rect, egui::Sense::click());
                
                let texture_idx = if response.is_pointer_button_down_on() {
                    pressed_idx
                } else if response.hovered() {
                    hover_idx
                } else {
                    normal_idx
                };
                
                if let Some(btn_info) = lib_name.get_egui_texture(ctx, texture_idx) {
                    if let Some(btn_handle) = btn_info.egui_texture {
                        ui.painter().image(
                            btn_handle.id(),
                            button_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE
                        );
                    }
                }
                
                return response.clicked();
            }
        }
        
        false
    }
    
    /// 处理创建角色
    fn on_create_character(&mut self) {
        if self.new_character_dialog.name.trim().is_empty() {
            self.show_message("错误", "请输入角色名称！");
            return;
        }
        
        if self.new_character_dialog.name.chars().count() < 2 {
            self.show_message("错误", "角色名称至少需2个字符！");
            return;
        }
        
        if self.new_character_dialog.name.chars().count() > 16 {
            self.show_message("错误", "角色名称最多16个字符！");
            return;
        }
        
        // 创建新角色
        let new_char = CharacterInfo {
            index: self.characters.len() as i32,
            name: self.new_character_dialog.name.clone(),
            level: 1,
            class: self.new_character_dialog.class,
            gender: self.new_character_dialog.gender,
            last_access: "刚刚".to_string(),
        };
        
        self.characters.push(new_char);
        
        let class_name = match self.new_character_dialog.class {
            0 => "战士",
            1 => "法师",
            2 => "道士",
            3 => "刺客",
            4 => "弓手",
            _ => "未知",
        };
        let gender_name = if self.new_character_dialog.gender == 0 { "男" } else { "女" };
        
        println!("🎭 创建角色成功: {} ({}{}) Lv.1", self.new_character_dialog.name, gender_name, class_name);
        
        // 重置对话框并关闭
        self.new_character_dialog.reset();
        self.show_new_character = false;
    }
    
    /// 处理删除角色
    fn on_delete_character(&mut self, character_index: i32) {
        // 查找并删除角色
        if let Some(pos) = self.characters.iter().position(|c| c.index == character_index) {
            let character_name = self.characters[pos].name.clone();
            self.characters.remove(pos);
            
            println!("✅ 角色已删除: {}", character_name);
            self.show_message("成功", &format!("角色 {} 已成功删除", character_name));
            
            // 更新选中索引
            if self.characters.is_empty() {
                self.selected_index = None;
            } else if let Some(idx) = self.selected_index {
                if idx >= self.characters.len() {
                    self.selected_index = Some(self.characters.len() - 1);
                }
            }
        }
        
        // 重置对话框并关闭
        self.delete_character_dialog.reset();
        self.show_delete_character = false;
    }
}

impl Scene for SelectScene {
    fn name(&self) -> &str {
        "CharacterSelect"
    }

    fn on_enter(&mut self) -> GameResult {
        // 使用 egui_macroquad::cfg() 配置字体和样式(一次性设置)
        egui_macroquad::cfg(|ctx| {
            let mut fonts = egui::FontDefinitions::default();
            
            // 加载中文字体
            let font_data = std::fs::read("assets/fonts/AlibabaPuHuiTi-3-55-Regular.ttf")
                .or_else(|_| std::fs::read("assets/fonts/Chinese.ttc"))
                .or_else(|_| {
                    // macOS 系统字体
                    #[cfg(target_os = "macos")]
                    {
                        std::fs::read("/System/Library/Fonts/PingFang.ttc")
                    }
                    #[cfg(not(target_os = "macos"))]
                    {
                        Err(std::io::Error::new(std::io::ErrorKind::NotFound, "No font"))
                    }
                })
                .or_else(|_| {
                    // Windows 系统字体
                    #[cfg(target_os = "windows")]
                    {
                        std::fs::read("C:\\Windows\\Fonts\\msyh.ttc")
                    }
                    #[cfg(not(target_os = "windows"))]
                    {
                        Err(std::io::Error::new(std::io::ErrorKind::NotFound, "No font"))
                    }
                })
                .unwrap_or_else(|_| {
                    println!("⚠ 无法加载中文字体，使用默认字体");
                    vec![]
                });
            
            if !font_data.is_empty() {
                fonts.font_data.insert(
                    "chinese".to_owned(),
                    Arc::new(egui::FontData::from_owned(font_data)),
                );
                
                // 设置字体优先级
                fonts.families.get_mut(&egui::FontFamily::Proportional)
                    .unwrap()
                    .insert(0, "chinese".to_owned());
                
                fonts.families.get_mut(&egui::FontFamily::Monospace)
                    .unwrap()
                    .insert(0, "chinese".to_owned());
            }
            
            ctx.set_fonts(fonts);
            
            // 设置 DPI 缩放 - 使 egui 与 macroquad 坐标系统对齐
            // macroquad 会根据系统 DPI 自动处理,egui 也需要同步
            let dpi_scale = screen_dpi_scale();
            ctx.set_pixels_per_point(dpi_scale);
            
            // 🎨 移除所有 egui 视觉风格 - 完全透明的UI
            let mut style = (*ctx.style()).clone();
            
            // 设置全局字体大小
            style.text_styles = [
                (egui::TextStyle::Heading, egui::FontId::new(24.0, egui::FontFamily::Proportional)),
                (egui::TextStyle::Body, egui::FontId::new(16.0, egui::FontFamily::Proportional)),
                (egui::TextStyle::Monospace, egui::FontId::new(14.0, egui::FontFamily::Monospace)),
                (egui::TextStyle::Button, egui::FontId::new(16.0, egui::FontFamily::Proportional)),
                (egui::TextStyle::Small, egui::FontId::new(12.0, egui::FontFamily::Proportional)),
            ].into();
            
            // 移除所有窗口/面板/按钮的背景和边框
            style.visuals.window_fill = egui::Color32::TRANSPARENT;
            style.visuals.window_stroke = egui::Stroke::NONE;
            style.visuals.panel_fill = egui::Color32::TRANSPARENT;
            style.visuals.window_shadow = egui::epaint::Shadow::NONE;
            
            // 移除所有组件的背景
            style.visuals.widgets.noninteractive.bg_fill = egui::Color32::TRANSPARENT;
            style.visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
            style.visuals.widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
            style.visuals.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
            
            // 移除所有边框
            style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
            style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
            style.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
            style.visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
            
            // 移除弹出菜单背景
            style.visuals.popup_shadow = egui::epaint::Shadow::NONE;
            style.visuals.extreme_bg_color = egui::Color32::TRANSPARENT;
            ctx.set_style(style);
        });
        
        println!("🎬 进入角色选择界面");
        Ok(())
    }
    
    fn on_exit(&mut self) -> GameResult {
        println!("🎬 离开角色选择界面");
        Ok(())
    }
    
    fn update(&mut self, dt: f32) -> GameResult<SceneTransition> {
        // 更新角色预览动画
        self.animation_timer += dt;
        if self.animation_timer >= self.animation_delay {
            self.animation_timer = 0.0;
            self.animation_frame = (self.animation_frame + 1) % 16;
        }
        
        // 更新新建角色对话框动画
        if self.show_new_character {
            self.new_character_dialog.update(dt);
        }
        
        Ok(SceneTransition::None)
    }
    
    fn render(&mut self) -> GameResult {
        clear_background(BLACK);
        
        // === macroquad 渲染层（从下到上绘制）===
        // 1. 背景和角色预览
        self.draw_background();
        self.draw_character_preview();
        self.draw_character_info();
        
        // === egui 交互层 ===
        egui_macroquad::ui(|ctx| {
            // 角色按钮
            self.draw_character_buttons(ctx);
            
            // 底部功能按钮
            egui::Area::new(egui::Id::new("bottom_buttons"))
                .fixed_pos(egui::pos2(0.0, 0.0))
                .show(ctx, |ui| {
                    self.draw_bottom_buttons(ctx, ui);
                });
            
            // 对话框组件
            self.new_character_dialog.show(ctx, &mut self.show_new_character);
            self.delete_character_dialog.show(ctx, &mut self.show_delete_character);
            self.message_box.show(ctx, &mut self.show_message_box);
            
            // 检查新建角色对话框事件
            let new_char_event = self.new_character_dialog.take_event();
            match new_char_event {
                NewCharacterEvent::Create => {
                    self.on_create_character();
                },
                NewCharacterEvent::Cancel => {
                    // 对话框取消，确保关闭状态
                    self.show_new_character = false;
                },
                NewCharacterEvent::None => {},
            }
            
            // 检查删除角色对话框事件
            let delete_event = self.delete_character_dialog.take_event();
            match delete_event {
                DeleteCharacterEvent::Delete(char_index) => {
                    self.on_delete_character(char_index);
                },
                DeleteCharacterEvent::Cancel => {
                    // 对话框取消，确保关闭状态
                    self.show_delete_character = false;
                    self.delete_character_dialog.reset();
                },
                DeleteCharacterEvent::None => {},
            }
        });
        
        egui_macroquad::draw();
        
        Ok(())
    }

    fn handle_input(&mut self) -> GameResult {
        // egui 已经在 render() 中处理了所有输入
        Ok(())
    }
}
