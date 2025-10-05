// Main application structure using eframe/egui
// This serves as the bridge between the game logic and the UI rendering

use eframe::egui;
use std::sync::Arc;
use parking_lot::RwLock;

use crate::scenes::{Scene, SceneType};
use crate::scenes::login_scene::LoginScene;
use crate::scenes::select_scene::{SelectScene, SelectCharacter};
use crate::scenes::game_scene::GameScene;
use crate::network::game_client::GameEvent;
use crate::network::game_client::GameClient;
use crate::network::network_command::NetworkCommand;
use crate::settings::ClientSettings;
use crate::graphics::TextureManager;
use crate::graphics::CharacterRenderer;
use crate::sounds::SoundManager;

/// Main application state
pub struct MirClientApp {
    /// Current active scene
    current_scene: SceneType,
    
    /// Scene instances
    login_scene: LoginScene,
    select_scene: Option<SelectScene>,
    game_scene: Option<GameScene>,
    
    /// Application settings
    settings: Arc<RwLock<ClientSettings>>,
    
    /// Game client (shared with network task)
    game_client: Arc<RwLock<GameClient>>,
    
    /// Game event receiver (from network layer)
    event_rx: Option<tokio::sync::mpsc::UnboundedReceiver<GameEvent>>,
    
    /// Network command sender (to network layer)
    command_tx: tokio::sync::mpsc::UnboundedSender<crate::network::NetworkCommand>,
    
    /// Texture manager for loading game resources
    texture_manager: TextureManager,
    
    /// Character renderer for loading character sprites
    character_renderer: CharacterRenderer,
    
    /// Sound manager for playing audio
    sound_manager: Option<SoundManager>,
    
    /// Delta time for frame updates
    last_frame_time: std::time::Instant,
    
    /// FPS counter
    fps: f32,
    frame_count: u32,
    fps_timer: std::time::Instant,
}

