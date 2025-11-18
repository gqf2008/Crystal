// LoginScene - 登录界面 (完全使用 egui 渲染)
// 对应 C# Client/MirScenes/LoginScene.cs

use crate::game::GameResult;
use crate::scenes::{SceneHandler, SceneTransition};
use crate::resources::mlibrary::MLibrary;
use macroquad::prelude::*;
use egui_macroquad::egui;
use std::path::PathBuf;
use std::collections::HashMap;

/// 登录场景 - 完全使用 egui 渲染版本
pub struct LoginScene {
    // UI 状态
    account_input: String,
    password_input: String,
    
    // 新建账号对话框状态
    show_new_account: bool,
    new_account_id: String,
    new_password1: String,
    new_password2: String,
    new_email: String,
    new_username: String,
    new_birthdate: String,
    new_question: String,
    new_answer: String,
    
    // 资源库
    chrsel_lib: Option<MLibrary>,
    title_lib: Option<MLibrary>,
    prguse_lib: Option<MLibrary>,
    
    // egui 纹理缓存 (key: "lib_index", value: egui TextureHandle)
    texture_cache: HashMap<String, egui::TextureHandle>,
    
    // 背景动画
    background_frame: usize,
    animation_playing: bool,
    frame_timer: f32,
    frame_delay: f32,
    
    // 状态
    show_login_dialog: bool,
    resources_loaded: bool,
    version_text: String,
}

impl LoginScene {
    pub fn new() -> Self {
        Self {
            account_input: String::new(),
            password_input: String::new(),
            show_new_account: false,
            new_account_id: String::new(),
            new_password1: String::new(),
            new_password2: String::new(),
            new_email: String::new(),
            new_username: String::new(),
            new_birthdate: String::new(),
            new_question: String::new(),
            new_answer: String::new(),
            
            chrsel_lib: None,
            title_lib: None,
            prguse_lib: None,
            texture_cache: HashMap::new(),
            
            background_frame: 0,
            animation_playing: false,
            frame_timer: 0.0,
            frame_delay: 0.1,
            
            show_login_dialog: false,
            resources_loaded: false,
            version_text: format!("Build: Crystal-Rust v{}", env!("CARGO_PKG_VERSION")),
        }
    }
    
    /// 加载资源
    fn load_resources(&mut self) -> GameResult {
        let data_path = PathBuf::from("Data");
        
        // 加载 ChrSel (背景动画)
        match MLibrary::open(data_path.join("ChrSel")) {
            Ok(lib) => {
                println!("✓ Loaded ChrSel: {} images", lib.count());
                self.chrsel_lib = Some(lib);
            }
            Err(e) => println!("✗ Failed to load ChrSel: {}", e),
        }
        
        // 加载 Title (标题/按钮)
        match MLibrary::open(data_path.join("Title")) {
            Ok(lib) => {
                println!("✓ Loaded Title: {} images", lib.count());
                self.title_lib = Some(lib);
            }
            Err(e) => println!("✗ Failed to load Title: {}", e),
        }
        
        // 加载 Prguse (UI元素)
        match MLibrary::open(data_path.join("Prguse")) {
            Ok(lib) => {
                println!("✓ Loaded Prguse: {} images", lib.count());
                self.prguse_lib = Some(lib);
            }
            Err(e) => println!("✗ Failed to load Prguse: {}", e),
        }
        
        self.resources_loaded = true;
        self.show_login_dialog = true;
        
        Ok(())
    }
    
    /// 将 macroquad 纹理转换为 egui ColorImage
    fn texture_to_color_image(texture: &Texture2D) -> egui::ColorImage {
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
        
        egui::ColorImage {
            size: [width, height],
            pixels,
        }
    }
    
    /// 获取或创建 egui 纹理
    fn get_or_create_egui_texture(
        &mut self,
        ctx: &egui::Context,
        lib: &mut MLibrary,
        lib_name: &str,
        index: usize,
    ) -> Option<egui::TextureHandle> {
        let key = format!("{}_{}", lib_name, index);
        
        // 检查缓存
        if let Some(handle) = self.texture_cache.get(&key) {
            return Some(handle.clone());
        }
        
        // 从库中加载纹理
        if let Ok(info) = lib.get_or_create_texture(index) {
            if let Some(ref texture) = info.image {
                let color_image = Self::texture_to_color_image(texture);
                let handle = ctx.load_texture(
                    &key,
                    color_image,
                    egui::TextureOptions {
                        magnification: egui::TextureFilter::Nearest,
                        minification: egui::TextureFilter::Linear,
                        ..Default::default()
                    },
                );
                self.texture_cache.insert(key.clone(), handle.clone());
                return Some(handle);
            }
        }
        
        None
    }
    
