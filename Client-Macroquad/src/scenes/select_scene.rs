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
use crate::resources::mlibrary::MLibrary;
use crate::scenes::{Scene, SceneTransition};
use macroquad::prelude::*;
use egui_macroquad::egui;
use std::collections::HashMap;
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
    // 资源
    chrsel_lib: Option<MLibrary>,
    prguse_lib: Option<MLibrary>,
    title_lib: Option<MLibrary>,
    resources_loaded: bool,
    
    // egui 纹理缓存
    texture_cache: HashMap<String, egui::TextureHandle>,
    
    // 角色数据
    characters: Vec<CharacterInfo>,
    selected_index: Option<usize>,
    
    // 对话框状态
    show_new_character: bool,
    show_delete_confirm: bool,
    show_message_box: bool,
    message_box_title: String,
    message_box_text: String,
    
    // 新建角色表单
    new_char_name: String,
    new_char_class: u8,
    new_char_gender: u8,
    
    // 角色预览动画
    animation_frame: usize,
    animation_timer: f32,
    animation_delay: f32,
    
    // 创建角色对话框动画
    dialog_animation_frame: usize,
    dialog_animation_timer: f32,
}

impl SelectScene {
    pub fn new(characters: Vec<CharacterInfo>) -> GameResult<Self> {
        let selected_index = if !characters.is_empty() { Some(0) } else { None };
        
        Ok(Self {
            chrsel_lib: None,
            prguse_lib: None,
            title_lib: None,
            resources_loaded: false,
            texture_cache: HashMap::new(),
            
            characters,
            selected_index,
            
            show_new_character: false,
            show_delete_confirm: false,
            show_message_box: false,
            message_box_title: String::new(),
            message_box_text: String::new(),
            
            new_char_name: String::new(),
            new_char_class: 0,
            new_char_gender: 0,
            
            animation_frame: 0,
            animation_timer: 0.0,
            animation_delay: 0.25,
            
            dialog_animation_frame: 0,
            dialog_animation_timer: 0.0,
        })
    }
    
    /// 加载资源
    fn load_resources(&mut self) -> GameResult {
        println!("📦 加载角色选择界面资源...");
        
        // ChrSel 库（角色预览）
        self.chrsel_lib = Some(MLibrary::open("Data/ChrSel")?);
        
        // Prguse 库（背景和UI元素）
        self.prguse_lib = Some(MLibrary::open("Data/Prguse")?);
        
        // Title 库（按钮）
        self.title_lib = Some(MLibrary::open("Data/Title")?);
        
        if let Some(ref lib) = self.chrsel_lib {
            println!("✓ Loaded ChrSel: {} images", lib.count());
        }
        if let Some(ref lib) = self.prguse_lib {
            println!("✓ Loaded Prguse: {} images", lib.count());
        }
        if let Some(ref lib) = self.title_lib {
            println!("✓ Loaded Title: {} images", lib.count());
        }
        
        self.resources_loaded = true;
        Ok(())
    }
    
    /// 获取或创建 egui 纹理
    fn get_or_create_egui_texture(
        ctx: &egui::Context,
        cache: &mut HashMap<String, egui::TextureHandle>,
        lib: &mut MLibrary,
        lib_name: &str,
        index: usize,
    ) -> Option<egui::TextureHandle> {
        let key = format!("{}_{}", lib_name, index);
        
        if let Some(handle) = cache.get(&key) {
            return Some(handle.clone());
        }
        
        if let Ok(info) = lib.get_or_create_texture(index) {
            if let Some(ref texture) = info.image {
                let image_data = texture.get_texture_data();
                let width = texture.width() as usize;
                let height = texture.height() as usize;
                
                let mut pixels = Vec::with_capacity(width * height);
                for y in 0..height {
                    for x in 0..width {
                        let idx = (y * width + x) * 4;
                        let r = image_data.bytes[idx];
                        let g = image_data.bytes[idx + 1];
                        let b = image_data.bytes[idx + 2];
                        let a = image_data.bytes[idx + 3];
                        pixels.push(egui::Color32::from_rgba_unmultiplied(r, g, b, a));
                    }
                }
                
                let color_image = egui::ColorImage {
                    size: [width, height],
                    pixels,
                };
                
                let handle = ctx.load_texture(
                    &key,
                    color_image,
                    egui::TextureOptions {
                        magnification: egui::TextureFilter::Nearest,
                        minification: egui::TextureFilter::Linear,
                        ..Default::default()
                    },
                );
                cache.insert(key.clone(), handle.clone());
                return Some(handle);
            }
        }
        
        None
    }
    