impl MirClientApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        settings: ClientSettings,
        game_client: Arc<RwLock<GameClient>>,
        event_rx: tokio::sync::mpsc::UnboundedReceiver<GameEvent>,
        command_tx: tokio::sync::mpsc::UnboundedSender<crate::network::NetworkCommand>,
    ) -> Self {
        // Configure egui style for game-like UI
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        
        // 创建纹理管理器
        let mut texture_manager = TextureManager::new();
        
        // 尝试加载基础UI库
        // TODO: 从settings中获取数据路径
        let _ = texture_manager.load_library("Prguse", std::path::Path::new("Data/Prguse.lib"));
        
        // 创建角色渲染器并加载 ChrSel.lib
        let mut character_renderer = CharacterRenderer::new();
        if let Err(e) = character_renderer.load_chrsel_library("Data/ChrSel.lib") {
            tracing::warn!("Failed to load ChrSel.lib: {}", e);
        } else {
            tracing::info!("Successfully loaded ChrSel.lib");
        }
        
        // 创建音频管理器
        let mut sound_manager = SoundManager::new().ok();
        
        // 加载音效文件
        if let Some(ref mut sm) = sound_manager {
            let _ = sm.load_sounds_from_dir(std::path::Path::new("Data/Sounds"), crate::sounds::SoundType::Effect);
            let _ = sm.load_sounds_from_dir(std::path::Path::new("Data/Music"), crate::sounds::SoundType::Music);
        }
        
        Self {
            current_scene: SceneType::Login,
            login_scene: LoginScene::new(),
            select_scene: None,
            game_scene: None,
            settings: Arc::new(RwLock::new(settings)),
            game_client,
            event_rx: Some(event_rx),
            command_tx,
            texture_manager,
            character_renderer,
            sound_manager,
            last_frame_time: std::time::Instant::now(),
            fps: 0.0,
            frame_count: 0,
            fps_timer: std::time::Instant::now(),
        }
    }
    
    /// Get reference to game client
    pub fn game_client(&self) -> Arc<RwLock<GameClient>> {
        self.game_client.clone()
    }
    
    /// Send login packet to server
    pub fn send_login(&mut self, username: &str, password: &str) {
        use crate::network::NetworkCommand;
        
        tracing::info!("Sending login command for user: {}", username);
        
        let command = NetworkCommand::Login {
            username: username.to_string(),
            password: password.to_string(),
        };
        
        // Send command to network thread
        if let Err(e) = self.command_tx.send(command) {
            tracing::error!("Failed to send login command: {}", e);
            self.login_scene.record_status("Failed to send login request");
            self.login_scene.connecting = false;
            self.login_scene.login_enabled = true;
        } else {
            self.login_scene.connecting = true;
            self.login_scene.login_enabled = false;
            self.login_scene.record_status("Logging in...");
        }
    }
    
    /// Switch to a different scene
    fn switch_scene(&mut self, scene_type: SceneType) {
        // Hide current scene
        match self.current_scene {
            SceneType::Login => self.login_scene.hide(),
            SceneType::Select => {
                if let Some(scene) = &mut self.select_scene {
                    scene.hide();
                }
            }
            SceneType::Game => {
                if let Some(scene) = &mut self.game_scene {
                    scene.hide();
                }
            }
        }
        
        // Play scene music
        if let Some(ref mut sound_manager) = self.sound_manager {
            let music_name = match scene_type {
                SceneType::Login => "LoginMusic",
                SceneType::Select => "SelectMusic",
                SceneType::Game => "InTown1",  // Town music
            };
            
            // Try to play the scene music
            if let Err(e) = sound_manager.play_music(music_name) {
                tracing::debug!("Failed to play music '{}': {}", music_name, e);
            } else {
                tracing::info!("♪ Playing music: {}", music_name);
            }
        }
        
        // Show new scene
        self.current_scene = scene_type;
        match scene_type {
            SceneType::Login => self.login_scene.show(),
            SceneType::Select => {
                if self.select_scene.is_none() {
                    self.select_scene = Some(SelectScene::new());
                }
                if let Some(scene) = &mut self.select_scene {
                    scene.show();
                }
            }
            SceneType::Game => {
                if self.game_scene.is_none() {
                    self.game_scene = Some(GameScene::new());
                }
                if let Some(scene) = &mut self.game_scene {
                    scene.show();
                }
            }
        }
    }
    
    /// Process game events from network layer
    fn process_events(&mut self) {
        let mut scene_to_switch: Option<SceneType> = None;
        
        if let Some(rx) = &mut self.event_rx {
            // Process up to 100 events per frame to avoid blocking
            for _ in 0..100 {
                match rx.try_recv() {
                    Ok(event) => {
                        // Check for scene switching events before forwarding
                        match &event {
                            GameEvent::LoginSuccess { .. } => {
                                scene_to_switch = Some(SceneType::Select);
                            }
                            GameEvent::LoginResponse { result } => {
                                // Login failed, restore UI state
                                self.login_scene.connecting = false;
                                self.login_scene.login_enabled = true;
                                tracing::warn!("Login failed with result code: {}", result);
                            }
                            GameEvent::LoginBanned { reason, expiry_date } => {
                                // Login banned, restore UI state
                                self.login_scene.connecting = false;
                                self.login_scene.login_enabled = true;
                                tracing::error!("Login banned: {} (expires: {})", reason, expiry_date);
                            }
                            GameEvent::Disconnected { .. } => {
                                scene_to_switch = Some(SceneType::Login);
                            }
                            GameEvent::NewCharacterResponse { result } => {
                                // Handle character creation response
                                if let Some(scene) = &mut self.select_scene {
                                    scene.new_character_dialog.creating = false;
                                    match result {
                                        0 => {
                                            // Success - will receive NewCharacterSuccess next
                                            tracing::info!("Character creation request accepted");
                                        }
                                        1 => {
                                            scene.new_character_dialog.error_message = 
                                                Some("角色名称已被使用".to_string());
                                        }
                                        2 => {
                                            scene.new_character_dialog.error_message = 
                                                Some("角色名称不合法".to_string());
                                        }
                                        3 => {
                                            scene.new_character_dialog.error_message = 
                                                Some("角色槽位已满".to_string());
                                        }
                                        _ => {
                                            scene.new_character_dialog.error_message = 
                                                Some(format!("创建失败 (错误码: {})", result));
                                        }
                                    }
                                }
                            }
                            GameEvent::NewCharacterSuccess { character } => {
                                // Character created successfully
                                if let Some(scene) = &mut self.select_scene {
                                    tracing::info!("✅ Character created: {}", character.name);
                                    
                                    // Add character to the list
                                    let new_char = SelectCharacter {
                                        index: character.index as u32,
                                        name: character.name.clone(),
                                        level: character.level,
                                        class: mir2_shared::enums::MirClass::try_from(character.class)
                                            .unwrap_or(mir2_shared::enums::MirClass::Warrior),
                                        gender: mir2_shared::enums::MirGender::try_from(character.gender)
                                            .unwrap_or(mir2_shared::enums::MirGender::Male),
                                        exists: true,
                                    };
                                    
                                    // Find empty slot and add character
                                    for slot in scene.characters.iter_mut() {
                                        if slot.is_none() {
                                            *slot = Some(new_char);
                                            break;
                                        }
                                    }
                                    
                                    // Close dialog
                                    scene.new_character_dialog.hide();
                                }
                            }
                            GameEvent::DeleteCharacterResponse { result } => {
                                // Handle character deletion response
                                if let Some(scene) = &mut self.select_scene {
                                    scene.character_deletion_dialog.deleting = false;
                                    match result {
                                        0 => {
                                            scene.character_deletion_dialog.error_message = 
                                                Some("删除角色功能当前已禁用".to_string());
                                        }
                                        1 => {
                                            scene.character_deletion_dialog.error_message = 
                                                Some("角色不存在\n请联系GM寻求帮助".to_string());
                                        }
                                        _ => {
                                            scene.character_deletion_dialog.error_message = 
                                                Some(format!("删除失败 (错误码: {})", result));
                                        }
                                    }
                                }
                            }
                            GameEvent::DeleteCharacterSuccess { character_index } => {
                                // Character deleted successfully
                                if let Some(scene) = &mut self.select_scene {
                                    tracing::info!("✅ Character deleted: index {}", character_index);
                                    
                                    // Remove character from the list
                                    for slot in scene.characters.iter_mut() {
                                        if let Some(character) = slot {
                                            if character.index as i32 == *character_index {
                                                *slot = None;
                                                break;
                                            }
                                        }
                                    }
                                    
                                    // Close dialog
                                    scene.character_deletion_dialog.hide();
                                }
                            }
                            _ => {}
                        }
                        
                        // Forward event to current scene
                        match self.current_scene {
                            SceneType::Login => self.login_scene.process_event(&event),
                            SceneType::Select => {
                                if let Some(scene) = &mut self.select_scene {
                                    scene.process_event(&event);
                                }
                            }
                            SceneType::Game => {
                                if let Some(scene) = &mut self.game_scene {
                                    scene.process_event(&event);
                                }
                            }
                        }
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                        tracing::warn!("Event channel disconnected");
                        break;
                    }
                }
            }
        }
        
        // Switch scene after processing all events
        if let Some(new_scene) = scene_to_switch {
            self.switch_scene(new_scene);
        }
    }
    
    /// Update FPS counter
    fn update_fps(&mut self) {
        self.frame_count += 1;
        let elapsed = self.fps_timer.elapsed().as_secs_f32();
        if elapsed >= 1.0 {
            self.fps = self.frame_count as f32 / elapsed;
            self.frame_count = 0;
            self.fps_timer = std::time::Instant::now();
        }
    }
}