    /// 绘制登录UI (完全使用 egui)
    fn draw_login_ui(&mut self, ctx: &egui::Context) {
        let screen_w = screen_width();
        let screen_h = screen_height();
        let dialog_w = 328.0;
        let dialog_h = 220.0;
        let dialog_x = (screen_w - dialog_w) / 2.0;
        let dialog_y = (screen_h - dialog_h) / 2.0;
        
        egui::Window::new("login_dialog")
            .title_bar(false)
            .resizable(false)
            .movable(false)
            .frame(egui::Frame::none())
            .fixed_pos([dialog_x, dialog_y])
            .fixed_size([dialog_w, dialog_h])
            .show(ctx, |ui| {
                // 1. 绘制对话框背景 (Prguse 1084)
                if let Some(ref mut lib) = self.prguse_lib {
                    if let Some(handle) = self.get_or_create_egui_texture(ctx, lib, "prguse", 1084) {
                        ui.put(
                            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(dialog_w, dialog_h)),
                            egui::Image::new(&handle).fit_to_exact_size(egui::vec2(dialog_w, dialog_h))
                        );
                    }
                }
                
                // 2. 绘制标题 (Title 30)
                if let Some(ref mut lib) = self.title_lib {
                    if let Some(handle) = self.get_or_create_egui_texture(ctx, lib, "title", 30) {
                        let title_w = handle.size()[0] as f32;
                        let title_h = handle.size()[1] as f32;
                        let title_x = (dialog_w - title_w) / 2.0;
                        ui.put(
                            egui::Rect::from_min_size(egui::pos2(title_x, 12.0), egui::vec2(title_w, title_h)),
                            egui::Image::new(&handle).fit_to_exact_size(egui::vec2(title_w, title_h))
                        );
                    }
                }
                
                // 3. 绘制账号标签 (Title 31)
                if let Some(ref mut lib) = self.title_lib {
                    if let Some(handle) = self.get_or_create_egui_texture(ctx, lib, "title", 31) {
                        let w = handle.size()[0] as f32;
                        let h = handle.size()[1] as f32;
                        ui.put(
                            egui::Rect::from_min_size(egui::pos2(52.0, 83.0), egui::vec2(w, h)),
                            egui::Image::new(&handle).fit_to_exact_size(egui::vec2(w, h))
                        );
                    }
                }
                
                // 4. 绘制密码标签 (Title 32)
                if let Some(ref mut lib) = self.title_lib {
                    if let Some(handle) = self.get_or_create_egui_texture(ctx, lib, "title", 32) {
                        let w = handle.size()[0] as f32;
                        let h = handle.size()[1] as f32;
                        ui.put(
                            egui::Rect::from_min_size(egui::pos2(43.0, 105.0), egui::vec2(w, h)),
                            egui::Image::new(&handle).fit_to_exact_size(egui::vec2(w, h))
                        );
                    }
                }
                
                // 5. 绘制输入框（支持中文输入）
                ui.put(
                    egui::Rect::from_min_size(egui::pos2(85.0, 82.0), egui::vec2(136.0, 18.0)),
                    egui::TextEdit::singleline(&mut self.account_input)
                        .desired_width(136.0)
                        .hint_text("请输入账号") // 添加提示文本
                );
                
                ui.put(
                    egui::Rect::from_min_size(egui::pos2(85.0, 104.0), egui::vec2(136.0, 18.0)),
                    egui::TextEdit::singleline(&mut self.password_input)
                        .password(true)
                        .desired_width(136.0)
                        .hint_text("请输入密码") // 添加提示文本
                );
                
                // 6. 绘制按钮
                // OK 按钮 (320-322)
                if self.draw_image_button(ui, ctx, "title", 320, 321, 322, egui::pos2(227.0, 81.0)) {
                    self.on_login_clicked();
                }
                
                // 新建账号按钮 (323-325)
                if self.draw_image_button(ui, ctx, "title", 323, 324, 325, egui::pos2(60.0, 163.0)) {
                    self.show_new_account = true;
                    self.show_login_dialog = false;
                }
                
                // 修改密码按钮 (326-328)
                if self.draw_image_button(ui, ctx, "title", 326, 327, 328, egui::pos2(166.0, 163.0)) {
                    println!("⚠ ChangePasswordDialog not implemented");
                }
                
                // 查看密钥按钮 (332-334)
                if self.draw_image_button(ui, ctx, "title", 332, 333, 334, egui::pos2(60.0, 189.0)) {
                    println!("⚠ InputKeyDialog not implemented");
                }
                
                // 关闭按钮 (329-331)
                if self.draw_image_button(ui, ctx, "title", 329, 330, 331, egui::pos2(166.0, 189.0)) {
                    std::process::exit(0);
                }
            });
    }
    
    /// 绘制图像按钮（三态：normal/hover/pressed）
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
            // 获取按钮尺寸
            if let Some(handle) = self.get_or_create_egui_texture(ctx, lib, lib_name, normal_idx) {
                let size = egui::vec2(handle.size()[0] as f32, handle.size()[1] as f32);
                let rect = egui::Rect::from_min_size(pos, size);
                let response = ui.allocate_rect(rect, egui::Sense::click());
                
                // 根据状态选择纹理
                let texture_idx = if response.is_pointer_button_down_on() {
                    pressed_idx
                } else if response.hovered() {
                    hover_idx
                } else {
                    normal_idx
                };
                
                // 绘制按钮图像
                if let Some(btn_handle) = self.get_or_create_egui_texture(ctx, lib, lib_name, texture_idx) {
                    ui.put(
                        rect,
                        egui::Image::new(&btn_handle).fit_to_exact_size(size)
                    );
                }
                
                return response.clicked();
            }
        }
        
        false
    }
    
    /// 绘制新建账号对话框
    fn draw_new_account_dialog(&mut self, ctx: &egui::Context) {
        let screen_w = screen_width();
        let screen_h = screen_height();
        let dialog_w = 360.0;
        let dialog_h = 254.0;
        let dialog_x = (screen_w - dialog_w) / 2.0;
        let dialog_y = (screen_h - dialog_h) / 2.0;
        
        egui::Window::new("new_account_dialog")
            .title_bar(false)
            .resizable(false)
            .movable(false)
            .frame(egui::Frame::none())
            .fixed_pos([dialog_x, dialog_y])
            .fixed_size([dialog_w, dialog_h])
            .show(ctx, |ui| {
                // 1. 绘制背景 (Prguse 63)
                if let Some(ref mut lib) = self.prguse_lib {
                    if let Some(handle) = self.get_or_create_egui_texture(ctx, lib, "prguse", 63) {
                        ui.put(
                            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(dialog_w, dialog_h)),
                            egui::Image::new(&handle).fit_to_exact_size(egui::vec2(dialog_w, dialog_h))
                        );
                    }
                }
                
                // 2. 绘制输入框
                let input_w = 140.0;
                let input_h = 18.0;
                
                ui.put(
                    egui::Rect::from_min_size(egui::pos2(86.0, 42.0), egui::vec2(input_w, input_h)),
                    egui::TextEdit::singleline(&mut self.new_account_id).desired_width(input_w)
                );
                ui.put(
                    egui::Rect::from_min_size(egui::pos2(86.0, 65.0), egui::vec2(input_w, input_h)),
                    egui::TextEdit::singleline(&mut self.new_password1).password(true).desired_width(input_w)
                );
                ui.put(
                    egui::Rect::from_min_size(egui::pos2(86.0, 88.0), egui::vec2(input_w, input_h)),
                    egui::TextEdit::singleline(&mut self.new_password2).password(true).desired_width(input_w)
                );
                ui.put(
                    egui::Rect::from_min_size(egui::pos2(86.0, 111.0), egui::vec2(input_w, input_h)),
                    egui::TextEdit::singleline(&mut self.new_birthdate).desired_width(input_w)
                );
                ui.put(
                    egui::Rect::from_min_size(egui::pos2(86.0, 134.0), egui::vec2(input_w, input_h)),
                    egui::TextEdit::singleline(&mut self.new_question).desired_width(input_w)
                );
                ui.put(
                    egui::Rect::from_min_size(egui::pos2(86.0, 157.0), egui::vec2(input_w, input_h)),
                    egui::TextEdit::singleline(&mut self.new_answer).desired_width(input_w)
                );
                ui.put(
                    egui::Rect::from_min_size(egui::pos2(86.0, 180.0), egui::vec2(input_w, input_h)),
                    egui::TextEdit::singleline(&mut self.new_email).desired_width(input_w)
                );
                ui.put(
                    egui::Rect::from_min_size(egui::pos2(86.0, 203.0), egui::vec2(input_w, input_h)),
                    egui::TextEdit::singleline(&mut self.new_username).desired_width(input_w)
                );
                
                // 3. 绘制按钮
                // OK 按钮 (200-202)
                if self.draw_image_button(ui, ctx, "title", 200, 201, 202, egui::pos2(60.0, 225.0)) {
                    self.on_create_account();
                }
                
                // Cancel 按钮 (203-205)
                if self.draw_image_button(ui, ctx, "title", 203, 204, 205, egui::pos2(166.0, 225.0)) {
                    self.close_new_account_dialog();
                }
            });
    }
    
    /// 关闭新建账号对话框
    fn close_new_account_dialog(&mut self) {
        self.show_new_account = false;
        self.show_login_dialog = true;
        self.new_account_id.clear();
        self.new_password1.clear();
        self.new_password2.clear();
        self.new_email.clear();
        self.new_username.clear();
        self.new_birthdate.clear();
        self.new_question.clear();
        self.new_answer.clear();
    }
    
    /// 创建账号
    fn on_create_account(&mut self) {
        if self.new_account_id.is_empty() {
            println!("⚠ 账号不能为空!");
            return;
        }
        if self.new_password1.is_empty() {
            println!("⚠ 密码不能为空!");
            return;
        }
        if self.new_password1 != self.new_password2 {
            println!("⚠ 两次密码输入不一致!");
            return;
        }
        
        println!("✅ 创建账号: {}", self.new_account_id);
        println!("   用户名: {}", self.new_username);
        println!("   邮箱: {}", self.new_email);
        
        self.close_new_account_dialog();
    }
    
    /// 登录按钮点击
    fn on_login_clicked(&mut self) {
        if self.account_input.is_empty() || self.password_input.is_empty() {
            println!("⚠ Account or password is empty!");
            return;
        }
        
        println!("🔐 Login: account={}", self.account_input);
        
        // 开始播放登录成功动画
        self.animation_playing = true;
        self.background_frame = 0;
        self.show_login_dialog = false;
    }
    
    /// 处理输入
    fn handle_input(&mut self) -> GameResult {
        if is_key_pressed(KeyCode::Escape) {
            std::process::exit(0);
        }
        Ok(())
    }
}