    /// 显示消息框
    fn show_message(&mut self, title: &str, message: &str) {
        self.message_box_title = title.to_string();
        self.message_box_text = message.to_string();
        self.show_message_box = true;
    }
    
    /// 【macroquad 职责】绘制背景
    fn draw_background(&mut self) {
        // 背景 Prguse[65] - 背景图片从(0,0)开始,应用ImageInfo的偏移值
        if let Some(ref mut lib) = self.prguse_lib {
            if let Ok(info) = lib.get_or_create_texture(65) {
                if let Some(ref texture) = info.image {
                    draw_texture(texture, 0.0 + info.x as f32, 0.0 + info.y as f32, WHITE);
                }
            }
        }
        
        // 标题 Title[40] at (468, 20)
        if let Some(ref mut lib) = self.title_lib {
            if let Ok(info) = lib.get_or_create_texture(40) {
                if let Some(ref texture) = info.image {
                    // 使用基础位置 + 偏移值
                    draw_texture(texture, 468.0 + info.x as f32, 20.0 + info.y as f32, WHITE);
                }
            }
        }
    }
    
    /// 【macroquad 职责】绘制角色预览动画
    fn draw_character_preview(&mut self) {
        if let Some(selected_idx) = self.selected_index {
            if selected_idx < self.characters.len() {
                let character = &self.characters[selected_idx];
                
                // 计算角色预览帧索引
                // ChrSel 资源布局: 每个职业相差20, 男女相差280
                // 战士: 男20, 女300
                // 法师: 男40, 女320  
                // 道士: 男60, 女340
                // 刺客: 男80, 女360
                // 弓手: 男100, 女380
                let base_index = 20 + (character.class as usize * 20) + (character.gender as usize * 280);
                let frame_index = base_index + self.animation_frame;  // 动画帧0-15
                
                if let Some(ref mut lib) = self.chrsel_lib {
                    if let Ok(info) = lib.get_or_create_texture(frame_index) {
                        if let Some(ref texture) = info.image {
                            // C# 原版位置: (260, 420) + 偏移值
                            draw_texture(texture, 260.0 + info.x as f32, 420.0 + info.y as f32, WHITE);
                            
                            // 光晕效果 (对于法师)
                            if character.class == 1 {
                                let glow_index = frame_index + 560;
                                if let Ok(glow_info) = lib.get_or_create_texture(glow_index) {
                                    if let Some(ref glow_texture) = glow_info.image {
                                        draw_texture(glow_texture, 260.0 + glow_info.x as f32, 420.0 + glow_info.y as f32, Color::new(1.0, 1.0, 1.0, 0.5));
                                    }
                                }
                            }
                        }
                    }
                }
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
                if let Some(ref mut lib) = self.title_lib {
                    if let Some(texture) = Self::get_or_create_egui_texture(ctx, &mut self.texture_cache, lib, "title", texture_index) {
                        let (w, h) = lib.get_size(texture_index).unwrap_or((280, 90));
                        
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
                        
                        // 绘制文字（Y坐标向上移动2.5px）
                        let text_painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("char_text")));
                        
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
                if let Some(ref mut lib) = self.prguse_lib {
                    if let Some(texture) = Self::get_or_create_egui_texture(ctx, &mut self.texture_cache, lib, "prguse", 44) {
                        let (w, h) = lib.get_size(44).unwrap_or((280, 90));
                        
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
            if self.draw_image_button(ui, ctx, "title", 340, 341, 342, 
                egui::pos2(100.0 + x_point - x_point / 2.0 - 50.0, y)) {
                println!("🎮 开始游戏");
                // TODO: 进入游戏
            }
        }
        
        // 新建角色 Title[343-345]
        if self.draw_image_button(ui, ctx, "title", 343, 344, 345,
            egui::pos2(100.0 + x_point * 2.0 - x_point / 2.0 - 50.0, y)) {
            if self.characters.len() < 4 {
                println!("📝 打开创建角色对话框");
                self.show_new_character = true;
            } else {
                self.show_message("提示", "最多只能创建4个角色！");
            }
        }
        
        // 删除角色 Title[346-348]
        if self.draw_image_button(ui, ctx, "title", 346, 347, 348,
            egui::pos2(100.0 + x_point * 3.0 - x_point / 2.0 - 50.0, y)) {
            if self.selected_index.is_some() {
                self.show_delete_confirm = true;
            }
        }
        
        // 制作名单 Title[349-351]
        if self.draw_image_button(ui, ctx, "title", 349, 350, 351,
            egui::pos2(100.0 + x_point * 4.0 - x_point / 2.0 - 50.0, y)) {
            println!("📜 制作名单");
        }
        
        // 退出 Title[352-354]
        if self.draw_image_button(ui, ctx, "title", 352, 353, 354,
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
        lib_name: &str,
        normal_idx: usize,
        hover_idx: usize,
        pressed_idx: usize,
        pos: egui::Pos2,
    ) -> bool {
        let lib = match lib_name {
            "title" => self.title_lib.as_mut(),
            "prguse" => self.prguse_lib.as_mut(),
            _ => None,
        };
        
        if let Some(lib) = lib {
            if let Some(handle) = Self::get_or_create_egui_texture(ctx, &mut self.texture_cache, lib, lib_name, normal_idx) {
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
                
                if let Some(btn_handle) = Self::get_or_create_egui_texture(ctx, &mut self.texture_cache, lib, lib_name, texture_idx) {
                    ui.painter().image(
                        btn_handle.id(),
                        button_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE
                    );
                }
                
                return response.clicked();
            }
        }
        
        false
    }
    
    /// 绘制消息框
    fn draw_message_box(&mut self, ctx: &egui::Context) {
        let (dialog_w, dialog_h) = if let Some(lib) = self.prguse_lib.as_mut() {
            if let Ok(size) = lib.get_size(360) {
                (size.0 as f32, size.1 as f32)
            } else {
                (460.0, 200.0)
            }
        } else {
            (460.0, 200.0)
        };
        
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_message_box = false;
            return;
        }
        
        egui::Area::new(egui::Id::new("message_box"))
            .default_pos(egui::pos2(
                (screen_width() / screen_dpi_scale() - dialog_w) / 2.0,
                (screen_height() / screen_dpi_scale() - dialog_h) / 2.0
            ))
            .interactable(true)
            .movable(true)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let rect = ui.allocate_rect(
                    egui::Rect::from_min_size(ui.cursor().min, egui::vec2(dialog_w, dialog_h)),
                    egui::Sense::hover()
                ).rect;
                
                if let Some(lib) = self.prguse_lib.as_mut() {
                    if let Some(handle) = Self::get_or_create_egui_texture(ctx, &mut self.texture_cache, lib, "prguse", 360) {
                        ui.painter().image(
                            handle.id(),
                            rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE
                        );
                    }
                }
                
                ui.put(
                    egui::Rect::from_min_size(egui::pos2(rect.min.x + 35.0, rect.min.y + 35.0), egui::vec2(390.0, 25.0)),
                    egui::Label::new(
                        egui::RichText::new(&self.message_box_title)
                            .color(egui::Color32::from_rgb(255, 200, 100))
                            .size(12.0)
                    )
                );
                
                ui.put(
                    egui::Rect::from_min_size(egui::pos2(rect.min.x + 35.0, rect.min.y + 60.0), egui::vec2(390.0, 80.0)),
                    egui::Label::new(
                        egui::RichText::new(&self.message_box_text)
                            .color(egui::Color32::WHITE)
                            .size(10.0)
                    )
                );
                
                if self.draw_image_button(ui, ctx, "title", 200, 201, 202, egui::pos2(rect.min.x + 360.0, rect.min.y + 157.0)) {
                    self.show_message_box = false;
                }
            });
    }
    
    /// 【egui 职责】绘制创建角色对话框
    fn draw_new_character_dialog(&mut self, ctx: &egui::Context) {
        println!("🎨 绘制创建角色对话框");
        let screen_w = screen_width() / screen_dpi_scale();
        let screen_h = screen_height() / screen_dpi_scale();
        
        // 对话框尺寸 (Prguse[73])
        let dialog_w = 656.0;
        let dialog_h = 537.0;
        let dialog_x = (screen_w - dialog_w) / 2.0;
        let dialog_y = (screen_h - dialog_h) / 2.0;
        
        // 绘制对话框背景
        if let Some(ref mut lib) = self.prguse_lib {
            if let Some(texture) = Self::get_or_create_egui_texture(ctx, &mut self.texture_cache, lib, "prguse", 73) {
                let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("new_char_bg")));
                let rect = egui::Rect::from_min_size(
                    egui::pos2(dialog_x, dialog_y),
                    egui::vec2(dialog_w, dialog_h)
                );
                painter.image(
                    texture.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        }
        
        // 绘制标题 Title[20]
        if let Some(ref mut lib) = self.title_lib {
            if let Ok(img_info) = lib.get_or_create_texture(20) {
                if let Some(texture) = Self::get_or_create_egui_texture(ctx, &mut self.texture_cache, lib, "title", 20) {
                    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("new_char_title")));
                    // 应用ImageInfo偏移
                    let rect = egui::Rect::from_min_size(
                        egui::pos2(dialog_x + (dialog_w - img_info.width as f32) / 2.0 + img_info.x as f32, dialog_y + 12.0 + img_info.y as f32),
                        egui::vec2(img_info.width as f32, img_info.height as f32)
                    );
                painter.image(
                    texture.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
            }
        }
        
        // 绘制角色预览动画
        self.draw_dialog_character_preview(ctx, dialog_x, dialog_y);
        
        // 5个职业按钮: 战士/法师/道士/刺客/弓手
        let class_buttons = [
            (0, 323.0, 296.0, 2426, 2427), // 战士
            (1, 373.0, 296.0, 2429, 2430), // 法师
            (2, 423.0, 296.0, 2432, 2433), // 道士
            (3, 473.0, 296.0, 2435, 2436), // 刺客
            (4, 523.0, 296.0, 2438, 2439), // 弓手
        ];
        
        for (class_id, btn_x, btn_y, normal_idx, selected_idx) in class_buttons {
            let idx = if self.new_char_class == class_id { selected_idx } else { normal_idx };
            if let Some(ref mut lib) = self.prguse_lib {
                if let Ok(img_info) = lib.get_or_create_texture(idx) {
                    if let Some(texture) = Self::get_or_create_egui_texture(ctx, &mut self.texture_cache, lib, "prguse", idx) {
                        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new(format!("class_btn_{}", class_id))));
                        // 应用ImageInfo的偏移
                        let rect = egui::Rect::from_min_size(
                            egui::pos2(dialog_x + btn_x + img_info.x as f32, dialog_y + btn_y + img_info.y as f32),
                            egui::vec2(img_info.width as f32, img_info.height as f32)
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
                        self.new_char_class = class_id;
                    }
                }
                }
            }
        }
        
