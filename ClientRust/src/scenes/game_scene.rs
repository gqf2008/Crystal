// GameScene - Main game scene (核心游戏场景)
// Mirrors Client/MirScenes/GameScene.cs (12,297 lines)
pub mod map_control;
// pub mod map_loader; // 已删除 - 使用 objects::MapReader 代替
pub mod tile_texture_manager;

use super::{Scene, SceneType};
use crate::objects::*;
use crate::network::game_client::GameEvent;
use mir2_shared::UserItem;
use mir2_shared::enums::{AttackMode, PetMode, LightSetting};
use std::collections::{HashMap, VecDeque};

/// Output message (chat/system messages)
#[derive(Debug, Clone)]
pub struct OutputMessage {
    pub text: String,
    pub color: (u8, u8, u8), // RGB
    pub timestamp: i64,
}

/// Main game scene
#[derive(Debug)]
pub struct GameScene {
    // Player and hero
    pub user: Option<UserObject>,
    pub hero: Option<HeroObject>,
    
    // Game objects
    pub objects: HashMap<u32, Box<dyn std::fmt::Debug>>, // All game objects by ID
    pub monsters: HashMap<u32, MonsterObject>,
    pub npcs: HashMap<u32, NPCObject>,
    pub items: HashMap<u32, ItemObject>,
    pub players: HashMap<u32, UserObject>,
    pub spells: Vec<SpellObject>,
    pub effects: Vec<Effect>,
    pub damages: Vec<Damage>,
    
    // Game state
    pub gold: u32,
    pub credit: u32,
    pub attack_mode: AttackMode,
    pub pet_mode: PetMode,
    pub lights: LightSetting,
    
    // Storage
    pub storage: Vec<Option<UserItem>>,         // 80 slots
    pub guild_storage: Vec<Option<UserItem>>,   // 112 slots
    pub refine_storage: Vec<Option<UserItem>>,  // 16 slots
    pub hero_storage: [Option<HeroObject>; 8],  // 8 hero slots
    
    // Item interaction
    pub hover_item: Option<UserItem>,
    pub selected_item: Option<UserItem>,
    pub picked_up_gold: bool,
    
    // Timing
    pub move_time: i64,
    pub attack_time: i64,
    pub spell_time: i64,
    pub pickup_time: i64,
    pub use_item_time: i64,
    
    // Flags
    pub can_move: bool,
    pub can_run: bool,
    pub observing: bool,
    pub allow_observe: bool,
    
    // Output messages (chat log)
    pub output_messages: VecDeque<OutputMessage>,
    pub max_output_messages: usize,
    
    // Map info
    pub map_info: HashMap<i32, String>, // map_id -> map_name
    pub current_map_index: i32,
    pub map_control: Option<map_control::MapControl>,
    
    // Rendering
    pub tile_texture_manager: std::cell::RefCell<crate::scenes::game_scene::tile_texture_manager::TileTextureManager>,
    pub player_x: i32,
    pub player_y: i32,
    
    // Dialogs (TODO: implement when dialog system ready)
    // pub main_dialog: MainDialog,
    // pub chat_dialog: ChatDialog,
    // pub inventory_dialog: InventoryDialog,
    // ... 40+ dialogs
    
    // Pathfinding
    pub pathfinder: Option<PathFinder>,
}