impl SceneHandler for LoginScene {
    fn name(&self) -> &str {
        "登录界面"
    }
    
    fn on_enter(&mut self) -> GameResult {
        self.account_input.clear();
        self.password_input.clear();
        println!("🎬 进入登录界面");
        Ok(())
    }
    
    fn on_exit(&mut self) -> GameResult {
        println!("🎬 离开登录界面");
        Ok(())
    }
    
    fn update(&mut self, dt: f32) -> GameResult<SceneTransition> {
        // 触发资源加载
        if !self.resources_loaded {
            self.load_resources()?;
        }
        
        // 更新背景动画
        if self.animation_playing {
            self.frame_timer += dt;
            if self.frame_timer >= self.frame_delay {
                self.frame_timer = 0.0;
                self.background_frame += 1;
                
                if self.background_frame >= 19 {
                    println!("✓ Login animation finished, switching to character select...");
                    return Ok(SceneTransition::CharacterSelect);
                }
            }
        }
        
        self.handle_input()?;
        
        Ok(SceneTransition::None)
    }
    
    fn render(&mut self) -> GameResult {
        clear_background(BLACK);
        
        // 绘制背景动画
        if let Some(ref mut lib) = self.chrsel_lib {
            let frame_index = if self.animation_playing {
                self.background_frame
            } else {
                0
            };
            
            if let Ok(info) = lib.get_or_create_texture(frame_index) {
                if let Some(ref texture) = info.image {
                    draw_texture(texture, 0.0, 0.0, WHITE);
                }
            }
        }
        
        // 使用 egui 绘制所有UI
        egui_macroquad::ui(|ctx| {
            if self.show_login_dialog {
                self.draw_login_ui(ctx);
            }
            
            if self.show_new_account {
                self.draw_new_account_dialog(ctx);
            }
        });
        
        // 绘制 egui
        egui_macroquad::draw();
        
        Ok(())
    }
}