        // 2个性别按钮: 男/女
        let gender_buttons = [
            (0, 323.0, 343.0, 2420, 2421), // 男
            (1, 373.0, 343.0, 2423, 2424), // 女
        ];
        
        for (gender_id, btn_x, btn_y, normal_idx, selected_idx) in gender_buttons {
            let idx = if self.new_char_gender == gender_id { selected_idx } else { normal_idx };
            if let Some(ref mut lib) = self.prguse_lib {
                if let Ok(img_info) = lib.get_or_create_texture(idx) {
                    if let Some(texture) = Self::get_or_create_egui_texture(ctx, &mut self.texture_cache, lib, "prguse", idx) {
                        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new(format!("gender_btn_{}", gender_id))));
                        // 应用ImageInfo的偏移
                        let rect = egui::Rect::from_min_size(
                            egui::pos2(dialog_x + btn_x + img_info.x as f32, dialog_y + btn_y + img_info.y as f32),
                            egui::vec2(img_info.width as f32, img_info.height as f32)
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
                        self.new_char_gender = gender_id;
                    }
                }
                }
            }
        }
        
        // 输入框区域
        egui::Area::new(egui::Id::new("char_name_input"))
            .fixed_pos(egui::pos2(dialog_x + 325.0, dialog_y + 265.0))
            .show(ctx, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.new_char_name)
                        .desired_width(240.0)
                        .font(egui::TextStyle::Body)
                );
            });
        
        // OK 按钮 Title[360]
        egui::Area::new(egui::Id::new("ok_button"))
            .fixed_pos(egui::pos2(0.0, 0.0))
            .show(ctx, |ui| {
                if self.draw_image_button(ui, ctx, "title", 360, 361, 362, egui::pos2(dialog_x + 160.0, dialog_y + 425.0)) {
                    self.handle_create_character();
                }
            });
        
        // Cancel 按钮 Title[280]
        egui::Area::new(egui::Id::new("cancel_button"))
            .fixed_pos(egui::pos2(0.0, 0.0))
            .show(ctx, |ui| {
                if self.draw_image_button(ui, ctx, "title", 280, 281, 282, egui::pos2(dialog_x + 425.0, dialog_y + 425.0)) {
                    self.new_char_name.clear();
                    self.new_char_class = 0;
                    self.new_char_gender = 0;
                    self.show_new_character = false;
                }
            });
    }
    
    /// 绘制对话框中的角色预览
    fn draw_dialog_character_preview(&mut self, ctx: &egui::Context, dialog_x: f32, dialog_y: f32) {
        // 计算角色动画索引
        let base_index = 20 + (self.new_char_class as usize * 20) + (self.new_char_gender as usize * 280);
        let frame_index = base_index + self.dialog_animation_frame;
        
        if let Some(ref mut lib) = self.chrsel_lib {
            if let Ok(info) = lib.get_or_create_texture(frame_index) {
                // 转换为 egui 纹理
                let cache_key = format!("chrsel_{}", frame_index);
                if !self.texture_cache.contains_key(&cache_key) {
                    if let Some(rgba_data) = info.get_bgra_data() {
                        let color_image = egui::ColorImage::from_rgba_unmultiplied(
                            [info.width as usize, info.height as usize],
                            rgba_data
                        );
                        let handle = ctx.load_texture(&cache_key, color_image, egui::TextureOptions::default());
                        self.texture_cache.insert(cache_key.clone(), handle);
                    }
                }
                
                if let Some(egui_texture) = self.texture_cache.get(&cache_key) {
                    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("char_preview")));
                    let rect = egui::Rect::from_min_size(
                        egui::pos2(dialog_x + 176.0 - info.x as f32, dialog_y + 270.0 - info.y as f32),
                        egui::vec2(info.width as f32, info.height as f32)
                    );
                    painter.image(
                        egui_texture.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                    
                    // 法师光效
                    if self.new_char_class == 1 {
                        let effect_index = frame_index + 560;
                        if let Ok(effect_info) = lib.get_or_create_texture(effect_index) {
                            let effect_key = format!("chrsel_{}", effect_index);
                            if !self.texture_cache.contains_key(&effect_key) {
                                if let Some(rgba_data) = effect_info.get_bgra_data() {
                                    let effect_color = egui::ColorImage::from_rgba_unmultiplied(
                                        [effect_info.width as usize, effect_info.height as usize],
                                        rgba_data
                                    );
                                    let effect_handle = ctx.load_texture(&effect_key, effect_color, egui::TextureOptions::default());
                                    self.texture_cache.insert(effect_key.clone(), effect_handle);
                                }
                            }
                            
                            if let Some(effect_tex) = self.texture_cache.get(&effect_key) {
                                let effect_rect = egui::Rect::from_min_size(
                                    egui::pos2(dialog_x + 176.0 - effect_info.x as f32, dialog_y + 270.0 - effect_info.y as f32),
                                    egui::vec2(effect_info.width as f32, effect_info.height as f32)
                                );
                                painter.image(
                                    effect_tex.id(),
                                    effect_rect,
                                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 180),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// 处理创建角色
    fn handle_create_character(&mut self) {
        if self.new_char_name.trim().is_empty() {
            self.show_message("错误", "请输入角色名称！");
            return;
        }
        
        if self.new_char_name.chars().count() < 2 {
            self.show_message("错误", "角色名称至少需2个字符！");
            return;
        }
        
        if self.new_char_name.chars().count() > 16 {
            self.show_message("错误", "角色名称最多16个字符！");
            return;
        }
        
        // 创建新角色
        let new_char = CharacterInfo {
            index: self.characters.len() as i32,
            name: self.new_char_name.clone(),
            level: 1,
            class: self.new_char_class,
            gender: self.new_char_gender,
            last_access: "刚刚".to_string(),
        };
        
        self.characters.push(new_char);
        
        let class_name = match self.new_char_class {
            0 => "战士",
            1 => "法师",
            2 => "道士",
            3 => "刺客",
            4 => "弓手",
            _ => "未知",
        };
        let gender_name = if self.new_char_gender == 0 { "男" } else { "女" };
        
        println!("🎭 创建角色成功: {} ({}{}) Lv.1", self.new_char_name, gender_name, class_name);
        
        // 清空表单
        self.new_char_name.clear();
        self.new_char_class = 0;
        self.new_char_gender = 0;
        self.show_new_character = false;
        self.dialog_animation_frame = 0;
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
        if !self.resources_loaded {
            self.load_resources()?;
        }
        
        // 更新角色预览动画
        self.animation_timer += dt;
        if self.animation_timer >= self.animation_delay {
            self.animation_timer = 0.0;
            self.animation_frame = (self.animation_frame + 1) % 16;
        }
        
        // 更新对话框角色动画
        if self.show_new_character {
            self.dialog_animation_timer += dt;
            if self.dialog_animation_timer >= 0.1 {
                self.dialog_animation_timer = 0.0;
                self.dialog_animation_frame = (self.dialog_animation_frame + 1) % 16;
            }
        }
        
        Ok(SceneTransition::None)
    }
    
    fn render(&mut self) -> GameResult {
        clear_background(BLACK);
        
        // macroquad 渲染层
        self.draw_background();
        self.draw_character_preview();
        
        // egui 交互层
        egui_macroquad::ui(|ctx| {
            // 角色按钮
            self.draw_character_buttons(ctx);
            
            // 底部功能按钮
            egui::Area::new(egui::Id::new("bottom_buttons"))
                .fixed_pos(egui::pos2(0.0, 0.0))
                .show(ctx, |ui| {
                    self.draw_bottom_buttons(ctx, ui);
                });
            
            // 消息框
            if self.show_message_box {
                self.draw_message_box(ctx);
            }
            
            // 创建角色对话框
            if self.show_new_character {
                self.draw_new_character_dialog(ctx);
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