impl GameScene {
    /// Load map file by name (searches in Map and Data/Map directories)
    fn load_map_file(map_name: &str) -> std::io::Result<map_control::MapControl> {
        use std::path::PathBuf;
        
        // Try different paths - prioritize ClientRust/Map first
        let paths = [
            PathBuf::from(format!("Map/{}.map", map_name)),         // ClientRust/Map
            PathBuf::from(format!("./Map/{}.map", map_name)),
            PathBuf::from(format!("Data/Map/{}.map", map_name)),
            PathBuf::from(format!("./Data/Map/{}.map", map_name)),
            PathBuf::from(format!("../Data/Map/{}.map", map_name)),
            PathBuf::from(format!("../../Data/Map/{}.map", map_name)),
        ];
        
        for path in &paths {
            if path.exists() {
                tracing::debug!("📂 Found map file at: {}", path.display());
                match MapReader::new(path.to_str().unwrap()) {
                    Ok(reader) => {
                        tracing::info!("✅ MapReader loaded: {}x{}", reader.width, reader.height);
                        return Ok(map_control::MapControl::from_map_reader(reader));
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to parse map {}: {}", path.display(), e);
                        return Err(e);
                    }
                }
            }
        }
        
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Map file not found: {}", map_name),
        ))
    }
    
    /// Create new game scene
    pub fn new() -> Self {
        // 🧪 临时: 尝试加载测试地图 "0"
        let test_map = Self::load_map_file("0").ok();
        let (initial_x, initial_y) = if let Some(ref map) = test_map {
            let center_x = map.width / 2;
            let center_y = map.height / 2;
            // 确保是偶数坐标
            let x = if center_x % 2 == 0 { center_x } else { center_x + 1 };
            let y = if center_y % 2 == 0 { center_y } else { center_y + 1 };
            tracing::info!("🧪 Loaded test map '0': {}x{}, player at ({}, {})", map.width, map.height, x, y);
            (x, y)
        } else {
            tracing::warn!("⚠️  Failed to load test map '0', using default empty position");
            (100, 100) // 默认偶数坐标
        };
        
        Self {
            user: None,
            hero: None,
            objects: HashMap::new(),
            monsters: HashMap::new(),
            npcs: HashMap::new(),
            items: HashMap::new(),
            players: HashMap::new(),
            spells: Vec::new(),
            effects: Vec::new(),
            damages: Vec::new(),
            gold: 0,
            credit: 0,
            attack_mode: AttackMode::Peace,
            pet_mode: PetMode::Both,
            lights: LightSetting::Normal,
            storage: vec![None; 80],
            guild_storage: vec![None; 112],
            refine_storage: vec![None; 16],
            hero_storage: [None, None, None, None, None, None, None, None],
            hover_item: None,
            selected_item: None,
            picked_up_gold: false,
            move_time: 0,
            attack_time: 0,
            spell_time: 0,
            pickup_time: 0,
            use_item_time: 0,
            can_move: true,
            can_run: true,
            observing: false,
            allow_observe: false,
            output_messages: VecDeque::new(),
            max_output_messages: 100,
            map_info: HashMap::new(),
            current_map_index: 0,
            map_control: test_map, // 🧪 使用测试地图
            tile_texture_manager: std::cell::RefCell::new(
                crate::scenes::game_scene::tile_texture_manager::TileTextureManager::new()
            ),
            player_x: initial_x,
            player_y: initial_y,
            pathfinder: None,
        }
    }
    
    /// Add output message to chat log
    pub fn add_output_message(&mut self, text: String, color: (u8, u8, u8)) {
        let message = OutputMessage {
            text,
            color,
            timestamp: get_current_time(),
        };
        
        self.output_messages.push_back(message);
        
        // Limit message count
        while self.output_messages.len() > self.max_output_messages {
            self.output_messages.pop_front();
        }
    }
    
    /// Add game object
    pub fn add_monster(&mut self, monster: MonsterObject) {
        let id = monster.map_object.object_id();
        self.monsters.insert(id, monster);
    }
    
    pub fn add_npc(&mut self, npc: NPCObject) {
        let id = npc.map_object.object_id();
        self.npcs.insert(id, npc);
    }
    
    pub fn add_item(&mut self, item: ItemObject) {
        let id = item.map_object.object_id();
        self.items.insert(id, item);
    }
    
    pub fn add_player(&mut self, player: UserObject) {
        let id = player.player.map_object.object_id();
        self.players.insert(id, player);
    }
    
    /// Remove object by ID
    pub fn remove_object(&mut self, object_id: u32) {
        self.monsters.remove(&object_id);
        self.npcs.remove(&object_id);
        self.items.remove(&object_id);
        self.players.remove(&object_id);
        self.spells.retain(|s| s.map_object.object_id() != object_id);
    }
    
    /// Get current time in milliseconds
    fn get_time(&self) -> i64 {
        get_current_time()
    }
    
    /// Update all game objects
    pub fn update_objects(&mut self, delta_time: f32) {
        let current_time = self.get_time();
        
        // Update user
        if let Some(ref mut user) = self.user {
            // TODO: user.update(delta_time);
            let _ = user;
        }
        
        // Update hero
        if let Some(ref mut hero) = self.hero {
            hero.update_loyalty(delta_time);
        }
        
        // Update monsters
        for monster in self.monsters.values_mut() {
            // TODO: monster.update(delta_time);
            let _ = monster;
        }
        
        // Update spells
        for spell in self.spells.iter_mut() {
            spell.update_position(current_time);
        }
        
        // Remove expired spells
        self.spells.retain(|s| !s.should_remove(current_time));
        
        // Update effects
        for effect in self.effects.iter_mut() {
            effect.update(current_time);
        }
        
        // Remove finished effects
        self.effects.retain(|e| !e.is_finished());
        
        // Update damage numbers
        for damage in self.damages.iter_mut() {
            damage.update(current_time, delta_time);
        }
        
        // Remove finished damage numbers
        self.damages.retain(|d| !d.is_finished());
    }
    
    /// Change attack mode
    pub fn set_attack_mode(&mut self, mode: AttackMode) {
        self.attack_mode = mode;
        self.add_output_message(
            format!("Attack mode: {:?}", mode),
            (255, 255, 0),
        );
    }
    
    /// Change pet mode
    pub fn set_pet_mode(&mut self, mode: PetMode) {
        self.pet_mode = mode;
        self.add_output_message(
            format!("Pet mode: {:?}", mode),
            (255, 255, 0),
        );
    }
    
    /// Pick up item
    pub fn pickup_item(&mut self, item_id: u32) {
        let current_time = self.get_time();
        
        // Check pickup cooldown
        if current_time < self.pickup_time {
            return;
        }
        
        // TODO: Send pickup packet to server
        println!("Picking up item {}", item_id);
        
        self.pickup_time = current_time + 200; // 200ms cooldown
    }
    
    /// Use item from inventory
    pub fn use_item(&mut self, slot: usize) {
        let current_time = self.get_time();
        
        // Check use item cooldown
        if current_time < self.use_item_time {
            return;
        }
        
        // TODO: Check if item exists in slot
        // TODO: Send use item packet to server
        println!("Using item in slot {}", slot);
        
        self.use_item_time = current_time + 300; // 300ms cooldown
    }
    
    /// Draw map with 3 layers (Back, Middle, Front)
    fn draw_map(&self, ctx: &mut ggez::Context, canvas: &mut crate::graphics::Canvas, ggez_manager: &mut crate::graphics::GgezManager, map: &map_control::MapControl) {
        // Get tile manager
        let mut tile_manager = self.tile_texture_manager.borrow_mut();
        
        // Map tile dimensions (matches C# CellWidth/CellHeight)
        const TILE_WIDTH: f32 = 48.0;
        const TILE_HEIGHT: f32 = 32.0;
        
        // 使用实际窗口大小
        let screen_width = ctx.gfx.drawable_size().0;
        let screen_height = ctx.gfx.drawable_size().1;
        
        // Screen offset in tiles (C# formula: OffSetX = ScreenWidth / 2 / CellWidth)
        let offset_x_tiles = (screen_width / 2.0 / TILE_WIDTH) as i32;
        let offset_y_tiles = (screen_height / 2.0 / TILE_HEIGHT) as i32 - 1;
        
        // Calculate visible tile range (C#: ViewRangeX = OffSetX + 6)
        let view_range_x = offset_x_tiles + 6;
        let view_range_y = offset_y_tiles + 6;
        
        // C#允许循环从负数开始,在循环内部检查边界!
        // 不要用.max(0)裁剪,这会改变遍历顺序导致坐标错位
        let start_x = self.player_x - view_range_x;
        let start_y = self.player_y - view_range_y;
        let end_x = self.player_x + view_range_x;
        let end_y = self.player_y + view_range_y;
        
        tracing::debug!("🎨 Drawing map: player=({}, {}), range=({},{} to {},{})", 
            self.player_x, self.player_y, start_x, start_y, end_x, end_y);
        
        use ggez::graphics::DrawParam;
        
        let mut drawn_tiles = 0;
        
        // ========== LAYER 1: BackImage (Ground) ==========
        let mut checked_cells = 0;
        let mut back_image_cells = 0;
        let mut texture_found = 0;
        for y in start_y..=end_y {  // C# uses <=, so use ..= in Rust
            if y <= 0 || y % 2 == 1 { continue; } // Skip y<=0 or odd rows (matches C#)
            if y >= map.height { break; }  // C#: if (y >= Height) break;
            for x in start_x..=end_x {  // C# uses <=, so use ..= in Rust
                if x <= 0 || x % 2 == 1 { continue; } // Skip x<=0 or odd columns (matches C#)
                if x >= map.width { break; }  // C#: if (x >= Width) break;
                
                if let Some(cell) = map.get_cell(x, y) {
                    checked_cells += 1;
                    if cell.back_image > 0 && cell.back_index != -1 {
                        back_image_cells += 1;
                        // C#: index = (M2CellInfo[x, y].BackImage & 0x1FFFFFFF) - 1;
                        let image_index = ((cell.back_image & 0x1FFFFFFF) - 1) as u16;
                        if back_image_cells == 1 {
                            tracing::debug!("🎨 First BackImage cell: ({}, {}), back_image=0x{:X} (raw), masked_index={}, back_index={}", 
                                x, y, cell.back_image, image_index, cell.back_index);
                        }
                        
                        if let Some(texture) = tile_manager.get_tile_texture(ctx, cell.back_index as i32, image_index, ggez_manager) {
                            texture_found += 1;
                            if texture_found == 1 {
                                tracing::debug!("🎨 First texture: {} (width={}, height={}, offset=({}, {}))", 
                                    texture.texture_name, texture.width, texture.height, 
                                    texture.offset_x, texture.offset_y);
                            }
                            if let Some(image) = ggez_manager.get_texture(&texture.texture_name) {
                                // C# formula:
                                // drawX = (x - User.Movement.X + OffSetX) * CellWidth - OffSetX + User.OffSetMove.X
                                // drawY = (y - User.Movement.Y + OffSetY) * CellHeight + User.OffSetMove.Y
                                // Libraries.MapLibs[].Draw(index, drawX, drawY); 
                                // ⚠️ C# Draw(int, int, int)版本不应用mi.X/mi.Y偏移!
                                let draw_x = ((x - self.player_x + offset_x_tiles) as f32 * TILE_WIDTH) 
                                    - offset_x_tiles as f32 + 0.0;
                                let draw_y = (y - self.player_y + offset_y_tiles) as f32 * TILE_HEIGHT 
                                    + 0.0;
                                
                                if drawn_tiles == 0 {
                                    tracing::debug!("🎨 First tile: cell({},{}), offset_x={}, offset_y={}, draw_x={:.1}, draw_y={:.1}", 
                                        x, y, offset_x_tiles, offset_y_tiles, draw_x, draw_y);
                                }
                                
                                canvas.draw(
                                    image,
                                    DrawParam::default()
                                        .dest([draw_x, draw_y])
                                );
                                
                                drawn_tiles += 1;
                            }
                        }
                    }
                }
            }
        }
        
        // ========== LAYER 2: MiddleImage (Decorations) ==========
        // Note: Middle layer does NOT skip odd coordinates (unlike BackImage)
        for y in start_y..=(end_y + 5) {  // C#: y <= User.Movement.Y + ViewRangeY + 5
            if y <= 0 { continue; } // Skip y<=0 (matches C#)
            if y >= map.height { break; }  // C#: boundary check
            for x in start_x..=end_x {  // C# uses <=
                if x < 0 { continue; } // Skip x<0 (matches C#)
                if x >= map.width { break; }  // C#: boundary check
                if let Some(cell) = map.get_cell(x, y) {
                    if cell.middle_image > 0 && cell.middle_index != -1 {
                        // C#: index = M2CellInfo[x, y].MiddleImage - 1; (no mask)
                        let image_index = (cell.middle_image - 1) as u16;
                        
                        if let Some(texture) = tile_manager.get_tile_texture(ctx, cell.middle_index as i32, image_index, ggez_manager) {
                            if let Some(image) = ggez_manager.get_texture(&texture.texture_name) {
                                // C# formula:
                                // drawX = (x - User.Movement.X + OffSetX) * CellWidth - OffSetX + User.OffSetMove.X
                                // drawY = (y - User.Movement.Y + OffSetY) * CellHeight + User.OffSetMove.Y
                                // Libraries.MapLibs[].Draw(index, drawX, drawY);
                                // ⚠️ C# Draw(int, int, int)版本不应用mi.X/mi.Y偏移!
                                let draw_x = ((x - self.player_x + offset_x_tiles) as f32 * TILE_WIDTH) 
                                    - offset_x_tiles as f32 + 0.0;
                                let draw_y = (y - self.player_y + offset_y_tiles) as f32 * TILE_HEIGHT 
                                    + 0.0;
                                
                                canvas.draw(
                                    image,
                                    DrawParam::default()
                                        .dest([draw_x, draw_y])
                                );
                                
                                drawn_tiles += 1;
                            }
                        }
                    }
                }
            }
        }
        
        // ========== LAYER 3: FrontImage (Buildings, Trees) ==========
        // Note: Front layer does NOT skip odd coordinates (unlike BackImage)
        for y in start_y..=(end_y + 5) {  // C#: y <= User.Movement.Y + ViewRangeY + 5
            if y <= 0 { continue; } // Skip y<=0 (matches C#)
            if y >= map.height { break; }  // C#: boundary check
            for x in start_x..=end_x {  // C# uses <=
                if x < 0 { continue; } // Skip x<0 (matches C#)
                if x >= map.width { break; }  // C#: boundary check
                if let Some(cell) = map.get_cell(x, y) {
                    if cell.front_image > 0 && cell.front_index != -1 && cell.front_index != 200 {
                        let image_index = (cell.front_image & 0x7FFF) - 1;
                        
                        if image_index >= 0 {
                            if let Some(texture) = tile_manager.get_tile_texture(ctx, cell.front_index as i32, image_index as u16, ggez_manager) {
                                if let Some(image) = ggez_manager.get_texture(&texture.texture_name) {
                                    // C# formula:
                                    // drawX = (x - User.Movement.X + OffSetX) * CellWidth - OffSetX + User.OffSetMove.X
                                    // drawY = (y - User.Movement.Y + OffSetY) * CellHeight + User.OffSetMove.Y
                                    // Libraries.MapLibs[].Draw(index, drawX, drawY);
                                    // ⚠️ C# Draw(int, int, int)版本不应用mi.X/mi.Y偏移!
                                    let draw_x = ((x - self.player_x + offset_x_tiles) as f32 * TILE_WIDTH) 
                                        - offset_x_tiles as f32 + 0.0;
                                    let draw_y = (y - self.player_y + offset_y_tiles) as f32 * TILE_HEIGHT 
                                        + 0.0;
                                    
                                    canvas.draw(
                                        image,
                                        DrawParam::default()
                                            .dest([draw_x, draw_y])
                                    );
                                    
                                    drawn_tiles += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        tracing::debug!("🎨 Map draw summary: checked={}, back_image={}, texture_found={}, drawn={}", 
            checked_cells, back_image_cells, texture_found, drawn_tiles);
        
        if drawn_tiles > 0 {
            tracing::trace!("Drew {} tiles (3 layers)", drawn_tiles);
        }
    }
}

impl Default for GameScene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene for GameScene {
    fn scene_type(&self) -> SceneType {
        SceneType::Game
    }
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn initialize(&mut self) {
        println!("🎮🎮🎮 GameScene::initialize CALLED! 🎮🎮🎮");
        tracing::error!("🎮🎮🎮 GameScene::initialize CALLED! 🎮🎮🎮");
        
        // Load map tile libraries
        let mut tile_mgr = self.tile_texture_manager.borrow_mut();
        tracing::error!("🔄 About to call load_tiles_libraries()...");
        match tile_mgr.load_tiles_libraries() {
            Ok(count) => {
                tracing::error!("✅✅✅ Loaded {} map tile libraries ✅✅✅", count);
            }
            Err(e) => {
                tracing::error!("❌❌❌ Failed to load map tile libraries: {} ❌❌❌", e);
            }
        }
        drop(tile_mgr);
        
        tracing::error!("🎮 GameScene::initialize COMPLETED!");
        // TODO: Initialize pathfinder
        // TODO: Load UI dialogs
        // TODO: Request initial game state from server
    }
    
    fn update(&mut self, delta_time: f32) {
        // Update all game objects
        self.update_objects(delta_time);
        
        // TODO: Update camera
        // TODO: Update map
    }
    
    fn draw(&self, ctx: &mut ggez::Context, canvas: &mut crate::graphics::Canvas, ggez_manager: &mut crate::graphics::GgezManager) {
        use ggez::graphics::{Color, DrawParam, Text, PxScale};
        use ggez::mint::Point2;
        
        // 1. Draw map (if loaded)
        if let Some(map) = &self.map_control {
            self.draw_map(ctx, canvas, ggez_manager, map);
            
            // Display map info (debug - bottom left)
            let map_info_text = format!(
                "Map: {} ({}x{})\nPos: ({}, {})", 
                map.title, map.width, map.height,
                self.player_x, self.player_y
            );
            let mut map_info = Text::new(map_info_text);
            map_info.set_scale(PxScale::from(14.0));
            canvas.draw(
                &map_info,
                DrawParam::default()
                    .dest(Point2 { x: 10.0, y: 700.0 })
                    .color(Color::from_rgba(100, 255, 100, 200)),
            );
        } else {
            // No map loaded - show waiting message
            let mut waiting = Text::new("🗺️  Waiting for map data...");
            waiting.set_scale(PxScale::from(32.0));
            canvas.draw(
                &waiting,
                DrawParam::default()
                    .dest(Point2 { x: 300.0, y: 350.0 })
                    .color(Color::from_rgb(255, 200, 100)),
            );
        }
        
        // 2. Display player info (top right)
        if let Some(user) = &self.user {
            let player_text = format!(
                "Player: {} | Gold: {}", 
                user.player.map_object.name,
                self.gold
            );
            let mut player_info = Text::new(player_text);
            player_info.set_scale(PxScale::from(18.0));
            canvas.draw(
                &player_info,
                DrawParam::default()
                    .dest(Point2 { x: 700.0, y: 10.0 })
                    .color(Color::from_rgb(255, 255, 100)),
            );
        }
        
        // TODO: Draw game objects (sorted by Y position)
        // TODO: Draw effects
        // TODO: Draw damage numbers
        // TODO: Draw UI dialogs
    }
    
    /// Process game events from GameClient
    fn process_event(&mut self, event: &GameEvent) {
        match event {
            GameEvent::PlayerSpawned { player } => {
                println!("Player spawned: {}", player.name);
                // TODO: Create UserObject from PlayerState
                // For now, just log
            }
            
            GameEvent::PlayerMoved { location } => {
                if self.user.is_some() {
                    // TODO: Update user position using public API
                    println!("Player moved to: ({}, {})", location.x, location.y);
                }
            }
            
            GameEvent::ObjectSpawned { object } => {
                match object {
                    crate::network::game_client::GameObject::Player { id, name, .. } => {
                        println!("Player spawned: {} ({})", name, id);
                    }
                    crate::network::game_client::GameObject::Monster { id, name, .. } => {
                        println!("Monster spawned: {} ({})", name, id);
                    }
                    crate::network::game_client::GameObject::Npc { id, name, .. } => {
                        println!("NPC spawned: {} ({})", name, id);
                    }
                    crate::network::game_client::GameObject::Item { id, .. } => {
                        println!("Item spawned: {}", id);
                    }
                }
                // TODO: Create appropriate object type (Monster/NPC/Item/Player)
            }
            
            GameEvent::ObjectRemoved { object_id } => {
                println!("Removing object {}", object_id);
                self.remove_object(*object_id);
            }
            
            GameEvent::ChatReceived { message } => {
                self.add_output_message(
                    message.text.clone(),
                    (255, 255, 255),
                );
            }
            
            GameEvent::GoldChanged { gold } => {
                self.gold = *gold;
            }
            
            GameEvent::SystemMessage { message } => {
                self.add_output_message(
                    message.clone(),
                    (255, 255, 0),
                );
            }
            
            GameEvent::ItemGained { item, grid_type } => {
                println!("Item gained: {:?} in {}", item, grid_type);
                // TODO: Add to appropriate inventory
            }
            
            GameEvent::MagicCast { spell, target_id } => {
                println!("Magic cast: {:?} on target {}", spell, target_id);
                // TODO: Create spell effect
            }
            
            GameEvent::MapInformation { map_index, file_name, title } => {
                tracing::info!("🗺️  Loading map: {} ({})", title, file_name);
                self.current_map_index = *map_index;
                self.map_info.insert(*map_index, title.clone());
                
                // Load map file using objects::MapReader
                match Self::load_map_file(file_name) {
                    Ok(mut map) => {
                        map.title = title.clone();
                        map.filename = file_name.clone();
                        tracing::info!("✅ Map loaded: {} ({}x{})", map.title, map.width, map.height);
                        self.map_control = Some(map);
                        
                        // Update player position (center of map for now)
                        // ⚠️ 重要: 坐标必须是偶数 (C# MapControl 只绘制偶数坐标的格子)
                        if self.player_x == 0 && self.player_y == 0 {
                            let center_x = self.map_control.as_ref().unwrap().width / 2;
                            let center_y = self.map_control.as_ref().unwrap().height / 2;
                            // 确保是偶数坐标
                            self.player_x = if center_x % 2 == 0 { center_x } else { center_x + 1 };
                            self.player_y = if center_y % 2 == 0 { center_y } else { center_y + 1 };
                            tracing::info!("📍 Player positioned at even coords: ({}, {})", self.player_x, self.player_y);
                        }
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to load map {}: {}", file_name, e);
                        // Create empty fallback map
                        let mut fallback = map_control::MapControl::new(100, 100);
                        fallback.title = title.clone();
                        fallback.filename = file_name.clone();
                        self.map_control = Some(fallback);
                    }
                }
            }
            
            _ => {
                // TODO: Handle other game events
            }
        }
    }
    
    fn handle_mouse_move(&mut self, _x: i32, _y: i32) {
        // TODO: Update hover states
        // TODO: Update cursor
    }
    
    fn handle_mouse_button(&mut self, button: super::MouseButton, pressed: bool, x: i32, y: i32) {
        use super::MouseButton;
        
        if pressed {
            tracing::debug!("GameScene click at ({}, {}) with {:?}", x, y, button);
            
            match button {
                MouseButton::Left => {
                    // TODO: Handle left click (move, attack, interact)
                }
                MouseButton::Right => {
                    // TODO: Handle right click (pickup item)
                }
                MouseButton::Middle => {
                    // TODO: Handle middle click
                }
                _ => {}
            }
        }
    }
    
    fn handle_key_press(&mut self, key: super::KeyCode, modifiers: super::ModifiersState) -> bool {
        use super::KeyCode;
        
        match key {
            // Movement keys handled separately
            KeyCode::ArrowUp | KeyCode::ArrowDown | KeyCode::ArrowLeft | KeyCode::ArrowRight => {
                // TODO: Handle movement
                true
            }
            
            // Attack mode (Ctrl+H)
            KeyCode::KeyH if modifiers.control_key() => {
                // TODO: Cycle through attack modes
                true
            }
            
            // Pet mode (Ctrl+E)
            KeyCode::KeyE if modifiers.control_key() => {
                // TODO: Cycle through pet modes
                true
            }
            
            // Inventory (Tab)
            KeyCode::Tab => {
                // TODO: Toggle inventory
                true
            }
            
            // Character (C)
            KeyCode::KeyC => {
                // TODO: Toggle character dialog
                true
            }
            
            // Skills (S)
            KeyCode::KeyS => {
                // TODO: Toggle skills dialog
                true
            }
            
            // Quest (Q)
            KeyCode::KeyQ => {
                // TODO: Toggle quest dialog
                true
            }
            
            // Guild (G)
            KeyCode::KeyG => {
                // TODO: Toggle guild dialog
                true
            }
            
            _ => false
        }
    }
}

/// Get current time in milliseconds
fn get_current_time() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_scene_creation() {
        let scene = GameScene::new();
        assert_eq!(scene.scene_type(), SceneType::Game);
        assert_eq!(scene.gold, 0);
        assert_eq!(scene.storage.len(), 80);
        assert_eq!(scene.guild_storage.len(), 112);
    }

    #[test]
    fn test_add_output_message() {
        let mut scene = GameScene::new();
        
        scene.add_output_message("Test message".to_string(), (255, 255, 255));
        assert_eq!(scene.output_messages.len(), 1);
        
        // Test message limit
        scene.max_output_messages = 3;
        scene.add_output_message("Message 2".to_string(), (255, 255, 255));
        scene.add_output_message("Message 3".to_string(), (255, 255, 255));
        scene.add_output_message("Message 4".to_string(), (255, 255, 255));
        
        assert_eq!(scene.output_messages.len(), 3);
        assert_eq!(scene.output_messages[0].text, "Message 2");
    }

    #[test]
    fn test_attack_pet_modes() {
        let mut scene = GameScene::new();
        
        scene.set_attack_mode(AttackMode::All);
        assert_eq!(scene.attack_mode, AttackMode::All);
        
        scene.set_pet_mode(PetMode::AttackOnly);
        assert_eq!(scene.pet_mode, PetMode::AttackOnly);
    }
}