impl eframe::App for MirClientApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Calculate delta time
        let now = std::time::Instant::now();
        let delta_time = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;
        
        // Update FPS
        self.update_fps();
        
        // Process network events
        self.process_events();
        
        // Update current scene
        match self.current_scene {
            SceneType::Login => self.login_scene.update(delta_time),
            SceneType::Select => {
                if let Some(scene) = &mut self.select_scene {
                    scene.update(delta_time);
                }
            }
            SceneType::Game => {
                if let Some(scene) = &mut self.game_scene {
                    scene.update(delta_time);
                }
            }
        }
        
        // Render UI using egui
        egui::CentralPanel::default().show(ctx, |ui| {
            // Draw current scene
            match self.current_scene {
                SceneType::Login => {
                    self.render_login_scene(ui);
                }
                SceneType::Select => {
                    self.render_select_scene(ui);
                }
                SceneType::Game => {
                    self.render_game_scene(ui);
                }
            }
            
            // Debug info (top-right corner)
            ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
                ui.label(format!("FPS: {:.1}", self.fps));
                ui.label(format!("Scene: {:?}", self.current_scene));
            });
        });
        
        // Request continuous repaint for game loop
        ctx.request_repaint();
    }
}

// Scene rendering methods
impl MirClientApp {
    fn render_login_scene(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            
            // Title
            ui.heading("Legend of Mir 2 - Rust Edition");
            ui.add_space(20.0);
            
            // Show connection status
            if self.login_scene.connecting {
                ui.label("Connecting to server...");
                ui.add_space(10.0);
            } else if self.login_scene.version_checked && self.login_scene.version_valid {
                ui.colored_label(egui::Color32::GREEN, "✓ Connected");
                ui.add_space(10.0);
            }
            
            // Login form
            ui.label("Username:");
            ui.text_edit_singleline(&mut self.login_scene.username);
            
            ui.add_space(10.0);
            ui.label("Password:");
            let password_edit = egui::TextEdit::singleline(&mut self.login_scene.password)
                .password(true);
            ui.add(password_edit);
            
            ui.add_space(20.0);
            
            // Login button
            let login_enabled = self.login_scene.login_enabled 
                && !self.login_scene.username.is_empty() 
                && !self.login_scene.password.is_empty();
            
            if ui.add_enabled(login_enabled, egui::Button::new("Login")).clicked() {
                tracing::info!("Login button clicked: user={}", self.login_scene.username);
                
                // Get credentials
                let username = self.login_scene.username.clone();
                let password = self.login_scene.password.clone();
                
                // Update scene state
                self.login_scene.submit_login();
                
                // Send login packet
                self.send_login(&username, &password);
            }
            
            if ui.button("Create Account").clicked() {
                tracing::info!("Create account clicked");
                self.login_scene.open_new_account_dialog();
            }
            
            if ui.button("Exit").clicked() {
                std::process::exit(0);
            }
            
            // Show status messages
            ui.add_space(20.0);
            
            // Show ban information if present
            if let Some(ban_info) = &self.login_scene.login_ban_info {
                ui.colored_label(
                    egui::Color32::RED, 
                    format!("⛔ Login Banned: {}", ban_info.reason)
                );
                ui.colored_label(
                    egui::Color32::from_rgb(255, 150, 150), 
                    format!("Expiry: {} (ticks)", ban_info.expiry_date)
                );
                ui.add_space(10.0);
            }
            
            // Show status messages with color coding
            if let Some(status) = &self.login_scene.last_status {
                let color = if self.login_scene.last_login_result.is_some() {
                    // Login failed - show in red
                    egui::Color32::from_rgb(255, 100, 100)
                } else if self.login_scene.ready_for_character_select {
                    // Login success - show in green
                    egui::Color32::GREEN
                } else {
                    // Normal status - show in yellow
                    egui::Color32::YELLOW
                };
                ui.colored_label(color, status);
            }
        });
    }
    
    fn render_select_scene(&mut self, ui: &mut egui::Ui) {
        // Apply custom background for select scene
        let frame = egui::Frame::none()
            .fill(egui::Color32::from_rgb(15, 20, 30))
            .inner_margin(20.0);
        
        frame.show(ui, |ui| {
            ui.vertical_centered(|ui| {
                // Title bar with decorative style
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() / 2.0 - 200.0);
                    ui.label(egui::RichText::new("━━━━━━━━")
                        .size(18.0)
                        .color(egui::Color32::from_rgb(100, 150, 200)));
                    ui.label(egui::RichText::new(" 🎮 Select Character ")
                        .size(28.0)
                        .strong()
                        .color(egui::Color32::from_rgb(255, 220, 150)));
                    ui.label(egui::RichText::new("━━━━━━━━")
                        .size(18.0)
                        .color(egui::Color32::from_rgb(100, 150, 200)));
                });
                
                ui.add_space(30.0);
                
                if let Some(scene) = &mut self.select_scene {
                    // Count actual characters
                    let char_count = scene.characters.iter().filter(|c| c.is_some()).count();
                    
                    if char_count == 0 {
                        // Empty state with decorative frame
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(25, 30, 40))
                            .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgb(60, 80, 100)))
                            .rounding(8.0)
                            .inner_margin(40.0)
                            .show(ui, |ui| {
                                ui.vertical_centered(|ui| {
                                    ui.label(egui::RichText::new("📭")
                                        .size(48.0));
                                    ui.add_space(10.0);
                                    ui.label(egui::RichText::new("No characters found")
                                        .size(18.0)
                                        .color(egui::Color32::from_rgb(150, 150, 150)));
                                    ui.add_space(15.0);
                                    if ui.add(egui::Button::new(
                                        egui::RichText::new("➕ Create Your First Character")
                                            .size(16.0)
                                    ).min_size(egui::vec2(250.0, 40.0))).clicked() {
                                        tracing::info!("Create first character clicked");
                                        scene.new_character_dialog.show();
                                    }
                                });
                            });
                    } else {
                        // Character count badge
                        ui.horizontal(|ui| {
                            ui.add_space(ui.available_width() / 2.0 - 100.0);
                            egui::Frame::none()
                                .fill(egui::Color32::from_rgb(40, 60, 80))
                                .rounding(12.0)
                                .inner_margin(egui::vec2(15.0, 8.0))
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new(format!("📋 {} character(s) available", char_count))
                                        .size(14.0)
                                        .color(egui::Color32::from_rgb(200, 220, 255)));
                                });
                        });
                        
                        // Character cards container
                        ui.add_space(20.0);
                        
                        egui::ScrollArea::vertical()
                            .max_height(400.0)
                            .show(ui, |ui| {
                                for (idx, character_slot) in scene.characters.iter().enumerate() {
                                    let is_selected = scene.selected_index == idx;
                                    
                                    // Character card with enhanced styling
                                    let card_fill = if is_selected {
                                        egui::Color32::from_rgb(40, 60, 90)
                                    } else {
                                        egui::Color32::from_rgb(25, 30, 40)
                                    };
                                    
                                    let card_stroke = if is_selected {
                                        egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 150, 255))
                                    } else {
                                        egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 70, 80))
                                    };
                                    
                                    let response = egui::Frame::none()
                                        .fill(card_fill)
                                        .stroke(card_stroke)
                                        .rounding(6.0)
                                        .inner_margin(15.0)
                                        .show(ui, |ui| {
                                            ui.set_min_width(500.0);
                                            
                                            ui.horizontal(|ui| {
                                                // Character preview image (if available)
                                                if let Some(character) = character_slot {
                                                    // Load character sprite if not cached
                                                    if !scene.character_preview_textures.contains_key(&idx) {
                                                        match self.character_renderer.load_character_color_image(
                                                            character.class,
                                                            character.gender,
                                                            0  // Frame 0 for preview
                                                        ) {
                                                            Ok((_, color_image)) => {
                                                                let texture = ui.ctx().load_texture(
                                                                    format!("char_preview_{}", idx),
                                                                    color_image,
                                                                    egui::TextureOptions::default(),
                                                                );
                                                                scene.character_preview_textures.insert(idx, texture);
                                                            }
                                                            Err(e) => {
                                                                tracing::warn!("Failed to load character sprite for slot {}: {}", idx, e);
                                                            }
                                                        }
                                                    }
                                                    
                                                    // Display character preview image
                                                    if let Some(texture) = scene.character_preview_textures.get(&idx) {
                                                        ui.image(texture);
                                                        ui.add_space(10.0);
                                                    }
                                                }
                                                
                                                // Slot number badge
                                                egui::Frame::none()
                                                    .fill(egui::Color32::from_rgb(60, 80, 100))
                                                    .rounding(4.0)
                                                    .inner_margin(egui::vec2(8.0, 4.0))
                                                    .show(ui, |ui| {
                                                        ui.label(egui::RichText::new(format!("#{}", idx + 1))
                                                            .size(12.0)
                                                            .color(egui::Color32::from_rgb(180, 200, 220)));
                                                    });
                                                
                                                ui.add_space(15.0);
                                                
                                                if let Some(character) = character_slot {
                                                    // Character name (highlighted if selected)
                                                    let name_color = if is_selected {
                                                        egui::Color32::from_rgb(150, 200, 255)
                                                    } else {
                                                        egui::Color32::from_rgb(200, 220, 255)
                                                    };
                                                    ui.label(egui::RichText::new(format!("👤 {}", character.name))
                                                        .size(16.0)
                                                        .strong()
                                                        .color(name_color));
                                                    
                                                    ui.add_space(10.0);
                                                    
                                                    // Level badge
                                                    egui::Frame::none()
                                                        .fill(egui::Color32::from_rgb(80, 60, 40))
                                                        .rounding(4.0)
                                                        .inner_margin(egui::vec2(6.0, 2.0))
                                                        .show(ui, |ui| {
                                                            ui.label(egui::RichText::new(format!("⬆️ Lv.{}", character.level))
                                                                .size(14.0)
                                                                .color(egui::Color32::from_rgb(255, 220, 150)));
                                                        });
                                                    
                                                    ui.add_space(10.0);
                                                    
                                                    // Class badge with emoji and color
                                                    let (class_icon, class_color) = match character.class {
                                                        mir2_shared::enums::MirClass::Warrior => ("⚔️ Warrior", egui::Color32::from_rgb(255, 150, 100)),
                                                        mir2_shared::enums::MirClass::Wizard => ("🔮 Wizard", egui::Color32::from_rgb(150, 150, 255)),
                                                        mir2_shared::enums::MirClass::Taoist => ("☯️ Taoist", egui::Color32::from_rgb(100, 255, 150)),
                                                        mir2_shared::enums::MirClass::Assassin => ("🗡️ Assassin", egui::Color32::from_rgb(200, 100, 200)),
                                                        mir2_shared::enums::MirClass::Archer => ("🏹 Archer", egui::Color32::from_rgb(150, 255, 150)),
                                                    };
                                                    
                                                    egui::Frame::none()
                                                        .fill(egui::Color32::from_rgba_premultiplied(
                                                            class_color.r() / 4, 
                                                            class_color.g() / 4,
                                                            class_color.b() / 4,
                                                            100
                                                        ))
                                                        .rounding(4.0)
                                                        .inner_margin(egui::vec2(8.0, 2.0))
                                                        .show(ui, |ui| {
                                                            ui.label(egui::RichText::new(class_icon)
                                                                .size(14.0)
                                                                .color(class_color));
                                                        });
                                                    
                                                    // Selected indicator
                                                    if is_selected {
                                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                            ui.label(egui::RichText::new("✅ Selected")
                                                                .size(14.0)
                                                                .color(egui::Color32::from_rgb(100, 255, 150)));
                                                        });
                                                    }
                                                } else {
                                                    // Empty slot
                                                    ui.label(egui::RichText::new("📭 Empty Slot")
                                                        .size(16.0)
                                                        .color(egui::Color32::from_rgb(120, 120, 120)));
                                                    ui.label(egui::RichText::new("(Click to create a new character)")
                                                        .size(12.0)
                                                        .color(egui::Color32::from_rgb(100, 100, 100)));
                                                }
                                            });
                                        });
                                    
                                    // Click to select character slot
                                    if response.response.interact(egui::Sense::click()).clicked() {
                                        scene.selected_index = idx;
                                        tracing::info!("Selected character slot {}", idx);
                                    }
                                    
                                    ui.add_space(8.0);
                                }
                            });
                        
                        // Action buttons section with separator
                        ui.add_space(20.0);
                        ui.separator();
                        ui.add_space(15.0);
                        
                        ui.horizontal(|ui| {
                            ui.add_space(ui.available_width() / 2.0 - 250.0);
                            
                            // Start Game button (only enabled if character exists in selected slot)
                            let can_start = scene.characters.get(scene.selected_index)
                                .and_then(|c| c.as_ref())
                                .is_some();
                            
                            ui.add_enabled_ui(can_start, |ui| {
                                if ui.add(egui::Button::new(
                                    egui::RichText::new("🚀 Start Game")
                                        .size(16.0)
                                ).min_size(egui::vec2(140.0, 35.0))).clicked() {
                                    if let Some(Some(character)) = scene.characters.get(scene.selected_index) {
                                        tracing::info!("Starting game with character: {} (index={})", 
                                            character.name, character.index);
                                        
                                        // Send StartGame command with character index
                                        if let Err(e) = self.command_tx.send(NetworkCommand::StartGame { 
                                            character_index: character.index as i32 
                                        }) {
                                            tracing::error!("Failed to send StartGame command: {}", e);
                                        }
                                    }
                                }
                            });
                            
                            ui.add_space(10.0);
                            
                            // Create Character button (only enabled if selected slot is empty)
                            let can_create = scene.characters.get(scene.selected_index)
                                .map(|c| c.is_none())
                                .unwrap_or(false);
                            
                            ui.add_enabled_ui(can_create, |ui| {
                                if ui.add(egui::Button::new(
                                    egui::RichText::new("➕ Create Character")
                                        .size(16.0)
                                ).min_size(egui::vec2(160.0, 35.0))).clicked() {
                                    tracing::info!("Create character in slot {}", scene.selected_index);
                                    scene.new_character_dialog.show();
                                }
                            });
                            
                            ui.add_space(10.0);
                            
                            // Delete Character button (only enabled if character exists)
                            let can_delete = can_start;
                            ui.add_enabled_ui(can_delete, |ui| {
                                if ui.add(egui::Button::new(
                                    egui::RichText::new("🗑️ Delete Character")
                                        .size(16.0)
                                ).min_size(egui::vec2(160.0, 35.0))).clicked() {
                                    if let Some(Some(character)) = scene.characters.get(scene.selected_index) {
                                        tracing::info!("Delete character: {}", character.name);
                                        scene.character_deletion_dialog.show(character.clone());
                                    }
                                }
                            });
                        });
                    }
                }
                
                // Back button at bottom
                ui.add_space(30.0);
                ui.separator();
                ui.add_space(15.0);
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() / 2.0 - 75.0);
                    if ui.add(egui::Button::new(
                        egui::RichText::new("⬅️ Back to Login")
                            .size(16.0)
                    ).min_size(egui::vec2(150.0, 35.0))).clicked() {
                        self.switch_scene(SceneType::Login);
                    }
                });
            });
        });
        
        // Render dialogs outside vertical_centered (to avoid borrow conflict)
        let command_tx = self.command_tx.clone();
        if let Some(scene) = &mut self.select_scene {
            if scene.new_character_dialog.visible {
                Self::render_new_character_dialog_static(ui, scene, &command_tx);
            }
            if scene.character_deletion_dialog.visible {
                Self::render_character_deletion_dialog_static(ui, scene, &command_tx);
            }
        }
    }
    
    /// Render character creation dialog (modal) - static version to avoid borrow issues
    fn render_new_character_dialog_static(
        ui: &mut egui::Ui, 
        scene: &mut crate::scenes::select_scene::SelectScene,
        command_tx: &tokio::sync::mpsc::UnboundedSender<NetworkCommand>,
    ) {
        use mir2_shared::enums::{MirClass, MirGender};
        
        egui::Window::new("🎨 Create New Character")
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false)
            .resizable(false)
            .fixed_size([600.0, 500.0])
            .show(ui.ctx(), |ui| {
                let dialog = &mut scene.new_character_dialog;
                
                ui.vertical(|ui| {
                    ui.add_space(10.0);
                    
                    // Character name input
                    ui.heading("Character Name");
                    ui.add_space(5.0);
                    let name_response = ui.text_edit_singleline(&mut dialog.name);
                    if name_response.changed() {
                        dialog.error_message = None; // Clear error on edit
                    }
                    ui.add_space(10.0);
                    
                    // Class selection
                    ui.heading("Select Class");
                    ui.add_space(5.0);
                    ui.horizontal(|ui| {
                        if ui.selectable_label(dialog.selected_class == MirClass::Warrior, "⚔️ Warrior").clicked() {
                            dialog.selected_class = MirClass::Warrior;
                        }
                        if ui.selectable_label(dialog.selected_class == MirClass::Wizard, "🔮 Wizard").clicked() {
                            dialog.selected_class = MirClass::Wizard;
                        }
                        if ui.selectable_label(dialog.selected_class == MirClass::Taoist, "☯️ Taoist").clicked() {
                            dialog.selected_class = MirClass::Taoist;
                        }
                        if ui.selectable_label(dialog.selected_class == MirClass::Assassin, "🗡️ Assassin").clicked() {
                            dialog.selected_class = MirClass::Assassin;
                        }
                        if ui.selectable_label(dialog.selected_class == MirClass::Archer, "🏹 Archer").clicked() {
                            dialog.selected_class = MirClass::Archer;
                        }
                    });
                    ui.add_space(10.0);
                    
                    // Gender selection
                    ui.heading("Select Gender");
                    ui.add_space(5.0);
                    ui.horizontal(|ui| {
                        if ui.selectable_label(dialog.selected_gender == MirGender::Male, "♂️ Male").clicked() {
                            dialog.selected_gender = MirGender::Male;
                        }
                        if ui.selectable_label(dialog.selected_gender == MirGender::Female, "♀️ Female").clicked() {
                            dialog.selected_gender = MirGender::Female;
                        }
                    });
                    ui.add_space(15.0);
                    
                    // Class description
                    ui.group(|ui| {
                        ui.set_min_width(580.0);
                        ui.set_max_width(580.0);
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(format!("{} {} Description", 
                                dialog.get_class_icon(), 
                                format!("{:?}", dialog.selected_class)
                            )).strong());
                            ui.add_space(5.0);
                            ui.label(dialog.get_class_description());
                        });
                    });
                    ui.add_space(15.0);
                    
                    // Error message
                    if let Some(ref error) = dialog.error_message {
                        ui.colored_label(egui::Color32::from_rgb(255, 100, 100), 
                            format!("❌ {}", error));
                        ui.add_space(10.0);
                    }
                    
                    // Creating状态提示
                    if dialog.creating {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Creating character...");
                        });
                        ui.add_space(10.0);
                    }
                    
                    // Buttons
                    ui.horizontal(|ui| {
                        ui.add_enabled_ui(!dialog.creating, |ui| {
                            if ui.button("✅ Create").clicked() {
                                // Validate name
                                match dialog.validate_name() {
                                    Ok(_) => {
                                        tracing::info!("Creating character: name={}, class={:?}, gender={:?}", 
                                            dialog.name, dialog.selected_class, dialog.selected_gender);
                                        
                                        dialog.creating = true;
                                        
                                        // Send NewCharacter command
                                        if let Err(e) = command_tx.send(NetworkCommand::NewCharacter {
                                            name: dialog.name.clone(),
                                            class: dialog.selected_class as u8,
                                            gender: dialog.selected_gender as u8,
                                        }) {
                                            tracing::error!("Failed to send NewCharacter command: {}", e);
                                            dialog.error_message = Some("Failed to send request".to_string());
                                            dialog.creating = false;
                                        }
                                    }
                                    Err(e) => {
                                        dialog.error_message = Some(e);
                                    }
                                }
                            }
                        });
                        
                        ui.add_space(10.0);
                        
                        ui.add_enabled_ui(!dialog.creating, |ui| {
                            if ui.button("❌ Cancel").clicked() {
                                dialog.hide();
                            }
                        });
                    });
                });
            });
    }
    
    /// Render character deletion dialog (modal) - static version to avoid borrow issues
    fn render_character_deletion_dialog_static(
        ui: &mut egui::Ui,
        scene: &mut crate::scenes::select_scene::SelectScene,
        command_tx: &tokio::sync::mpsc::UnboundedSender<NetworkCommand>,
    ) {
        let dialog = &mut scene.character_deletion_dialog;
        
        if !dialog.show_name_input {
            // Step 1: Confirmation dialog
            egui::Window::new("⚠️ Delete Character")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .collapsible(false)
                .resizable(false)
                .fixed_size([400.0, 200.0])
                .show(ui.ctx(), |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        
                        if let Some(ref character) = dialog.character_to_delete {
                            ui.label(egui::RichText::new("Are you sure you want to delete this character?")
                                .size(16.0)
                                .strong());
                            ui.add_space(10.0);
                            
                            ui.label(egui::RichText::new(format!("👤 {}", character.name))
                                .size(18.0)
                                .color(egui::Color32::from_rgb(255, 200, 100)));
                            ui.label(format!("⬆️ Level {} {:?}", character.level, character.class));
                            
                            ui.add_space(20.0);
                            
                            ui.colored_label(
                                egui::Color32::from_rgb(255, 100, 100),
                                "⚠️ This action cannot be undone!"
                            );
                        }
                        
                        ui.add_space(20.0);
                        
                        ui.horizontal(|ui| {
                            if ui.button("✅ Yes, Delete").clicked() {
                                dialog.show_name_input_stage();
                            }
                            
                            ui.add_space(10.0);
                            
                            if ui.button("❌ No, Cancel").clicked() {
                                dialog.hide();
                            }
                        });
                    });
                });
        } else {
            // Step 2: Name confirmation dialog
            egui::Window::new("🔐 Confirm Deletion")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .collapsible(false)
                .resizable(false)
                .fixed_size([450.0, 280.0])
                .show(ui.ctx(), |ui| {
                    ui.vertical(|ui| {
                        ui.add_space(10.0);
                        
                        ui.label(egui::RichText::new("Please enter the character name to confirm:")
                            .size(14.0));
                        ui.add_space(5.0);
                        
                        if let Some(ref character) = dialog.character_to_delete {
                            ui.label(egui::RichText::new(format!("Type: {}", character.name))
                                .size(16.0)
                                .strong()
                                .color(egui::Color32::from_rgb(255, 200, 100)));
                        }
                        
                        ui.add_space(10.0);
                        
                        let name_response = ui.text_edit_singleline(&mut dialog.input_name);
                        if name_response.changed() {
                            dialog.error_message = None; // Clear error on edit
                        }
                        
                        ui.add_space(10.0);
                        
                        // Error message
                        if let Some(ref error) = dialog.error_message {
                            ui.colored_label(egui::Color32::from_rgb(255, 100, 100), 
                                format!("❌ {}", error));
                            ui.add_space(10.0);
                        }
                        
                        // Deleting status
                        if dialog.deleting {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label("Deleting character...");
                            });
                            ui.add_space(10.0);
                        }
                        
                        // Buttons
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.add_enabled_ui(!dialog.deleting, |ui| {
                                if ui.button("🗑️ Delete").clicked() {
                                    // Validate name
                                    match dialog.validate_name() {
                                        Ok(_) => {
                                            if let Some(index) = dialog.get_character_index() {
                                                tracing::info!("Deleting character at index {}", index);
                                                dialog.deleting = true;
                                                
                                                // Send DeleteCharacter command
                                                if let Err(e) = command_tx.send(NetworkCommand::DeleteCharacter {
                                                    index,
                                                }) {
                                                    tracing::error!("Failed to send DeleteCharacter command: {}", e);
                                                    dialog.error_message = Some("Failed to send request".to_string());
                                                    dialog.deleting = false;
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            dialog.error_message = Some(e);
                                        }
                                    }
                                }
                            });
                            
                            ui.add_space(10.0);
                            
                            ui.add_enabled_ui(!dialog.deleting, |ui| {
                                if ui.button("❌ Cancel").clicked() {
                                    dialog.hide();
                                }
                            });
                        });
                    });
                });
        }
    }
    
    fn render_game_scene(&mut self, ui: &mut egui::Ui) {
        // Game scene rendering (full screen)
        ui.label("Game Scene - Coming Soon");
        ui.label("Press ESC to return to character select");
        
        // TODO: Render game world, UI overlays, dialogs, etc.
        
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.switch_scene(SceneType::Select);
        }
    }
}
